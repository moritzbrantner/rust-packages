use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::path::PathBuf;

use clap::{ArgGroup, Parser, Subcommand, ValueEnum};
use video_analysis_core::{Result, SceneDetector, ScenePipeline};
use video_analysis_detectors::{
    AdaptiveDetector, AdaptiveScoreAlgorithm, ContentDetector, ContentScoreAlgorithm, HashDetector,
    HashScoreAlgorithm, HistogramDetector, HistogramScoreAlgorithm, ThresholdDetector,
    WeightedComponent, WeightedCompositeDetector,
};
use video_analysis_ffmpeg::FfmpegVideoSource;
use video_analysis_models::{
    HuggingFaceDownloader, HuggingFaceModelSpec, ModelBundle, ModelBundleStore, ModelPreset,
    ModelTask,
};
use video_analysis_output::{write_scene_list_csv, write_stats_csv};
use video_analysis_split::{split_video_ffmpeg, SplitOptions};

#[derive(Debug, Parser)]
#[command(name = "vanalyze", version, about = "Rust video scene analysis")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Detect(AnalyzeArgs),
    List(AnalyzeArgs),
    Split(SplitArgs),
    Models(ModelsArgs),
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
struct ModelsArgs {
    #[command(subcommand)]
    command: ModelsCommand,
}

#[derive(Debug, Subcommand)]
enum ModelsCommand {
    Presets,
    Download(ModelDownloadArgs),
    Inspect(ModelInspectArgs),
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
    #[arg(long, default_value = "main")]
    revision: String,
    #[arg(long, value_enum)]
    task: Option<ModelTaskKind>,
    #[arg(long = "file")]
    files: Vec<String>,
    #[arg(long, default_value = ".video-analysis-models")]
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
    #[arg(long, default_value = ".video-analysis-models", requires = "name")]
    bundle_dir: PathBuf,
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

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ModelPresetKind {
    #[value(name = "detr-resnet-50")]
    DetrResnet50,
    YolosTiny,
    DistilbertSst2,
    BertBaseNer,
    #[value(name = "minilm-l6-v2")]
    MiniLmL6V2,
}

impl From<ModelPresetKind> for ModelPreset {
    fn from(value: ModelPresetKind) -> Self {
        match value {
            ModelPresetKind::DetrResnet50 => Self::DetrResnet50,
            ModelPresetKind::YolosTiny => Self::YolosTiny,
            ModelPresetKind::DistilbertSst2 => Self::DistilbertSst2,
            ModelPresetKind::BertBaseNer => Self::BertBaseNer,
            ModelPresetKind::MiniLmL6V2 => Self::MiniLmL6V2,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ModelTaskKind {
    ObjectDetection,
    ImageClassification,
    TextClassification,
    TokenClassification,
    ZeroShotClassification,
    TextEmbedding,
}

impl From<ModelTaskKind> for ModelTask {
    fn from(value: ModelTaskKind) -> Self {
        match value {
            ModelTaskKind::ObjectDetection => Self::ObjectDetection,
            ModelTaskKind::ImageClassification => Self::ImageClassification,
            ModelTaskKind::TextClassification => Self::TextClassification,
            ModelTaskKind::TokenClassification => Self::TokenClassification,
            ModelTaskKind::ZeroShotClassification => Self::ZeroShotClassification,
            ModelTaskKind::TextEmbedding => Self::TextEmbedding,
        }
    }
}

#[derive(Debug, Parser, Clone)]
struct DetectorOptions {
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
    let cli = Cli::parse();
    match cli.command {
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
        },
    }
    Ok(())
}

fn list_model_presets() {
    for preset in ModelPreset::ALL {
        let spec = preset.spec();
        println!(
            "{:<18} {:<48} {:?}",
            preset.as_str(),
            spec.repo_id,
            spec.task
        );
    }
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

    let bundle = ModelBundleStore::new(args.bundle_dir)
        .downloader(downloader)
        .overwrite(args.overwrite)
        .download(&spec)?;
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
        ModelBundle::load(manifest)?
    } else {
        let name = args
            .name
            .expect("clap validates model bundle source arguments");
        ModelBundleStore::new(args.bundle_dir).load(name, args.revision)?
    };
    print!("{}", format_model_bundle(&bundle));
    Ok(())
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

fn run_detection(
    input: &PathBuf,
    selection: &DetectionSelection,
    options: &DetectorOptions,
) -> Result<video_analysis_core::DetectionResult> {
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

    fn detector_options() -> DetectorOptions {
        DetectorOptions {
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
    fn model_bundle_format_includes_manifest_and_files() {
        let bundle = ModelBundle {
            root: PathBuf::from("models/yolos-tiny/main"),
            manifest: video_analysis_models::ModelBundleManifest {
                schema_version: 1,
                name: "yolos-tiny".to_string(),
                repo_id: "hustvl/yolos-tiny".to_string(),
                revision: "main".to_string(),
                task: ModelTask::ObjectDetection,
                files: BTreeMap::from([(
                    "config.json".to_string(),
                    video_analysis_models::ModelBundleFile {
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
}
