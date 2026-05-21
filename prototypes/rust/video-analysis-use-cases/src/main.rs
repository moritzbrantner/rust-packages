//! Internal module support for main.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use clap::{ArgGroup, CommandFactory, FromArgMatches, Parser, Subcommand};
use video_analysis_core::Result;
use video_analysis_use_cases::audio_voice_analysis::{
    run_audio_voice_analysis, write_audio_voice_analysis_report, AudioVoiceAnalysisRequest,
};
use video_analysis_use_cases::image_person_edit::{
    run_image_person_edit, write_image_person_edit_report, ImagePersonEditRequest,
    PersonDetectorConfig,
};
use video_analysis_use_cases::video_red_cars::{
    run_video_red_cars, write_video_red_cars_report, VideoRedCarsRequest,
};
use video_analysis_use_cases::youtube::{
    run_youtube_video, write_youtube_video_report, AudioSeparationConfig, TranscriptionEngine,
    WhisperCppConfig, YoutubeVideoRequest,
};

#[derive(Debug, Parser)]
#[command(
    name = "video-analysis-use-cases",
    version,
    about = "Runnable video-analysis workspace use cases"
)]
struct Cli {
    #[arg(
        long,
        global = true,
        value_name = "PATH",
        help = "Read shell-style CLI arguments from a config file"
    )]
    config: Option<PathBuf>,
    #[command(subcommand)]
    command: CommandKind,
}

#[derive(Debug, Subcommand)]
enum CommandKind {
    YoutubeVideo(YoutubeVideoArgs),
    VideoRedCars(VideoRedCarsArgs),
    AudioVoiceAnalysis(AudioVoiceAnalysisArgs),
    ImagePersonEdit(ImagePersonEditArgs),
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
            transcriber_engine: if args.transcriber_command.is_some() {
                TranscriptionEngine::Whisper
            } else {
                TranscriptionEngine::WhisperCpp
            },
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
            whisper_cpp: WhisperCppConfig::default(),
            audio_separation: AudioSeparationConfig::default(),
            object_command: args.object_command,
            object_args: args.object_args,
            ocr_command: args.ocr_command,
            ocr_args: args.ocr_args,
            text_command: args.text_command,
            text_args: args.text_args,
        }
    }
}

#[derive(Debug, Parser)]
struct VideoRedCarsArgs {
    #[arg(long)]
    input: PathBuf,
    #[arg(long, default_value = "use-case-output/video-red-cars")]
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
    vehicle_detector_command: Option<PathBuf>,
    #[arg(long = "vehicle-detector-arg")]
    vehicle_detector_args: Vec<String>,
}

impl From<VideoRedCarsArgs> for VideoRedCarsRequest {
    fn from(args: VideoRedCarsArgs) -> Self {
        Self {
            input: args.input,
            work_dir: args.work_dir,
            output: args.output,
            scene_threshold: args.scene_threshold,
            min_scene_len: args.min_scene_len,
            max_frames: args.max_frames,
            visual_sample_every: args.visual_sample_every,
            vehicle_detector_command: args.vehicle_detector_command.unwrap_or_default(),
            vehicle_detector_args: args.vehicle_detector_args,
        }
    }
}

#[derive(Debug, Parser)]
struct AudioVoiceAnalysisArgs {
    #[arg(long)]
    input: PathBuf,
    #[arg(long, default_value = "use-case-output/audio-voice-analysis")]
    work_dir: PathBuf,
    #[arg(long)]
    output: Option<PathBuf>,
    #[arg(long)]
    transcriber_command: Option<PathBuf>,
    #[arg(long = "transcriber-arg")]
    transcriber_args: Vec<String>,
    #[arg(long)]
    demucs_command: Option<PathBuf>,
    #[arg(long)]
    require_voice_stem: bool,
}

impl From<AudioVoiceAnalysisArgs> for AudioVoiceAnalysisRequest {
    fn from(args: AudioVoiceAnalysisArgs) -> Self {
        let mut request = AudioVoiceAnalysisRequest {
            input: args.input,
            work_dir: args.work_dir,
            output: args.output,
            ..AudioVoiceAnalysisRequest::default()
        };
        if let Some(command) = args.transcriber_command {
            request.transcription.engine = TranscriptionEngine::Whisper;
            request.transcription.command = Some(video_analysis_use_cases::ExternalCommandConfig {
                command,
                args: args.transcriber_args,
            });
        }
        if let Some(command) = args.demucs_command {
            request.audio_separation.command =
                Some(video_analysis_use_cases::ExternalCommandConfig {
                    command,
                    args: Vec::new(),
                });
        }
        request.require_voice_stem = args.require_voice_stem;
        request
    }
}

