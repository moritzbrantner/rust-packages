use std::ffi::OsString;
use std::path::{Path, PathBuf};

use clap::{ArgGroup, CommandFactory, FromArgMatches, Parser, Subcommand};
use video_analysis_core::Result;
use video_analysis_radiance_pipeline::{
    RadianceTrainingMethod, VideoToRadiancePipeline, VideoToRadianceRequest,
};
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
    RadianceScene(RadianceSceneArgs),
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

#[derive(Debug, Parser)]
struct RadianceSceneArgs {
    #[arg(long)]
    input: PathBuf,
    #[arg(long, default_value = "use-case-output/radiance-scene")]
    work_dir: PathBuf,
    #[arg(long)]
    output: Option<PathBuf>,
    #[arg(long, default_value = "splatfacto")]
    method: String,
    #[arg(long, default_value_t = 10)]
    frame_sample_every: u32,
    #[arg(long)]
    max_frames: Option<u32>,
    #[arg(long)]
    run_training: bool,
    #[arg(long, default_value = "ffmpeg")]
    ffmpeg_command: PathBuf,
    #[arg(long, default_value = "colmap")]
    colmap_command: PathBuf,
    #[arg(long, default_value = "ns-process-data")]
    ns_process_data_command: PathBuf,
    #[arg(long, default_value = "ns-train")]
    ns_train_command: PathBuf,
    #[arg(long, default_value = "ns-export")]
    ns_export_command: PathBuf,
    #[arg(long = "colmap-arg")]
    extra_colmap_args: Vec<String>,
    #[arg(long = "train-arg")]
    extra_train_args: Vec<String>,
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

impl TryFrom<RadianceSceneArgs> for VideoToRadianceRequest {
    type Error = video_analysis_core::DetectError;

    fn try_from(args: RadianceSceneArgs) -> Result<Self> {
        let method = match args.method.as_str() {
            "splatfacto" => RadianceTrainingMethod::Splatfacto,
            "nerfacto" => RadianceTrainingMethod::Nerfacto,
            other => {
                return Err(video_analysis_core::DetectError::InvalidArgument(format!(
                    "unsupported radiance method `{other}`"
                )))
            }
        };
        Ok(Self {
            input: args.input,
            work_dir: args.work_dir,
            frame_sample_every: args.frame_sample_every,
            max_frames: args.max_frames,
            method,
            run_training: args.run_training,
            ffmpeg_command: args.ffmpeg_command,
            colmap_command: args.colmap_command,
            ns_process_data_command: args.ns_process_data_command,
            ns_train_command: args.ns_train_command,
            ns_export_command: args.ns_export_command,
            extra_colmap_args: args.extra_colmap_args.into_iter().map(Into::into).collect(),
            extra_train_args: args.extra_train_args.into_iter().map(Into::into).collect(),
        })
    }
}

fn main() -> Result<()> {
    let cli = parse_cli()?;
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
        CommandKind::RadianceScene(args) => {
            let output = args
                .output
                .clone()
                .unwrap_or_else(|| args.work_dir.join("radiance-scene.json"));
            let request = VideoToRadianceRequest::try_from(args)?;
            let result = VideoToRadiancePipeline::run(request)?;
            let report = serde_json::json!({
                "use_case": "radiance-scene",
                "frames_dir": result.frames_dir,
                "colmap_dir": result.colmap_dir,
                "nerfstudio_dir": result.nerfstudio_dir,
                "export_dir": result.export_dir,
                "splat_ply": result.splat_ply,
                "view_count": result.view_set.as_ref().map(|views| views.view_count()),
                "gaussian_stats": result.gaussian_stats.as_ref().map(|stats| serde_json::json!({
                    "count": stats.count,
                    "mean_opacity": stats.mean_opacity,
                    "bounds": stats.bounds.as_ref().map(|bounds| serde_json::json!({
                        "min": [bounds.min.x, bounds.min.y, bounds.min.z],
                        "max": [bounds.max.x, bounds.max.y, bounds.max.z]
                    })),
                    "min_scale": [stats.min_scale.x, stats.min_scale.y, stats.min_scale.z],
                    "max_scale": [stats.max_scale.x, stats.max_scale.y, stats.max_scale.z]
                })),
                "completed": result.completed,
                "skipped": result.skipped
            });
            if let Some(parent) = output.parent().filter(|path| !path.as_os_str().is_empty()) {
                std::fs::create_dir_all(parent)?;
            }
            let file = std::fs::File::create(&output)?;
            serde_json::to_writer_pretty(file, &report).map_err(|err| {
                video_analysis_core::DetectError::Source(format!(
                    "failed to write radiance-scene report: {err}"
                ))
            })?;
            println!("{}", output.display());
        }
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
    Cli::from_arg_matches(&matches).map_err(|err| {
        video_analysis_core::DetectError::Source(format!("failed to parse CLI arguments: {err}"))
    })
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
    let mut raw_args = raw_args.into_iter().map(Into::into);
    let program = raw_args
        .next()
        .unwrap_or_else(|| OsString::from(package_name));
    let mut args = vec![program];
    let conf_path = current_dir.join(format!("{package_name}.conf"));
    if conf_path.is_file() {
        args.extend(read_conf_args(&conf_path)?);
    }
    args.extend(raw_args);
    Ok(args)
}

fn read_conf_args(path: &Path) -> Result<Vec<OsString>> {
    let contents = std::fs::read_to_string(path)?;
    let Some(args) = shlex::split(&contents) else {
        return Err(video_analysis_core::DetectError::InvalidArgument(format!(
            "failed to parse config file `{}` as shell-style CLI arguments",
            path.display()
        )));
    };
    Ok(args.into_iter().map(OsString::from).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_conf_arguments_are_loaded() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("video-analysis-use-cases.conf"),
            "youtube-video --input input.mp4",
        )
        .unwrap();

        let cli = parse_cli_from(
            dir.path().to_path_buf(),
            [OsString::from("video-analysis-use-cases")],
        )
        .unwrap();

        match cli.command {
            CommandKind::YoutubeVideo(args) => {
                assert_eq!(args.input, Some(PathBuf::from("input.mp4")));
            }
            other => panic!("unexpected command from package conf: {other:?}"),
        }
    }

    #[test]
    fn explicit_cli_arguments_override_package_conf_arguments() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("video-analysis-use-cases.conf"),
            "youtube-video --input config.mp4 --work-dir from-conf",
        )
        .unwrap();

        let cli = parse_cli_from(
            dir.path().to_path_buf(),
            [
                OsString::from("video-analysis-use-cases"),
                OsString::from("--input"),
                OsString::from("cli.mp4"),
                OsString::from("--work-dir"),
                OsString::from("from-cli"),
            ],
        )
        .unwrap();

        match cli.command {
            CommandKind::YoutubeVideo(args) => {
                assert_eq!(args.input, Some(PathBuf::from("cli.mp4")));
                assert_eq!(args.work_dir, PathBuf::from("from-cli"));
            }
            other => panic!("unexpected command from merged CLI args: {other:?}"),
        }
    }
}
