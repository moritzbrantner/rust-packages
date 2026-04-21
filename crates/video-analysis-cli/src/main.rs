use std::fs::File;
use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use video_analysis_core::{Result, SceneDetector, ScenePipeline};
use video_analysis_detectors::{
    AdaptiveDetector, ContentDetector, HashDetector, HistogramDetector, ThresholdDetector,
};
use video_analysis_ffmpeg::FfmpegVideoSource;
use video_analysis_models::{HuggingFaceDownloader, HuggingFaceModelSpec, ModelPreset, ModelTask};
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
    #[arg(long, value_enum, default_value_t = DetectorKind::Content)]
    detector: DetectorKind,
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
    #[arg(long, value_enum, default_value_t = DetectorKind::Content)]
    detector: DetectorKind,
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
}

#[derive(Debug, Parser)]
struct ModelDownloadArgs {
    #[arg(long, value_enum)]
    preset: Option<ModelPresetKind>,
    #[arg(long)]
    repo_id: Option<String>,
    #[arg(long, default_value = "main")]
    revision: String,
    #[arg(long, value_enum)]
    task: Option<ModelTaskKind>,
    #[arg(long = "file")]
    files: Vec<String>,
    #[arg(long)]
    cache_dir: Option<PathBuf>,
    #[arg(long)]
    token: Option<String>,
    #[arg(long, default_value_t = false)]
    no_progress: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum DetectorKind {
    Content,
    Adaptive,
    Threshold,
    Histogram,
    Hash,
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
            let result = run_detection(&args.input, args.detector, &args.detector_options)?;
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
            let result = run_detection(&args.input, args.detector, &args.detector_options)?;
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
            let result = run_detection(&args.input, args.detector, &args.detector_options)?;
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
            if args.files.is_empty() {
                return Err(video_analysis_core::DetectError::InvalidArgument(
                    "--file is required when --repo-id is used".to_string(),
                ));
            }
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
        (Some(_), Some(_)) => {
            return Err(video_analysis_core::DetectError::InvalidArgument(
                "use either --preset or --repo-id, not both".to_string(),
            ));
        }
        (None, None) => {
            return Err(video_analysis_core::DetectError::InvalidArgument(
                "either --preset or --repo-id is required".to_string(),
            ));
        }
    };

    let mut downloader = HuggingFaceDownloader::new().progress(!args.no_progress);
    if let Some(cache_dir) = args.cache_dir {
        downloader = downloader.cache_dir(cache_dir);
    }
    if let Some(token) = args.token {
        downloader = downloader.token(token);
    }

    let downloaded = downloader.download(&spec)?;
    println!(
        "downloaded {} from {}",
        downloaded.spec.name, downloaded.spec.repo_id
    );
    for (remote, local) in downloaded.files {
        println!("{remote}\t{}", local.display());
    }
    Ok(())
}

fn run_detection(
    input: &PathBuf,
    detector: DetectorKind,
    options: &DetectorOptions,
) -> Result<video_analysis_core::DetectionResult> {
    let mut source = FfmpegVideoSource::open(input)?;
    let mut pipeline = match detector {
        DetectorKind::Content => ScenePipeline::builder()
            .detector(
                ContentDetector::new(options.threshold.unwrap_or(27.0), options.min_scene_len)
                    .luma_only(options.luma_only),
            )
            .start_in_scene(true)
            .build()?,
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
            .build()?,
        DetectorKind::Threshold => ScenePipeline::builder()
            .detector(ThresholdDetector::new(
                options.threshold.unwrap_or(12.0),
                options.min_scene_len,
            ))
            .start_in_scene(true)
            .build()?,
        DetectorKind::Histogram => ScenePipeline::builder()
            .detector(HistogramDetector::new(
                options.threshold.unwrap_or(0.05),
                options.bins,
                options.min_scene_len,
            ))
            .start_in_scene(true)
            .build()?,
        DetectorKind::Hash => ScenePipeline::builder()
            .detector(HashDetector::new(
                options.threshold.unwrap_or(0.395),
                options.hash_size,
                options.lowpass,
                options.min_scene_len,
            ))
            .start_in_scene(true)
            .build()?,
    };
    pipeline.detect(&mut source)
}

#[allow(dead_code)]
fn _assert_detector_trait<T: SceneDetector>() {}
