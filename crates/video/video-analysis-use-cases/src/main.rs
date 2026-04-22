use std::path::PathBuf;

use clap::{ArgGroup, Parser, Subcommand};
use video_analysis_core::Result;
use video_analysis_use_cases::youtube::{
    run_youtube_video, write_youtube_video_report, YoutubeVideoRequest,
};

#[derive(Debug, Parser)]
#[command(
    name = "video-analysis-use-cases",
    version,
    about = "Runnable video-analysis workspace use cases"
)]
struct Cli {
    #[command(subcommand)]
    command: CommandKind,
}

#[derive(Debug, Subcommand)]
enum CommandKind {
    YoutubeVideo(YoutubeVideoArgs),
}

#[derive(Debug, Parser)]
#[command(group(
    ArgGroup::new("video_source")
        .args(["url", "input"])
        .required(true)
))]
struct YoutubeVideoArgs {
    #[arg(long)]
    url: Option<String>,
    #[arg(long)]
    input: Option<PathBuf>,
    #[arg(long, default_value = "use-case-output/youtube-video")]
    work_dir: PathBuf,
    #[arg(long)]
    output: Option<PathBuf>,
    #[arg(long, default_value_t = 27.0)]
    scene_threshold: f32,
    #[arg(long, default_value_t = 15)]
    min_scene_len: u64,
    #[arg(long)]
    max_frames: Option<u64>,
    #[arg(long, default_value_t = 30)]
    visual_sample_every: u64,
    #[arg(long)]
    skip_transcription: bool,
    #[arg(long)]
    transcriber_command: Option<PathBuf>,
    #[arg(long = "transcriber-arg")]
    transcriber_args: Vec<String>,
    #[arg(long)]
    object_command: Option<PathBuf>,
    #[arg(long = "object-arg")]
    object_args: Vec<String>,
    #[arg(long)]
    ocr_command: Option<PathBuf>,
    #[arg(long = "ocr-arg")]
    ocr_args: Vec<String>,
    #[arg(long)]
    text_command: Option<PathBuf>,
    #[arg(long = "text-arg")]
    text_args: Vec<String>,
}

impl From<YoutubeVideoArgs> for YoutubeVideoRequest {
    fn from(args: YoutubeVideoArgs) -> Self {
        Self {
            url: args.url,
            input: args.input,
            work_dir: args.work_dir,
            output: args.output,
            scene_threshold: args.scene_threshold,
            min_scene_len: args.min_scene_len,
            max_frames: args.max_frames,
            visual_sample_every: args.visual_sample_every,
            skip_transcription: args.skip_transcription,
            transcriber_command: args.transcriber_command,
            transcriber_args: args.transcriber_args,
            object_command: args.object_command,
            object_args: args.object_args,
            ocr_command: args.ocr_command,
            ocr_args: args.ocr_args,
            text_command: args.text_command,
            text_args: args.text_args,
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        CommandKind::YoutubeVideo(args) => {
            let request = YoutubeVideoRequest::from(args);
            let report_path = request
                .output
                .clone()
                .unwrap_or_else(|| request.work_dir.join("analysis.json"));
            let report = run_youtube_video(request)?;
            write_youtube_video_report(&report_path, &report)?;
            println!("{}", report_path.display());
        }
    }
    Ok(())
}