#[derive(Debug, Parser)]
struct ImagePersonEditArgs {
    #[arg(long)]
    input: PathBuf,
    #[arg(long, default_value = "use-case-output/image-person-edit")]
    work_dir: PathBuf,
    #[arg(long)]
    output: Option<PathBuf>,
    #[arg(long)]
    prompt: String,
    #[arg(long, default_value = "")]
    negative_prompt: String,
    #[arg(long)]
    model: String,
    #[arg(long)]
    person_detector_command: Option<PathBuf>,
    #[arg(long = "person-detector-arg")]
    person_detector_args: Vec<String>,
    #[arg(long)]
    editor_command: Option<PathBuf>,
    #[arg(long = "editor-arg")]
    editor_args: Vec<String>,
}

impl From<ImagePersonEditArgs> for ImagePersonEditRequest {
    fn from(args: ImagePersonEditArgs) -> Self {
        let person_detector_args = args.person_detector_args;
        let person_detector = args
            .person_detector_command
            .clone()
            .map(|command| PersonDetectorConfig::ExternalCommand {
                command,
                args: person_detector_args.clone(),
            })
            .unwrap_or_default();
        Self {
            input: args.input,
            work_dir: args.work_dir,
            output: args.output,
            prompt: args.prompt,
            negative_prompt: args.negative_prompt,
            model: args.model,
            person_detector_command: args.person_detector_command.clone().unwrap_or_default(),
            person_detector_args,
            editor_command: args.editor_command,
            editor_args: args.editor_args,
            person_detector,
            comfyui: image_analysis_comfyui::ComfyUiClientOptions::default(),
        }
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
        CommandKind::VideoRedCars(args) => {
            let request = VideoRedCarsRequest::from(args);
            let report_path = request
                .output
                .clone()
                .unwrap_or_else(|| request.work_dir.join("analysis.json"));
            let report = run_video_red_cars(request)?;
            write_video_red_cars_report(&report_path, &report)?;
            println!("{}", report_path.display());
        }
        CommandKind::AudioVoiceAnalysis(args) => {
            let request = AudioVoiceAnalysisRequest::from(args);
            let report_path = request
                .output
                .clone()
                .unwrap_or_else(|| request.work_dir.join("analysis.json"));
            let report = run_audio_voice_analysis(request)?;
            write_audio_voice_analysis_report(&report_path, &report)?;
            println!("{}", report_path.display());
        }
        CommandKind::ImagePersonEdit(args) => {
            let request = ImagePersonEditRequest::from(args);
            let report_path = request
                .output
                .clone()
                .unwrap_or_else(|| request.work_dir.join("analysis.json"));
            let report = run_image_person_edit(request)?;
            write_image_person_edit_report(&report_path, &report)?;
            println!("{}", report_path.display());
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
    let cli = Cli::from_arg_matches(&matches).map_err(|err| {
        video_analysis_core::DetectError::Source(format!("failed to parse CLI arguments: {err}"))
    })?;
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

    let package_conf_path = current_dir.join(format!("{package_name}.conf"));
    package_conf_path.is_file().then_some(package_conf_path)
}

fn explicit_config_path(raw_args: &[OsString]) -> Result<Option<PathBuf>> {
    let mut config_path = None;
    let mut index = 0;
    while index < raw_args.len() {
        let arg = raw_args[index].as_os_str();
        if arg == "--config" {
            index += 1;
            let Some(value) = raw_args.get(index) else {
                return Err(video_analysis_core::DetectError::InvalidArgument(
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
            other => panic!("expected youtube-video command, got {other:?}"),
        }
    }

    #[test]
    fn explicit_config_arguments_are_loaded() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("custom.conf"),
            "youtube-video --input input.mp4",
        )
        .unwrap();

        let cli = parse_cli_from(
            dir.path().to_path_buf(),
            [
                OsString::from("video-analysis-use-cases"),
                OsString::from("--config"),
                OsString::from("custom.conf"),
            ],
        )
        .unwrap();

        match cli.command {
            CommandKind::YoutubeVideo(args) => {
                assert_eq!(args.input, Some(PathBuf::from("input.mp4")));
            }
            other => panic!("expected youtube-video command, got {other:?}"),
        }
    }

    #[test]
    fn explicit_config_replaces_package_conf() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("video-analysis-use-cases.conf"),
            "video-red-cars --input auto.mp4",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("custom.conf"),
            "youtube-video --input explicit.mp4",
        )
        .unwrap();

        let cli = parse_cli_from(
            dir.path().to_path_buf(),
            [
                OsString::from("video-analysis-use-cases"),
                OsString::from("--config=custom.conf"),
            ],
        )
        .unwrap();

        match cli.command {
            CommandKind::YoutubeVideo(args) => {
                assert_eq!(args.input, Some(PathBuf::from("explicit.mp4")));
            }
            other => panic!("expected youtube-video command, got {other:?}"),
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
            other => panic!("expected youtube-video command, got {other:?}"),
        }
    }
}
