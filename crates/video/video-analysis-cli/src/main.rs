#![doc = include_str!("../README.md")]

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs::File;
use std::path::{Path, PathBuf};

use clap::{ArgGroup, CommandFactory, FromArgMatches, Parser, Subcommand, ValueEnum};
use jobs_core::BackgroundJobRunner;
use model_runtime::{
    jobs::spawn_model_download_job, HuggingFaceDownloader, HuggingFaceModelSpec, ModelBundle,
    ModelBundleStore, ModelPreset, ModelTask,
};
use serde::Serialize;
use three_d_processing_core::Point3;
use three_d_processing_io::{read_mesh, write_mesh};
use video_analysis_cli::package_catalog::{package_by_name, package_catalog, PackageInfo};
use video_analysis_core::{
    DetectError, PixelFormat, Result, SceneDetector, ScenePipeline, VideoSource,
};
use video_analysis_dataset::AnalysisDataset;
use video_analysis_detectors::{
    AdaptiveDetector, AdaptiveScoreAlgorithm, ContentDetector, ContentScoreAlgorithm, HashDetector,
    HashScoreAlgorithm, HistogramDetector, HistogramScoreAlgorithm, ThresholdDetector,
    WeightedComponent, WeightedCompositeDetector,
};
use video_analysis_ffmpeg::FfmpegVideoSource;
use video_analysis_output::{write_scene_list_csv, write_stats_csv};
use video_analysis_posture::{Keypoint3d, Pose3dEstimate, Skeleton};
use video_analysis_posture_io::{
    read_coco_keypoints_json, write_coco_keypoints_json, write_stick_figure_gltf,
    write_stick_figure_ply,
};
use video_analysis_recognition::RawPose2dPrediction;
#[cfg(feature = "onnx")]
use video_analysis_recognition::VisionModelBackend;
#[cfg(feature = "onnx")]
use video_analysis_recognition::{normalize_predictions, PredictionRepairOptions};
use video_analysis_split::{split_video_ffmpeg, SplitOptions};

#[derive(Debug, Parser)]
#[command(name = "vanalyze", version, about = "Rust video scene analysis")]
struct Cli {
    #[arg(
        long,
        global = true,
        value_name = "PATH",
        help = "Read shell-style CLI arguments from a config file"
    )]
    config: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Analyze(AnalyzeReportArgs),
    Detect(AnalyzeArgs),
    List(AnalyzeArgs),
    Packages(PackagesArgs),
    Split(SplitArgs),
    Models(ModelsArgs),
    Mesh(MeshArgs),
    Posture(PostureArgs),
}

#[derive(Debug, Parser)]
struct AnalyzeArgs {
    #[arg(short, long)]
    input: PathBuf,
    #[command(flatten)]
    detection: DetectionSelection,
    #[arg(long)]
    output: Option<PathBuf>,
    #[arg(long)]
    stats: Option<PathBuf>,
    #[command(flatten)]
    detector_options: DetectorOptions,
}

#[derive(Debug, Parser)]
struct AnalyzeReportArgs {
    #[arg(short, long)]
    input: PathBuf,
    #[command(flatten)]
    detection: DetectionSelection,
    #[arg(long)]
    output: Option<PathBuf>,
    #[arg(long, default_value_t = 30)]
    sample_every_frames: u64,
    #[arg(long)]
    max_frames: Option<u64>,
    #[arg(long)]
    model_manifest: Option<PathBuf>,
    #[arg(long, value_enum, requires = "model_manifest")]
    model_backend: Option<ModelBackendKind>,
    #[command(flatten)]
    detector_options: DetectorOptions,
}

#[derive(Debug, Parser)]
struct SplitArgs {
    #[arg(short, long)]
    input: PathBuf,
    #[command(flatten)]
    detection: DetectionSelection,
    #[arg(long)]
    output_dir: PathBuf,
    #[command(flatten)]
    detector_options: DetectorOptions,
}

#[derive(Debug, Parser)]
struct PackagesArgs {
    #[command(subcommand)]
    command: PackagesCommand,
}

#[derive(Debug, Subcommand)]
enum PackagesCommand {
    List(PackageListArgs),
    Inspect(PackageInspectArgs),
}

#[derive(Debug, Parser)]
struct PackageListArgs {
    #[arg(long, default_value_t = false)]
    json: bool,
}

#[derive(Debug, Parser)]
struct PackageInspectArgs {
    package: String,
    #[arg(long, default_value_t = false)]
    json: bool,
}

#[derive(Debug, Parser)]
struct ModelsArgs {
    #[command(subcommand)]
    command: ModelsCommand,
}

#[derive(Debug, Subcommand)]
enum ModelsCommand {
    Presets,
    Download(ModelDownloadArgs),
    Inspect(ModelInspectArgs),
    Run(ModelRunArgs),
}

#[derive(Debug, Parser)]
struct MeshArgs {
    #[command(subcommand)]
    command: MeshCommand,
}

#[derive(Debug, Subcommand)]
enum MeshCommand {
    Inspect(MeshInspectArgs),
    Convert(MeshConvertArgs),
}

#[derive(Debug, Parser)]
struct MeshInspectArgs {
    #[arg(long)]
    input: PathBuf,
}

#[derive(Debug, Parser)]
struct MeshConvertArgs {
    #[arg(long)]
    input: PathBuf,
    #[arg(long)]
    output: PathBuf,
}

#[derive(Debug, Parser)]
struct PostureArgs {
    #[command(subcommand)]
    command: PostureCommand,
}

#[derive(Debug, Subcommand)]
enum PostureCommand {
    Estimate(PostureEstimateArgs),
    Export(PostureExportArgs),
}

#[derive(Debug, Parser)]
#[command(group(
    ArgGroup::new("posture_source")
        .args(["manifest", "predictions_json"])
        .required(true)
))]
struct PostureEstimateArgs {
    #[arg(long)]
    manifest: Option<PathBuf>,
    #[arg(long)]
    predictions_json: Option<PathBuf>,
    #[arg(long, requires = "manifest", default_value_t = ModelBackendKind::Onnx)]
    backend: ModelBackendKind,
    #[arg(long, requires = "manifest")]
    input: Option<PathBuf>,
    #[arg(long, requires = "manifest")]
    width: Option<u32>,
    #[arg(long, requires = "manifest")]
    height: Option<u32>,
    #[arg(long, default_value_t = RawPixelFormatKind::Rgb24, requires = "manifest")]
    pixel_format: RawPixelFormatKind,
    #[arg(long)]
    output: PathBuf,
}

#[derive(Debug, Parser)]
struct PostureExportArgs {
    #[arg(long)]
    input: PathBuf,
    #[arg(long)]
    output: PathBuf,
}

#[derive(Debug, Parser)]
#[command(group(
    ArgGroup::new("model_source")
        .args(["preset", "repo_id"])
        .required(true)
))]
struct ModelDownloadArgs {
    #[arg(long, value_enum)]
    preset: Option<ModelPresetKind>,
    #[arg(long, requires = "files")]
    repo_id: Option<String>,
    #[arg(long, requires = "repo_id")]
    name: Option<String>,
    #[arg(long, default_value = "main")]
    revision: String,
    #[arg(long, value_enum)]
    task: Option<ModelTaskKind>,
    #[arg(long = "file")]
    files: Vec<String>,
    #[arg(long, default_value = ".model-runtime")]
    bundle_dir: PathBuf,
    #[arg(long, default_value_t = false)]
    overwrite: bool,
    #[arg(long)]
    cache_dir: Option<PathBuf>,
    #[arg(long)]
    token: Option<String>,
    #[arg(long, default_value_t = false)]
    no_progress: bool,
}

#[derive(Debug, Parser)]
#[command(group(
    ArgGroup::new("bundle_source")
        .args(["manifest", "name"])
        .required(true)
))]
struct ModelInspectArgs {
    #[arg(long)]
    manifest: Option<PathBuf>,
    #[arg(long)]
    name: Option<String>,
    #[arg(long, default_value = "main", requires = "name")]
    revision: String,
    #[arg(long, default_value = ".model-runtime", requires = "name")]
    bundle_dir: PathBuf,
}

#[derive(Debug, Parser)]
struct ModelRunArgs {
    #[arg(long)]
    manifest: PathBuf,
    #[arg(long, value_enum)]
    backend: ModelBackendKind,
    #[arg(long)]
    input: PathBuf,
    #[arg(long)]
    width: u32,
    #[arg(long)]
    height: u32,
    #[arg(long, value_enum, default_value_t = RawPixelFormatKind::Rgb24)]
    pixel_format: RawPixelFormatKind,
    #[arg(long)]
    output: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum ModelBackendKind {
    Onnx,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum RawPixelFormatKind {
    Rgb24,
    Bgr24,
}

impl From<RawPixelFormatKind> for PixelFormat {
    fn from(value: RawPixelFormatKind) -> Self {
        match value {
            RawPixelFormatKind::Rgb24 => Self::Rgb24,
            RawPixelFormatKind::Bgr24 => Self::Bgr24,
        }
    }
}

impl std::fmt::Display for ModelBackendKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Onnx => f.write_str("onnx"),
        }
    }
}

impl std::fmt::Display for RawPixelFormatKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rgb24 => f.write_str("rgb24"),
            Self::Bgr24 => f.write_str("bgr24"),
        }
    }
}

#[derive(Debug, Clone, Parser)]
struct DetectionSelection {
    #[arg(long, value_enum, default_value_t = DetectorKind::Content, conflicts_with = "detectors")]
    detector: DetectorKind,
    #[arg(long, value_enum, value_delimiter = ',', conflicts_with = "detector")]
    detectors: Vec<DetectorKind>,
    #[arg(long, default_value_t = 0.5)]
    combined_threshold: f32,
    #[arg(long = "detector-weight", value_parser = parse_detector_weight)]
    detector_weights: Vec<DetectorWeight>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum DetectorKind {
    Content,
    Adaptive,
    Threshold,
    Histogram,
    Hash,
}

impl DetectorKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Content => "content",
            Self::Adaptive => "adaptive",
            Self::Threshold => "threshold",
            Self::Histogram => "histogram",
            Self::Hash => "hash",
        }
    }

    fn from_name(value: &str) -> Option<Self> {
        match value {
            "content" => Some(Self::Content),
            "adaptive" => Some(Self::Adaptive),
            "threshold" => Some(Self::Threshold),
            "histogram" | "hist" => Some(Self::Histogram),
            "hash" => Some(Self::Hash),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
struct DetectorWeight {
    detector: DetectorKind,
    weight: f32,
}

fn parse_detector_weight(value: &str) -> std::result::Result<DetectorWeight, String> {
    let (detector, weight) = value
        .split_once('=')
        .ok_or_else(|| "detector weight must use detector=value".to_string())?;
    let detector = DetectorKind::from_name(detector)
        .ok_or_else(|| format!("unknown detector `{detector}` in detector weight"))?;
    let weight = weight
        .parse::<f32>()
        .map_err(|err| format!("invalid detector weight `{weight}`: {err}"))?;
    if !weight.is_finite() || weight <= 0.0 {
        return Err("detector weight must be finite and greater than zero".to_string());
    }
    Ok(DetectorWeight { detector, weight })
}

fn parse_frame_rate_override(value: &str) -> std::result::Result<String, String> {
    if let Some((num, den)) = value.split_once('/') {
        let numerator = num
            .parse::<u64>()
            .map_err(|err| format!("invalid frame-rate numerator `{num}`: {err}"))?;
        let denominator = den
            .parse::<u64>()
            .map_err(|err| format!("invalid frame-rate denominator `{den}`: {err}"))?;
        if numerator == 0 || denominator == 0 {
            return Err("frame-rate numerator and denominator must be positive".to_string());
        }
        return Ok(value.to_string());
    }
    let fps = value
        .parse::<f64>()
        .map_err(|err| format!("invalid frame-rate `{value}`: {err}"))?;
    if !fps.is_finite() || fps <= 0.0 {
        return Err("frame-rate must be finite and greater than zero".to_string());
    }
    Ok(value.to_string())
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ModelPresetKind {
    #[value(name = "detr-resnet-50")]
    DetrResnet50,
    YolosTiny,
    DistilbertSst2,
    BertBaseNer,
    #[value(name = "minilm-l6-v2")]
    MiniLmL6V2,
    #[value(name = "xenova-distilbert-sst2-onnx")]
    XenovaDistilbertSst2Onnx,
    #[value(name = "xenova-minilm-l6-v2-onnx")]
    XenovaMiniLmL6V2Onnx,
    #[value(name = "xenova-bart-large-mnli-onnx")]
    XenovaBartLargeMnliOnnx,
    #[value(name = "xenova-detr-resnet-50-onnx")]
    XenovaDetrResnet50Onnx,
    #[value(name = "xenova-yolov8n-pose-onnx")]
    XenovaYolov8nPoseOnnx,
    #[value(name = "trocr-base-printed-onnx")]
    XenovaTrocrBasePrintedOnnx,
    #[value(name = "trocr-base-handwritten-onnx")]
    XenovaTrocrBaseHandwrittenOnnx,
    #[value(name = "wav2vec2-base-960h")]
    Wav2Vec2Base960h,
}

impl From<ModelPresetKind> for ModelPreset {
    fn from(value: ModelPresetKind) -> Self {
        match value {
            ModelPresetKind::DetrResnet50 => Self::DetrResnet50,
            ModelPresetKind::YolosTiny => Self::YolosTiny,
            ModelPresetKind::DistilbertSst2 => Self::DistilbertSst2,
            ModelPresetKind::BertBaseNer => Self::BertBaseNer,
            ModelPresetKind::MiniLmL6V2 => Self::MiniLmL6V2,
            ModelPresetKind::XenovaDistilbertSst2Onnx => Self::XenovaDistilbertSst2Onnx,
            ModelPresetKind::XenovaMiniLmL6V2Onnx => Self::XenovaMiniLmL6V2Onnx,
            ModelPresetKind::XenovaBartLargeMnliOnnx => Self::XenovaBartLargeMnliOnnx,
            ModelPresetKind::XenovaDetrResnet50Onnx => Self::XenovaDetrResnet50Onnx,
            ModelPresetKind::XenovaYolov8nPoseOnnx => Self::XenovaYolov8nPoseOnnx,
            ModelPresetKind::XenovaTrocrBasePrintedOnnx => Self::XenovaTrocrBasePrintedOnnx,
            ModelPresetKind::XenovaTrocrBaseHandwrittenOnnx => Self::XenovaTrocrBaseHandwrittenOnnx,
            ModelPresetKind::Wav2Vec2Base960h => Self::Wav2Vec2Base960h,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ModelTaskKind {
    ObjectDetection,
    PoseEstimation2d,
    PoseLifting3d,
    ImageClassification,
    TextClassification,
    TokenClassification,
    ZeroShotClassification,
    TextEmbedding,
    AudioEmbedding,
    SpeakerDiarization,
}

impl From<ModelTaskKind> for ModelTask {
    fn from(value: ModelTaskKind) -> Self {
        match value {
            ModelTaskKind::ObjectDetection => Self::ObjectDetection,
            ModelTaskKind::PoseEstimation2d => Self::PoseEstimation2d,
            ModelTaskKind::PoseLifting3d => Self::PoseLifting3d,
            ModelTaskKind::ImageClassification => Self::ImageClassification,
            ModelTaskKind::TextClassification => Self::TextClassification,
            ModelTaskKind::TokenClassification => Self::TokenClassification,
            ModelTaskKind::ZeroShotClassification => Self::ZeroShotClassification,
            ModelTaskKind::TextEmbedding => Self::TextEmbedding,
            ModelTaskKind::AudioEmbedding => Self::AudioEmbedding,
            ModelTaskKind::SpeakerDiarization => Self::SpeakerDiarization,
        }
    }
}

#[derive(Debug, Parser, Clone)]
struct DetectorOptions {
    #[arg(
        long = "frame-rate",
        alias = "framerate",
        value_parser = parse_frame_rate_override,
        help = "Accept the PySceneDetect 0.7 frame-rate spelling; source metadata still drives decoding"
    )]
    frame_rate: Option<String>,
    #[arg(long)]
    threshold: Option<f32>,
    #[arg(long, default_value_t = 15)]
    min_scene_len: u64,
    #[arg(long, default_value_t = false)]
    luma_only: bool,
    #[arg(long, default_value_t = 2)]
    window_width: usize,
    #[arg(long, default_value_t = 15.0)]
    min_content_val: f32,
    #[arg(long, default_value_t = 256)]
    bins: usize,
    #[arg(long, default_value_t = 16)]
    hash_size: usize,
    #[arg(long, default_value_t = 2)]
    lowpass: usize,
}

fn main() -> Result<()> {
    let cli = parse_cli()?;
    match cli.command {
        Command::Analyze(args) => run_analyze_report(args)?,
        Command::Detect(args) => {
            let result = run_detection(&args.input, &args.detection, &args.detector_options)?;
            if let Some(path) = args.output {
                write_scene_list_csv(File::create(path)?, &result.scenes)?;
            } else {
                write_scene_list_csv(std::io::stdout(), &result.scenes)?;
            }
            if let Some(path) = args.stats {
                write_stats_csv(File::create(path)?, &result.metrics)?;
            }
        }
        Command::List(args) => {
            let result = run_detection(&args.input, &args.detection, &args.detector_options)?;
            for (index, scene) in result.scenes.iter().enumerate() {
                println!(
                    "Scene {:>3}: start frame {} ({:.3}s), end frame {} ({:.3}s)",
                    index + 1,
                    scene.start.frame_index,
                    scene.start.timestamp.seconds(),
                    scene.end.frame_index,
                    scene.end.timestamp.seconds()
                );
            }
        }
        Command::Packages(args) => match args.command {
            PackagesCommand::List(args) => list_packages(args)?,
            PackagesCommand::Inspect(args) => inspect_package(args)?,
        },
        Command::Split(args) => {
            let result = run_detection(&args.input, &args.detection, &args.detector_options)?;
            let options = SplitOptions {
                output_dir: args.output_dir,
                ..SplitOptions::default()
            };
            let outputs = split_video_ffmpeg(&args.input, &result.scenes, &options)?;
            for output in outputs {
                println!("{}", output.display());
            }
        }
        Command::Models(args) => match args.command {
            ModelsCommand::Presets => list_model_presets(),
            ModelsCommand::Download(args) => download_model(args)?,
            ModelsCommand::Inspect(args) => inspect_model_bundle(args)?,
            ModelsCommand::Run(args) => run_model(args)?,
        },
        Command::Mesh(args) => match args.command {
            MeshCommand::Inspect(args) => inspect_mesh(args)?,
            MeshCommand::Convert(args) => convert_mesh(args)?,
        },
        Command::Posture(args) => match args.command {
            PostureCommand::Estimate(args) => estimate_posture(args)?,
            PostureCommand::Export(args) => export_posture(args)?,
        },
    }
    Ok(())
}

fn parse_cli() -> Result<Cli> {
    parse_cli_from(std::env::current_dir()?, std::env::args_os())
}

fn parse_cli_from<I, T>(current_dir: PathBuf, raw_args: I) -> Result<Cli>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
{
    let args = args_with_optional_conf(env!("CARGO_PKG_NAME"), &current_dir, raw_args)?;
    let matches = Cli::command()
        .args_override_self(true)
        .try_get_matches_from(args)
        .unwrap_or_else(|err| err.exit());
    let cli = Cli::from_arg_matches(&matches)
        .map_err(|err| DetectError::Source(format!("failed to parse CLI arguments: {err}")))?;
    let _ = cli.config.as_ref();
    Ok(cli)
}

fn args_with_optional_conf<I, T>(
    package_name: &str,
    current_dir: &Path,
    raw_args: I,
) -> Result<Vec<OsString>>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
{
    let mut raw_args = raw_args
        .into_iter()
        .map(Into::into)
        .collect::<Vec<OsString>>();
    let program = if raw_args.is_empty() {
        OsString::from(package_name)
    } else {
        raw_args.remove(0)
    };
    let auto_conf_path = automatic_config_path(package_name, current_dir, &program);
    let mut args = vec![program];
    let explicit_conf_path = explicit_config_path(&raw_args)?.map(|path| {
        if path.is_absolute() {
            path
        } else {
            current_dir.join(path)
        }
    });
    let conf_path = explicit_conf_path.or(auto_conf_path);
    if let Some(conf_path) = conf_path {
        args.extend(read_conf_args(&conf_path)?);
    }
    args.extend(raw_args);
    Ok(args)
}

fn automatic_config_path(
    package_name: &str,
    current_dir: &Path,
    program: &OsString,
) -> Option<PathBuf> {
    if let Some(stem) = Path::new(program)
        .file_stem()
        .filter(|stem| !stem.is_empty())
    {
        let mut path = current_dir.join(stem);
        path.set_extension("conf");
        if path.is_file() {
            return Some(path);
        }
    }

    for candidate in [
        package_name,
        package_name
            .strip_prefix("moenarch-")
            .or_else(|| package_name.strip_prefix("moritzbrantner-"))
            .unwrap_or(package_name),
    ] {
        let package_conf_path = current_dir.join(format!("{candidate}.conf"));
        if package_conf_path.is_file() {
            return Some(package_conf_path);
        }
    }

    None
}

fn explicit_config_path(raw_args: &[OsString]) -> Result<Option<PathBuf>> {
    let mut config_path = None;
    let mut index = 0;
    while index < raw_args.len() {
        let arg = raw_args[index].as_os_str();
        if arg == "--config" {
            index += 1;
            let Some(value) = raw_args.get(index) else {
                return Err(DetectError::InvalidArgument(
                    "`--config` requires a path".to_string(),
                ));
            };
            config_path = Some(PathBuf::from(value));
        } else if let Some(value) = arg.to_str().and_then(|arg| arg.strip_prefix("--config=")) {
            config_path = Some(PathBuf::from(value));
        }
        index += 1;
    }
    Ok(config_path)
}

fn read_conf_args(path: &Path) -> Result<Vec<OsString>> {
    let contents = std::fs::read_to_string(path)?;
    let Some(args) = shlex::split(&contents) else {
        return Err(DetectError::InvalidArgument(format!(
            "failed to parse config file `{}` as shell-style CLI arguments",
            path.display()
        )));
    };
    Ok(args.into_iter().map(OsString::from).collect())
}

fn list_model_presets() {
    for preset in ModelPreset::ALL {
        let spec = preset.spec();
        println!(
            "{:<18} {:<48} {:?}",
            preset.as_str(),
            spec.repo_id_value().unwrap_or(""),
            spec.task
        );
    }
}

fn list_packages(args: PackageListArgs) -> Result<()> {
    let packages = package_catalog();
    if args.json {
        serde_json::to_writer_pretty(
            std::io::stdout(),
            &serde_json::json!(packages
                .iter()
                .map(package_to_json)
                .collect::<Vec<serde_json::Value>>()),
        )
        .map_err(|err| {
            DetectError::Source(format!("failed to write package catalog JSON: {err}"))
        })?;
        println!();
        return Ok(());
    }

    for package in packages {
        println!("{:<32} {}", package.name, package.role);
    }
    Ok(())
}

fn inspect_package(args: PackageInspectArgs) -> Result<()> {
    let Some(package) = package_by_name(&args.package) else {
        return Err(DetectError::InvalidArgument(format!(
            "unknown package `{}`",
            args.package
        )));
    };

    if args.json {
        serde_json::to_writer_pretty(std::io::stdout(), &package_to_json(&package)).map_err(
            |err| DetectError::Source(format!("failed to write package metadata JSON: {err}")),
        )?;
        println!();
        return Ok(());
    }

    println!("package\t{}", package.name);
    println!("role\t{}", package.role);
    for capability in package.capabilities {
        println!("{}\t{}", capability.kind.as_str(), capability.entrypoint);
    }
    Ok(())
}

fn package_to_json(package: &PackageInfo) -> serde_json::Value {
    serde_json::json!({
        "name": package.name,
        "role": package.role,
        "capabilities": package.capabilities.iter().map(|capability| {
            serde_json::json!({
                "kind": capability.kind.as_str(),
                "entrypoint": capability.entrypoint,
            })
        }).collect::<Vec<serde_json::Value>>(),
    })
}

fn download_model(args: ModelDownloadArgs) -> Result<()> {
    let spec = match (args.preset, args.repo_id) {
        (Some(preset), None) => ModelPreset::from(preset).spec().revision(args.revision),
        (None, Some(repo_id)) => {
            let task = args
                .task
                .map(ModelTask::from)
                .unwrap_or_else(|| ModelTask::Custom("custom".to_string()));
            let mut spec = HuggingFaceModelSpec::new(repo_id, task).revision(args.revision);
            if let Some(name) = args.name {
                spec = spec.name(name);
            }
            for file in args.files {
                spec = spec.file(file);
            }
            spec
        }
        _ => unreachable!("clap validates model source arguments"),
    };

    let mut downloader = HuggingFaceDownloader::new().progress(!args.no_progress);
    if let Some(cache_dir) = args.cache_dir {
        downloader = downloader.cache_dir(cache_dir);
    }
    if let Some(token) = args.token {
        downloader = downloader.token(token);
    }

    let store = ModelBundleStore::new(args.bundle_dir)
        .downloader(downloader)
        .overwrite(args.overwrite);
    let runner = BackgroundJobRunner::default();
    let mut handle = spawn_model_download_job(&runner, spec, store)
        .map_err(|err| DetectError::Source(err.to_string()))?;
    let bundle = handle
        .join_result()
        .map_err(|err| DetectError::Source(err.to_string()))?;
    println!(
        "downloaded {} from {}",
        bundle.manifest.name, bundle.manifest.repo_id
    );
    println!("manifest\t{}", bundle.manifest_path().display());
    for (remote, file) in &bundle.manifest.files {
        println!("{remote}\t{}", bundle.root.join(&file.local_path).display());
    }
    Ok(())
}

fn inspect_model_bundle(args: ModelInspectArgs) -> Result<()> {
    let bundle = if let Some(manifest) = args.manifest {
        ModelBundle::load(manifest).map_err(model_runtime_error)?
    } else {
        let name = args
            .name
            .expect("clap validates model bundle source arguments");
        ModelBundleStore::new(args.bundle_dir)
            .load(name, args.revision)
            .map_err(model_runtime_error)?
    };
    print!("{}", format_model_bundle(&bundle));
    Ok(())
}

fn run_model(args: ModelRunArgs) -> Result<()> {
    match args.backend {
        ModelBackendKind::Onnx => run_onnx_model(args),
    }
}

fn inspect_mesh(args: MeshInspectArgs) -> Result<()> {
    let mesh = read_mesh(&args.input)?;
    let bounds = mesh.bounds()?;
    println!("vertices\t{}", mesh.vertices.len());
    println!("triangles\t{}", mesh.triangles.len());
    println!("surface_area\t{}", mesh.surface_area()?);
    println!("manifold\t{}", mesh.is_manifold()?);
    println!("watertight\t{}", mesh.is_watertight()?);
    println!("components\t{}", mesh.connected_components()?.len());
    if let Some(bounds) = bounds {
        println!(
            "bounds\tmin=({}, {}, {}) max=({}, {}, {})",
            bounds.min.x, bounds.min.y, bounds.min.z, bounds.max.x, bounds.max.y, bounds.max.z
        );
    }
    Ok(())
}

fn convert_mesh(args: MeshConvertArgs) -> Result<()> {
    let mesh = read_mesh(&args.input)?;
    if let Some(parent) = args
        .output
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    write_mesh(&args.output, &mesh)
}

fn estimate_posture(args: PostureEstimateArgs) -> Result<()> {
    let poses = if let Some(path) = args.predictions_json {
        let data = std::fs::read(path)?;
        let predictions: Vec<RawPose2dPrediction> =
            serde_json::from_slice(&data).map_err(|err| {
                DetectError::Source(format!("failed to parse raw pose predictions JSON: {err}"))
            })?;
        predictions
            .iter()
            .map(|prediction| prediction.to_pose_estimate(None))
            .collect::<Result<Vec<_>>>()?
    } else {
        match args.backend {
            ModelBackendKind::Onnx => run_onnx_posture_estimate(&args)?,
        }
    };
    if let Some(parent) = args
        .output
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    write_coco_keypoints_json(args.output, &poses)
}

fn export_posture(args: PostureExportArgs) -> Result<()> {
    let poses = read_coco_keypoints_json(&args.input)?;
    let pose = poses.first().ok_or_else(|| {
        DetectError::InvalidArgument("posture export input contained no poses".to_string())
    })?;
    let keypoints = pose
        .keypoints
        .iter()
        .map(|keypoint| {
            let mut point = Keypoint3d::new(
                keypoint.name.clone(),
                Point3::new(keypoint.x, keypoint.y, 0.0),
            )?;
            if let Some(score) = keypoint.score {
                point = point.score(score)?;
            }
            if let Some(visible) = keypoint.visible {
                point = point.visible(visible);
            }
            Ok(point)
        })
        .collect::<Result<Vec<_>>>()?;
    let figure = Pose3dEstimate::new(keypoints)?.to_stick_figure(Skeleton::coco_17())?;
    if let Some(parent) = args
        .output
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    match args.output.extension().and_then(|value| value.to_str()) {
        Some("ply") => write_stick_figure_ply(args.output, &figure),
        Some("gltf") => write_stick_figure_gltf(args.output, &figure),
        _ => Err(DetectError::InvalidArgument(
            "posture export output must end in `.ply` or `.gltf`".to_string(),
        )),
    }
}

#[cfg(feature = "onnx")]
fn run_onnx_model(args: ModelRunArgs) -> Result<()> {
    let bundle = ModelBundle::load(args.manifest).map_err(model_runtime_error)?;
    let data = std::fs::read(&args.input)?;
    let frame = video_analysis_core::VideoFrame::packed(
        video_analysis_core::FramePosition {
            frame_index: 0,
            timestamp: video_analysis_core::Timestamp::new(
                0,
                video_analysis_core::Timebase::new(1, 1),
            ),
        },
        args.width,
        args.height,
        args.pixel_format.into(),
        &data,
        args.width as usize * 3,
    )?;
    let mut backend = video_analysis_recognition::OnnxObjectDetector::from_bundle(bundle)?;
    let predictions = VisionModelBackend::predict_frame(&mut backend, &frame)?;
    if let Some(path) = args.output {
        if let Some(parent) = path.parent().filter(|path| !path.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        serde_json::to_writer_pretty(File::create(path)?, &predictions).map_err(|err| {
            DetectError::Source(format!("failed to write model predictions JSON: {err}"))
        })?;
    } else {
        serde_json::to_writer_pretty(std::io::stdout(), &predictions).map_err(|err| {
            DetectError::Source(format!("failed to write model predictions JSON: {err}"))
        })?;
        println!();
    }
    Ok(())
}

#[cfg(feature = "onnx")]
fn run_onnx_posture_estimate(
    args: &PostureEstimateArgs,
) -> Result<Vec<video_analysis_posture::PoseEstimate>> {
    let manifest = args
        .manifest
        .as_ref()
        .expect("clap validates posture estimate sources");
    let input = args
        .input
        .as_ref()
        .expect("clap validates posture estimate manifest arguments");
    let width = args
        .width
        .expect("clap validates posture estimate manifest arguments");
    let height = args
        .height
        .expect("clap validates posture estimate manifest arguments");
    let bundle = ModelBundle::load(manifest).map_err(model_runtime_error)?;
    let data = std::fs::read(input)?;
    let frame = video_analysis_core::VideoFrame::packed(
        video_analysis_core::FramePosition {
            frame_index: 0,
            timestamp: video_analysis_core::Timestamp::new(
                0,
                video_analysis_core::Timebase::new(1, 1),
            ),
        },
        width,
        height,
        args.pixel_format.into(),
        &data,
        width as usize * 3,
    )?;
    let mut backend = video_analysis_posture::OnnxPose2dEstimator::from_bundle(bundle)?;
    backend.predict_frame(&frame)
}

#[cfg(not(feature = "onnx"))]
fn run_onnx_posture_estimate(
    _args: &PostureEstimateArgs,
) -> Result<Vec<video_analysis_posture::PoseEstimate>> {
    Err(DetectError::InvalidArgument(
        "`vanalyze posture estimate --backend onnx` requires building video-analysis-cli with the `onnx` or `onnxruntime` feature".to_string(),
    ))
}

#[cfg(not(feature = "onnx"))]
fn run_onnx_model(_args: ModelRunArgs) -> Result<()> {
    Err(DetectError::InvalidArgument(
        "`vanalyze models run --backend onnx` requires building video-analysis-cli with the `onnx` or `onnxruntime` feature".to_string(),
    ))
}

fn format_model_bundle(bundle: &ModelBundle) -> String {
    let mut output = String::new();
    output.push_str(&format!("name\t{}\n", bundle.manifest.name));
    output.push_str(&format!("repo_id\t{}\n", bundle.manifest.repo_id));
    output.push_str(&format!("revision\t{}\n", bundle.manifest.revision));
    output.push_str(&format!("task\t{:?}\n", bundle.manifest.task));
    output.push_str(&format!("manifest\t{}\n", bundle.manifest_path().display()));
    for (remote, file) in &bundle.manifest.files {
        output.push_str(&format!(
            "{remote}\t{}\t{} bytes\n",
            bundle.root.join(&file.local_path).display(),
            file.size_bytes
        ));
    }
    output
}

fn model_runtime_error(error: model_runtime::ModelRuntimeError) -> DetectError {
    match error {
        model_runtime::ModelRuntimeError::InvalidArgument(message) => {
            DetectError::InvalidArgument(message)
        }
        model_runtime::ModelRuntimeError::Source(message) => DetectError::Source(message),
        model_runtime::ModelRuntimeError::Io(error) => DetectError::Io(error),
    }
}

fn run_analyze_report(args: AnalyzeReportArgs) -> Result<()> {
    let report = build_analyze_report(&args)?;
    if let Some(path) = args.output {
        if let Some(parent) = path.parent().filter(|path| !path.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        serde_json::to_writer_pretty(File::create(path)?, &report).map_err(|err| {
            DetectError::Source(format!("failed to write analyze report JSON: {err}"))
        })?;
    } else {
        serde_json::to_writer_pretty(std::io::stdout(), &report).map_err(|err| {
            DetectError::Source(format!("failed to write analyze report JSON: {err}"))
        })?;
        println!();
    }
    Ok(())
}

fn build_analyze_report(args: &AnalyzeReportArgs) -> Result<AnalyzeReport> {
    if args.sample_every_frames == 0 {
        return Err(DetectError::InvalidArgument(
            "sample-every-frames must be greater than zero".to_string(),
        ));
    }
    if args.model_manifest.is_some() && args.model_backend != Some(ModelBackendKind::Onnx) {
        return Err(DetectError::InvalidArgument(
            "`vanalyze analyze --model-manifest` requires `--model-backend onnx`".to_string(),
        ));
    }

    #[cfg(not(feature = "onnx"))]
    if args.model_manifest.is_some() {
        return Err(DetectError::InvalidArgument(
            "`vanalyze analyze --model-backend onnx` requires building video-analysis-cli with the `onnx` or `onnxruntime` feature".to_string(),
        ));
    }

    #[cfg(feature = "onnx")]
    let mut model_backend = if let Some(manifest) = &args.model_manifest {
        let bundle = ModelBundle::load(manifest).map_err(model_runtime_error)?;
        Some(video_analysis_recognition::OnnxObjectDetector::from_bundle(
            bundle,
        )?)
    } else {
        None
    };

    let mut source = FfmpegVideoSource::open(&args.input)?;
    let mut pipeline = if args.detection.detectors.is_empty() {
        build_single_detector_pipeline(args.detection.detector, &args.detector_options)?
    } else {
        build_composite_detector_pipeline(&args.detection, &args.detector_options)?
    };
    #[cfg_attr(not(feature = "onnx"), allow(unused_mut))]
    let mut observations = Vec::new();
    let mut decoded = 0_u64;

    while args.max_frames.map(|max| decoded < max).unwrap_or(true) {
        let Some(frame) = source.next_frame()? else {
            break;
        };
        decoded += 1;
        pipeline.process_frame_ref(&frame.as_frame())?;

        #[cfg(feature = "onnx")]
        if let Some(backend) = model_backend.as_mut() {
            if frame.position.frame_index % args.sample_every_frames == 0 {
                let predictions = VisionModelBackend::predict_frame(backend, &frame.as_frame())?;
                observations.extend(
                    normalize_predictions(
                        predictions,
                        &ModelTask::ObjectDetection,
                        Some((frame.width, frame.height)),
                        PredictionRepairOptions::default(),
                    )
                    .into_iter()
                    .map(|prediction| prediction.to_observation("onnx").at_frame(frame.position)),
                );
            }
        }
    }

    let detection = pipeline.finish_detection()?;
    let mut dataset = AnalysisDataset::empty();
    dataset.metadata.source = Some(args.input.display().to_string());
    dataset.extend_detection_result(&detection);
    dataset.extend_observations(observations);
    let summary = dataset_summary(&dataset);
    let model = args
        .model_manifest
        .as_ref()
        .map(|manifest| AnalyzeModelReport {
            backend: "onnx".to_string(),
            manifest: manifest.display().to_string(),
        });

    Ok(AnalyzeReport {
        input: args.input.display().to_string(),
        frames_processed: detection.frames_processed,
        sample_every_frames: args.sample_every_frames,
        model,
        detection: AnalyzeDetectionReport {
            scene_count: detection.scenes.len(),
            cut_count: detection.cuts.len(),
        },
        dataset,
        summary,
    })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AnalyzeReport {
    input: String,
    frames_processed: u64,
    sample_every_frames: u64,
    model: Option<AnalyzeModelReport>,
    detection: AnalyzeDetectionReport,
    dataset: AnalysisDataset,
    summary: AnalyzeDatasetSummary,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AnalyzeModelReport {
    backend: String,
    manifest: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AnalyzeDetectionReport {
    scene_count: usize,
    cut_count: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AnalyzeDatasetSummary {
    record_count: usize,
    record_counts: BTreeMap<String, usize>,
}

fn dataset_summary(dataset: &AnalysisDataset) -> AnalyzeDatasetSummary {
    let mut record_counts = BTreeMap::new();
    for record in dataset.records() {
        *record_counts.entry(record.kind().to_string()).or_insert(0) += 1;
    }
    AnalyzeDatasetSummary {
        record_count: dataset.records.len(),
        record_counts,
    }
}

fn run_detection(
    input: &PathBuf,
    selection: &DetectionSelection,
    options: &DetectorOptions,
) -> Result<video_analysis_core::DetectionResult> {
    let _reserved_frame_rate_override = options.frame_rate.as_deref();
    let mut source = FfmpegVideoSource::open(input)?;
    let mut pipeline = if selection.detectors.is_empty() {
        build_single_detector_pipeline(selection.detector, options)?
    } else {
        build_composite_detector_pipeline(selection, options)?
    };
    pipeline.detect(&mut source)
}

fn build_single_detector_pipeline(
    detector: DetectorKind,
    options: &DetectorOptions,
) -> Result<ScenePipeline> {
    match detector {
        DetectorKind::Content => ScenePipeline::builder()
            .detector(
                ContentDetector::new(options.threshold.unwrap_or(27.0), options.min_scene_len)
                    .luma_only(options.luma_only),
            )
            .start_in_scene(true)
            .build(),
        DetectorKind::Adaptive => ScenePipeline::builder()
            .detector(
                AdaptiveDetector::new(
                    options.threshold.unwrap_or(3.0),
                    options.min_scene_len,
                    options.window_width,
                    options.min_content_val,
                )
                .luma_only(options.luma_only),
            )
            .start_in_scene(true)
            .build(),
        DetectorKind::Threshold => ScenePipeline::builder()
            .detector(ThresholdDetector::new(
                options.threshold.unwrap_or(12.0),
                options.min_scene_len,
            ))
            .start_in_scene(true)
            .build(),
        DetectorKind::Histogram => ScenePipeline::builder()
            .detector(HistogramDetector::new(
                options.threshold.unwrap_or(0.05),
                options.bins,
                options.min_scene_len,
            ))
            .start_in_scene(true)
            .build(),
        DetectorKind::Hash => ScenePipeline::builder()
            .detector(HashDetector::new(
                options.threshold.unwrap_or(0.395),
                options.hash_size,
                options.lowpass,
                options.min_scene_len,
            ))
            .start_in_scene(true)
            .build(),
    }
}

fn build_composite_detector_pipeline(
    selection: &DetectionSelection,
    options: &DetectorOptions,
) -> Result<ScenePipeline> {
    if !selection.combined_threshold.is_finite() || selection.combined_threshold < 0.0 {
        return Err(video_analysis_core::DetectError::InvalidArgument(
            "combined threshold must be finite and greater than or equal to zero".to_string(),
        ));
    }

    let mut seen = BTreeSet::new();
    for detector in &selection.detectors {
        if !seen.insert(*detector) {
            return Err(video_analysis_core::DetectError::InvalidArgument(format!(
                "duplicate detector `{}` in --detectors",
                detector.as_str()
            )));
        }
        if *detector == DetectorKind::Threshold {
            return Err(video_analysis_core::DetectError::InvalidArgument(
                "`threshold` cannot be used in weighted composite mode; use --detector threshold or an event-fusion mode when available".to_string(),
            ));
        }
    }

    let weights = detector_weight_map(selection)?;
    let selected: BTreeSet<_> = selection.detectors.iter().copied().collect();
    for detector in weights.keys() {
        if !selected.contains(detector) {
            return Err(video_analysis_core::DetectError::InvalidArgument(format!(
                "weight supplied for detector `{}` which is not present in --detectors",
                detector.as_str()
            )));
        }
    }

    let mut builder = WeightedCompositeDetector::builder()
        .threshold(selection.combined_threshold)
        .min_scene_len(options.min_scene_len);
    for detector in &selection.detectors {
        let weight = weights.get(detector).copied().unwrap_or(1.0);
        builder = builder.weighted_component(WeightedComponent::new(
            score_algorithm_for(*detector, options)?,
            weight,
        )?);
    }
    let detector = builder.build()?;
    ScenePipeline::builder()
        .detector(detector)
        .start_in_scene(true)
        .build()
}

fn detector_weight_map(selection: &DetectionSelection) -> Result<BTreeMap<DetectorKind, f32>> {
    let mut weights = BTreeMap::new();
    for detector_weight in &selection.detector_weights {
        if weights
            .insert(detector_weight.detector, detector_weight.weight)
            .is_some()
        {
            return Err(video_analysis_core::DetectError::InvalidArgument(format!(
                "duplicate weight for detector `{}`",
                detector_weight.detector.as_str()
            )));
        }
    }
    Ok(weights)
}

fn score_algorithm_for(
    detector: DetectorKind,
    options: &DetectorOptions,
) -> Result<impl video_analysis_detectors::ScoreAlgorithm> {
    match detector {
        DetectorKind::Content => Ok(CompositeScoreAlgorithm::Content(
            ContentScoreAlgorithm::new(options.threshold.unwrap_or(27.0))
                .luma_only(options.luma_only),
        )),
        DetectorKind::Adaptive => Ok(CompositeScoreAlgorithm::Adaptive(
            AdaptiveScoreAlgorithm::new(
                options.threshold.unwrap_or(3.0),
                options.window_width,
                options.min_content_val,
            )
            .luma_only(options.luma_only),
        )),
        DetectorKind::Histogram => Ok(CompositeScoreAlgorithm::Histogram(
            HistogramScoreAlgorithm::new(options.threshold.unwrap_or(0.05), options.bins),
        )),
        DetectorKind::Hash => Ok(CompositeScoreAlgorithm::Hash(HashScoreAlgorithm::new(
            options.threshold.unwrap_or(0.395),
            options.hash_size,
            options.lowpass,
        ))),
        DetectorKind::Threshold => Err(video_analysis_core::DetectError::InvalidArgument(
            "`threshold` cannot be used in weighted composite mode; use --detector threshold"
                .to_string(),
        )),
    }
}

enum CompositeScoreAlgorithm {
    Content(ContentScoreAlgorithm),
    Adaptive(AdaptiveScoreAlgorithm),
    Histogram(HistogramScoreAlgorithm),
    Hash(HashScoreAlgorithm),
}

impl video_analysis_detectors::ScoreAlgorithm for CompositeScoreAlgorithm {
    fn name(&self) -> &'static str {
        match self {
            Self::Content(algorithm) => algorithm.name(),
            Self::Adaptive(algorithm) => algorithm.name(),
            Self::Histogram(algorithm) => algorithm.name(),
            Self::Hash(algorithm) => algorithm.name(),
        }
    }

    fn latency(&self) -> usize {
        match self {
            Self::Content(algorithm) => algorithm.latency(),
            Self::Adaptive(algorithm) => algorithm.latency(),
            Self::Histogram(algorithm) => algorithm.latency(),
            Self::Hash(algorithm) => algorithm.latency(),
        }
    }

    fn process_frame(
        &mut self,
        frame: &video_analysis_core::VideoFrame<'_>,
        metrics: Option<&mut dyn video_analysis_core::MetricsSink>,
    ) -> Result<Vec<video_analysis_detectors::AlgorithmScore>> {
        match self {
            Self::Content(algorithm) => algorithm.process_frame(frame, metrics),
            Self::Adaptive(algorithm) => algorithm.process_frame(frame, metrics),
            Self::Histogram(algorithm) => algorithm.process_frame(frame, metrics),
            Self::Hash(algorithm) => algorithm.process_frame(frame, metrics),
        }
    }

    fn finish(
        &mut self,
        last_position: video_analysis_core::FramePosition,
        metrics: Option<&mut dyn video_analysis_core::MetricsSink>,
    ) -> Result<Vec<video_analysis_detectors::AlgorithmScore>> {
        match self {
            Self::Content(algorithm) => algorithm.finish(last_position, metrics),
            Self::Adaptive(algorithm) => algorithm.finish(last_position, metrics),
            Self::Histogram(algorithm) => algorithm.finish(last_position, metrics),
            Self::Hash(algorithm) => algorithm.finish(last_position, metrics),
        }
    }
}

#[allow(dead_code)]
fn _assert_detector_trait<T: SceneDetector>() {}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::error::ErrorKind;
    use tempfile::tempdir;

    fn detector_options() -> DetectorOptions {
        DetectorOptions {
            frame_rate: None,
            threshold: None,
            min_scene_len: 15,
            luma_only: false,
            window_width: 2,
            min_content_val: 15.0,
            bins: 256,
            hash_size: 16,
            lowpass: 2,
        }
    }

    #[test]
    fn model_download_requires_a_model_source() {
        let err = Cli::try_parse_from(["vanalyze", "models", "download"]).unwrap_err();

        assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn model_download_rejects_multiple_model_sources() {
        let err = Cli::try_parse_from([
            "vanalyze",
            "models",
            "download",
            "--preset",
            "detr-resnet-50",
            "--repo-id",
            "owner/model",
            "--file",
            "config.json",
        ])
        .unwrap_err();

        assert_eq!(err.kind(), ErrorKind::ArgumentConflict);
    }

    #[test]
    fn model_download_requires_files_for_custom_repo() {
        let err =
            Cli::try_parse_from(["vanalyze", "models", "download", "--repo-id", "owner/model"])
                .unwrap_err();

        assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn model_download_accepts_preset_without_files() {
        Cli::try_parse_from([
            "vanalyze",
            "models",
            "download",
            "--preset",
            "detr-resnet-50",
        ])
        .unwrap();
    }

    #[test]
    fn model_download_accepts_onnx_preset_without_files() {
        Cli::try_parse_from([
            "vanalyze",
            "models",
            "download",
            "--preset",
            "xenova-distilbert-sst2-onnx",
        ])
        .unwrap();
    }

    #[test]
    fn model_download_accepts_bart_mnli_onnx_preset_without_files() {
        let cli = Cli::try_parse_from([
            "vanalyze",
            "models",
            "download",
            "--preset",
            "xenova-bart-large-mnli-onnx",
        ])
        .unwrap();

        match cli.command {
            Command::Models(ModelsArgs {
                command: ModelsCommand::Download(args),
            }) => assert_eq!(
                args.preset.map(ModelPreset::from),
                Some(ModelPreset::XenovaBartLargeMnliOnnx)
            ),
            _ => panic!("expected models download command"),
        }
    }

    #[test]
    fn model_download_accepts_trocr_onnx_presets_without_files() {
        for (preset, expected) in [
            (
                "trocr-base-printed-onnx",
                ModelPreset::XenovaTrocrBasePrintedOnnx,
            ),
            (
                "trocr-base-handwritten-onnx",
                ModelPreset::XenovaTrocrBaseHandwrittenOnnx,
            ),
        ] {
            let cli = Cli::try_parse_from(["vanalyze", "models", "download", "--preset", preset])
                .unwrap();

            match cli.command {
                Command::Models(ModelsArgs {
                    command: ModelsCommand::Download(args),
                }) => assert_eq!(args.preset.map(ModelPreset::from), Some(expected)),
                _ => panic!("expected models download command"),
            }
        }
    }

    #[test]
    fn model_download_accepts_video_onnx_presets_without_files() {
        for (preset, expected) in [
            (
                "xenova-detr-resnet-50-onnx",
                ModelPreset::XenovaDetrResnet50Onnx,
            ),
            (
                "xenova-yolov8n-pose-onnx",
                ModelPreset::XenovaYolov8nPoseOnnx,
            ),
        ] {
            let cli = Cli::try_parse_from(["vanalyze", "models", "download", "--preset", preset])
                .unwrap();

            match cli.command {
                Command::Models(ModelsArgs {
                    command: ModelsCommand::Download(args),
                }) => assert_eq!(args.preset.map(ModelPreset::from), Some(expected)),
                _ => panic!("expected models download command"),
            }
        }
    }

    #[test]
    fn model_download_accepts_wav2vec2_preset_without_files() {
        let cli = Cli::try_parse_from([
            "vanalyze",
            "models",
            "download",
            "--preset",
            "wav2vec2-base-960h",
        ])
        .unwrap();

        match cli.command {
            Command::Models(ModelsArgs {
                command: ModelsCommand::Download(args),
            }) => assert_eq!(
                args.preset.map(ModelPreset::from),
                Some(ModelPreset::Wav2Vec2Base960h)
            ),
            _ => panic!("expected models download command"),
        }
    }

    #[test]
    fn model_download_accepts_custom_repo_with_files() {
        Cli::try_parse_from([
            "vanalyze",
            "models",
            "download",
            "--repo-id",
            "owner/model",
            "--file",
            "config.json",
        ])
        .unwrap();
    }

    #[test]
    fn model_download_accepts_custom_bundle_name() {
        let cli = Cli::try_parse_from([
            "vanalyze",
            "models",
            "download",
            "--repo-id",
            "owner/model",
            "--name",
            "local-model-name",
            "--file",
            "config.json",
        ])
        .unwrap();

        match cli.command {
            Command::Models(ModelsArgs {
                command: ModelsCommand::Download(args),
            }) => assert_eq!(args.name.as_deref(), Some("local-model-name")),
            _ => panic!("expected models download command"),
        }
    }

    #[test]
    fn model_download_accepts_audio_model_tasks() {
        for (task, expected) in [
            ("audio-embedding", ModelTask::AudioEmbedding),
            ("speaker-diarization", ModelTask::SpeakerDiarization),
        ] {
            let cli = Cli::try_parse_from([
                "vanalyze",
                "models",
                "download",
                "--repo-id",
                "owner/model",
                "--task",
                task,
                "--file",
                "model.onnx",
            ])
            .unwrap();

            match cli.command {
                Command::Models(ModelsArgs {
                    command: ModelsCommand::Download(args),
                }) => assert_eq!(args.task.map(ModelTask::from), Some(expected.clone())),
                _ => panic!("expected models download command"),
            }
        }
    }

    #[test]
    fn model_download_accepts_bundle_dir() {
        let cli = Cli::try_parse_from([
            "vanalyze",
            "models",
            "download",
            "--preset",
            "yolos-tiny",
            "--bundle-dir",
            "models",
        ])
        .unwrap();

        match cli.command {
            Command::Models(ModelsArgs {
                command: ModelsCommand::Download(args),
            }) => assert_eq!(args.bundle_dir, PathBuf::from("models")),
            _ => panic!("expected models download command"),
        }
    }

    #[test]
    fn model_download_accepts_overwrite() {
        let cli = Cli::try_parse_from([
            "vanalyze",
            "models",
            "download",
            "--preset",
            "yolos-tiny",
            "--overwrite",
        ])
        .unwrap();

        match cli.command {
            Command::Models(ModelsArgs {
                command: ModelsCommand::Download(args),
            }) => assert!(args.overwrite),
            _ => panic!("expected models download command"),
        }
    }

    #[test]
    fn model_inspect_accepts_manifest() {
        let cli = Cli::try_parse_from([
            "vanalyze",
            "models",
            "inspect",
            "--manifest",
            "models/yolos-tiny/main/manifest.json",
        ])
        .unwrap();

        match cli.command {
            Command::Models(ModelsArgs {
                command: ModelsCommand::Inspect(args),
            }) => assert_eq!(
                args.manifest,
                Some(PathBuf::from("models/yolos-tiny/main/manifest.json"))
            ),
            _ => panic!("expected models inspect command"),
        }
    }

    #[test]
    fn model_inspect_accepts_name_and_bundle_dir() {
        let cli = Cli::try_parse_from([
            "vanalyze",
            "models",
            "inspect",
            "--name",
            "yolos-tiny",
            "--bundle-dir",
            "models",
            "--revision",
            "main",
        ])
        .unwrap();

        match cli.command {
            Command::Models(ModelsArgs {
                command: ModelsCommand::Inspect(args),
            }) => {
                assert_eq!(args.name.as_deref(), Some("yolos-tiny"));
                assert_eq!(args.bundle_dir, PathBuf::from("models"));
                assert_eq!(args.revision, "main");
            }
            _ => panic!("expected models inspect command"),
        }
    }

    #[test]
    fn model_inspect_requires_bundle_source() {
        let err = Cli::try_parse_from(["vanalyze", "models", "inspect"]).unwrap_err();

        assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn model_run_accepts_onnx_raw_frame_input() {
        let cli = Cli::try_parse_from([
            "vanalyze",
            "models",
            "run",
            "--manifest",
            "models/yolos-tiny/main/manifest.json",
            "--backend",
            "onnx",
            "--input",
            "frame.rgb",
            "--width",
            "640",
            "--height",
            "480",
            "--pixel-format",
            "rgb24",
        ])
        .unwrap();

        match cli.command {
            Command::Models(ModelsArgs {
                command: ModelsCommand::Run(args),
            }) => {
                assert_eq!(
                    args.manifest,
                    PathBuf::from("models/yolos-tiny/main/manifest.json")
                );
                assert_eq!(args.input, PathBuf::from("frame.rgb"));
                assert_eq!(args.width, 640);
                assert_eq!(args.height, 480);
            }
            _ => panic!("expected models run command"),
        }
    }

    #[test]
    fn model_bundle_format_includes_manifest_and_files() {
        let bundle = ModelBundle {
            root: PathBuf::from("models/yolos-tiny/main"),
            manifest: model_runtime::ModelBundleManifest {
                schema_version: 1,
                name: "yolos-tiny".to_string(),
                repo_id: "hustvl/yolos-tiny".to_string(),
                revision: "main".to_string(),
                task: ModelTask::ObjectDetection,
                files: BTreeMap::from([(
                    "config.json".to_string(),
                    model_runtime::ModelBundleFile {
                        remote_path: "config.json".to_string(),
                        local_path: "files/config.json".to_string(),
                        size_bytes: 42,
                    },
                )]),
            },
        };

        let output = format_model_bundle(&bundle);

        assert!(output.contains("name\tyolos-tiny"));
        assert!(output.contains("manifest\tmodels/yolos-tiny/main/manifest.json"));
        assert!(output.contains("config.json\tmodels/yolos-tiny/main/files/config.json\t42 bytes"));
    }

    #[test]
    fn detect_accepts_single_detector() {
        Cli::try_parse_from([
            "vanalyze",
            "detect",
            "--input",
            "video.mp4",
            "--detector",
            "content",
        ])
        .unwrap();
    }

    #[test]
    fn detect_accepts_preferred_frame_rate_spelling() {
        let cli = Cli::try_parse_from([
            "vanalyze",
            "detect",
            "--input",
            "video.mp4",
            "--frame-rate",
            "30000/1001",
        ])
        .unwrap();

        match cli.command {
            Command::Detect(args) => {
                assert_eq!(
                    args.detector_options.frame_rate.as_deref(),
                    Some("30000/1001")
                );
            }
            _ => panic!("expected detect command"),
        }
    }

    #[test]
    fn detect_accepts_legacy_framerate_alias() {
        let cli = Cli::try_parse_from([
            "vanalyze",
            "detect",
            "--input",
            "video.mp4",
            "--framerate",
            "29.97",
        ])
        .unwrap();

        match cli.command {
            Command::Detect(args) => {
                assert_eq!(args.detector_options.frame_rate.as_deref(), Some("29.97"));
            }
            _ => panic!("expected detect command"),
        }
    }

    #[test]
    fn detect_rejects_invalid_frame_rate() {
        let err = Cli::try_parse_from([
            "vanalyze",
            "detect",
            "--input",
            "video.mp4",
            "--frame-rate",
            "0",
        ])
        .unwrap_err();

        assert_eq!(err.kind(), ErrorKind::ValueValidation);
    }

    #[test]
    fn analyze_accepts_report_args() {
        let cli = Cli::try_parse_from([
            "vanalyze",
            "analyze",
            "--input",
            "demo.mp4",
            "--sample-every-frames",
            "10",
        ])
        .unwrap();

        match cli.command {
            Command::Analyze(args) => {
                assert_eq!(args.input, PathBuf::from("demo.mp4"));
                assert_eq!(args.sample_every_frames, 10);
                assert_eq!(args.model_backend, None);
            }
            _ => panic!("expected analyze command"),
        }
    }

    #[test]
    fn analyze_rejects_model_backend_without_manifest() {
        let err = Cli::try_parse_from([
            "vanalyze",
            "analyze",
            "--input",
            "demo.mp4",
            "--model-backend",
            "onnx",
        ])
        .unwrap_err();

        assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn analyze_report_serializes_expected_shape() {
        let mut dataset = AnalysisDataset::empty();
        let result = video_analysis_core::DetectionResult {
            frames_processed: 2,
            ..Default::default()
        };
        dataset.extend_detection_result(&result);
        let report = AnalyzeReport {
            input: "demo.mp4".to_string(),
            frames_processed: 2,
            sample_every_frames: 30,
            model: None,
            detection: AnalyzeDetectionReport {
                scene_count: 0,
                cut_count: 0,
            },
            summary: dataset_summary(&dataset),
            dataset,
        };

        let value = serde_json::to_value(report).unwrap();

        assert_eq!(value["input"], "demo.mp4");
        assert_eq!(value["framesProcessed"], 2);
        assert!(value.get("dataset").is_some());
        assert!(value.get("summary").is_some());
    }

    #[test]
    fn analyze_generated_tiny_video_without_model() {
        if std::process::Command::new("ffmpeg")
            .arg("-version")
            .status()
            .is_err()
        {
            return;
        }

        let dir = tempdir().unwrap();
        let input = dir.path().join("tiny.mp4");
        video_analysis_ffmpeg::write_two_scene_test_video(&input).unwrap();

        let report = build_analyze_report(&AnalyzeReportArgs {
            input,
            detection: DetectionSelection {
                detector: DetectorKind::Content,
                detectors: Vec::new(),
                combined_threshold: 0.5,
                detector_weights: Vec::new(),
            },
            output: None,
            sample_every_frames: 30,
            max_frames: Some(3),
            model_manifest: None,
            model_backend: None,
            detector_options: detector_options(),
        })
        .unwrap();

        let value = serde_json::to_value(report).unwrap();
        assert_eq!(value["framesProcessed"], 3);
        assert!(value.get("dataset").is_some());
        assert!(value["detection"].get("sceneCount").is_some());
        assert!(value["detection"].get("cutCount").is_some());
    }

    #[cfg(not(feature = "onnx"))]
    #[test]
    fn analyze_model_path_reports_feature_gated_error_without_onnx() {
        let err = build_analyze_report(&AnalyzeReportArgs {
            input: PathBuf::from("demo.mp4"),
            detection: DetectionSelection {
                detector: DetectorKind::Content,
                detectors: Vec::new(),
                combined_threshold: 0.5,
                detector_weights: Vec::new(),
            },
            output: None,
            sample_every_frames: 30,
            max_frames: None,
            model_manifest: Some(PathBuf::from("manifest.json")),
            model_backend: Some(ModelBackendKind::Onnx),
            detector_options: detector_options(),
        })
        .expect_err("onnx feature gate");

        assert!(err
            .to_string()
            .contains("requires building video-analysis-cli"));
    }

    #[test]
    fn detect_accepts_composite_detectors() {
        Cli::try_parse_from([
            "vanalyze",
            "detect",
            "--input",
            "video.mp4",
            "--detectors",
            "content,adaptive",
            "--combined-threshold",
            "0.5",
            "--detector-weight",
            "content=1.0",
        ])
        .unwrap();
    }

    #[test]
    fn detect_rejects_detector_and_detectors_together() {
        let err = Cli::try_parse_from([
            "vanalyze",
            "detect",
            "--input",
            "video.mp4",
            "--detector",
            "content",
            "--detectors",
            "content,adaptive",
        ])
        .unwrap_err();

        assert_eq!(err.kind(), ErrorKind::ArgumentConflict);
    }

    #[test]
    fn detect_rejects_invalid_detector_weight() {
        let err = Cli::try_parse_from([
            "vanalyze",
            "detect",
            "--input",
            "video.mp4",
            "--detectors",
            "content,adaptive",
            "--detector-weight",
            "content=abc",
        ])
        .unwrap_err();

        assert_eq!(err.kind(), ErrorKind::ValueValidation);
    }

    #[test]
    fn composite_builder_rejects_threshold_detector() {
        let selection = DetectionSelection {
            detector: DetectorKind::Content,
            detectors: vec![DetectorKind::Content, DetectorKind::Threshold],
            combined_threshold: 0.5,
            detector_weights: Vec::new(),
        };
        let err = match build_composite_detector_pipeline(&selection, &detector_options()) {
            Ok(_) => panic!("threshold detector should be rejected"),
            Err(err) => err,
        };

        assert!(err.to_string().contains("threshold"));
    }

    #[test]
    fn composite_builder_rejects_duplicate_weights() {
        let selection = DetectionSelection {
            detector: DetectorKind::Content,
            detectors: vec![DetectorKind::Content, DetectorKind::Adaptive],
            combined_threshold: 0.5,
            detector_weights: vec![
                DetectorWeight {
                    detector: DetectorKind::Content,
                    weight: 1.0,
                },
                DetectorWeight {
                    detector: DetectorKind::Content,
                    weight: 0.5,
                },
            ],
        };
        let err = match build_composite_detector_pipeline(&selection, &detector_options()) {
            Ok(_) => panic!("duplicate weights should be rejected"),
            Err(err) => err,
        };

        assert!(err.to_string().contains("duplicate weight"));
    }

    #[test]
    fn mesh_commands_accept_and_convert_files() {
        let cli = Cli::try_parse_from([
            "vanalyze", "mesh", "convert", "--input", "mesh.obj", "--output", "mesh.ply",
        ])
        .unwrap();
        match cli.command {
            Command::Mesh(MeshArgs {
                command: MeshCommand::Convert(args),
            }) => {
                assert_eq!(args.input, PathBuf::from("mesh.obj"));
                assert_eq!(args.output, PathBuf::from("mesh.ply"));
            }
            _ => panic!("expected mesh convert command"),
        }

        let dir = tempdir().unwrap();
        let input = dir.path().join("mesh.obj");
        std::fs::write(&input, "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n").unwrap();
        let output = dir.path().join("mesh.ply");
        convert_mesh(MeshConvertArgs {
            input: input.clone(),
            output: output.clone(),
        })
        .unwrap();
        assert!(output.exists());
        inspect_mesh(MeshInspectArgs { input }).unwrap();
    }

    #[test]
    fn posture_commands_estimate_and_export() {
        let cli = Cli::try_parse_from([
            "vanalyze",
            "posture",
            "estimate",
            "--predictions-json",
            "poses.json",
            "--output",
            "poses_out.json",
        ])
        .unwrap();
        match cli.command {
            Command::Posture(PostureArgs {
                command: PostureCommand::Estimate(args),
            }) => {
                assert_eq!(args.predictions_json, Some(PathBuf::from("poses.json")));
                assert_eq!(args.output, PathBuf::from("poses_out.json"));
            }
            _ => panic!("expected posture estimate command"),
        }

        let dir = tempdir().unwrap();
        let predictions = dir.path().join("predictions.json");
        std::fs::write(
            &predictions,
            serde_json::to_vec_pretty(&vec![RawPose2dPrediction {
                keypoints: Skeleton::coco_17()
                    .keypoints
                    .iter()
                    .enumerate()
                    .map(|(index, name)| video_analysis_recognition::RawKeypoint2d {
                        name: name.clone(),
                        x: index as f32,
                        y: index as f32,
                        score: Some(1.0),
                        visible: Some(true),
                    })
                    .collect(),
                ..RawPose2dPrediction::default()
            }])
            .unwrap(),
        )
        .unwrap();
        let coco = dir.path().join("poses.json");
        estimate_posture(PostureEstimateArgs {
            manifest: None,
            predictions_json: Some(predictions),
            backend: ModelBackendKind::Onnx,
            input: None,
            width: None,
            height: None,
            pixel_format: RawPixelFormatKind::Rgb24,
            output: coco.clone(),
        })
        .unwrap();
        assert!(coco.exists());

        let gltf = dir.path().join("pose.gltf");
        export_posture(PostureExportArgs {
            input: coco,
            output: gltf.clone(),
        })
        .unwrap();
        assert!(gltf.exists());
    }

    #[test]
    fn package_conf_arguments_are_loaded() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("video-analysis-cli.conf"), "models presets").unwrap();

        let cli = parse_cli_from(dir.path().to_path_buf(), [OsString::from("vanalyze")]).unwrap();

        match cli.command {
            Command::Models(ModelsArgs {
                command: ModelsCommand::Presets,
            }) => {}
            other => panic!("unexpected command from package conf: {other:?}"),
        }
    }

    #[test]
    fn binary_name_conf_arguments_are_loaded_before_package_conf() {
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join("video-analysis-cli.conf"),
            "detect --input package.mp4",
        )
        .unwrap();
        std::fs::write(dir.path().join("vanalyze.conf"), "models presets").unwrap();

        let cli = parse_cli_from(dir.path().to_path_buf(), [OsString::from("vanalyze")]).unwrap();

        match cli.command {
            Command::Models(ModelsArgs {
                command: ModelsCommand::Presets,
            }) => {}
            other => panic!("unexpected command from binary config: {other:?}"),
        }
    }

    #[test]
    fn explicit_config_arguments_are_loaded() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("custom.conf"), "models presets").unwrap();

        let cli = parse_cli_from(
            dir.path().to_path_buf(),
            [
                OsString::from("vanalyze"),
                OsString::from("--config"),
                OsString::from("custom.conf"),
            ],
        )
        .unwrap();

        match cli.command {
            Command::Models(ModelsArgs {
                command: ModelsCommand::Presets,
            }) => {}
            other => panic!("unexpected command from explicit config: {other:?}"),
        }
    }

    #[test]
    fn explicit_config_replaces_package_conf() {
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join("video-analysis-cli.conf"),
            "detect --input auto.mp4",
        )
        .unwrap();
        std::fs::write(dir.path().join("custom.conf"), "models presets").unwrap();

        let cli = parse_cli_from(
            dir.path().to_path_buf(),
            [
                OsString::from("vanalyze"),
                OsString::from("--config=custom.conf"),
            ],
        )
        .unwrap();

        match cli.command {
            Command::Models(ModelsArgs {
                command: ModelsCommand::Presets,
            }) => {}
            other => panic!("unexpected command from explicit config: {other:?}"),
        }
    }

    #[test]
    fn explicit_cli_arguments_override_package_conf_arguments() {
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join("video-analysis-cli.conf"),
            "detect --input config.mp4 --min-scene-len 3",
        )
        .unwrap();

        let cli = parse_cli_from(
            dir.path().to_path_buf(),
            [
                OsString::from("vanalyze"),
                OsString::from("--input"),
                OsString::from("cli.mp4"),
                OsString::from("--min-scene-len"),
                OsString::from("7"),
            ],
        )
        .unwrap();

        match cli.command {
            Command::Detect(args) => {
                assert_eq!(args.input, PathBuf::from("cli.mp4"));
                assert_eq!(args.detector_options.min_scene_len, 7);
            }
            other => panic!("unexpected command from merged CLI args: {other:?}"),
        }
    }
}
