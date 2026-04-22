use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use video_analysis_core::{DetectError, Result};
use video_analysis_gaussian_splatting::GaussianSceneStats;
use video_analysis_radiance_fields::CameraViewSet;
use video_analysis_radiance_io::{
    colmap_to_view_set, read_colmap_text_dir, read_gaussian_splat_ply,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadianceTrainingMethod {
    Splatfacto,
    Nerfacto,
}

impl RadianceTrainingMethod {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Splatfacto => "splatfacto",
            Self::Nerfacto => "nerfacto",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VideoToRadianceRequest {
    pub input: PathBuf,
    pub work_dir: PathBuf,
    pub frame_sample_every: u32,
    pub max_frames: Option<u32>,
    pub method: RadianceTrainingMethod,
    pub run_training: bool,
    pub ffmpeg_command: PathBuf,
    pub colmap_command: PathBuf,
    pub ns_process_data_command: PathBuf,
    pub ns_train_command: PathBuf,
    pub ns_export_command: PathBuf,
    pub extra_colmap_args: Vec<OsString>,
    pub extra_train_args: Vec<OsString>,
}

impl VideoToRadianceRequest {
    pub fn new(input: impl Into<PathBuf>, work_dir: impl Into<PathBuf>) -> Self {
        Self {
            input: input.into(),
            work_dir: work_dir.into(),
            ..Self::default()
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.input.as_os_str().is_empty() {
            return Err(DetectError::InvalidArgument(
                "input path must not be empty".to_string(),
            ));
        }
        if self.work_dir.as_os_str().is_empty() {
            return Err(DetectError::InvalidArgument(
                "work_dir must not be empty".to_string(),
            ));
        }
        if self.frame_sample_every == 0 {
            return Err(DetectError::InvalidArgument(
                "frame_sample_every must be positive".to_string(),
            ));
        }
        for (name, command) in [
            ("ffmpeg_command", &self.ffmpeg_command),
            ("colmap_command", &self.colmap_command),
            ("ns_process_data_command", &self.ns_process_data_command),
            ("ns_train_command", &self.ns_train_command),
            ("ns_export_command", &self.ns_export_command),
        ] {
            if command.as_os_str().is_empty() {
                return Err(DetectError::InvalidArgument(format!(
                    "{name} must not be empty"
                )));
            }
        }
        Ok(())
    }
}

impl Default for VideoToRadianceRequest {
    fn default() -> Self {
        Self {
            input: PathBuf::new(),
            work_dir: PathBuf::from("use-case-output/radiance-scene"),
            frame_sample_every: 10,
            max_frames: None,
            method: RadianceTrainingMethod::Splatfacto,
            run_training: false,
            ffmpeg_command: PathBuf::from("ffmpeg"),
            colmap_command: PathBuf::from("colmap"),
            ns_process_data_command: PathBuf::from("ns-process-data"),
            ns_train_command: PathBuf::from("ns-train"),
            ns_export_command: PathBuf::from("ns-export"),
            extra_colmap_args: Vec::new(),
            extra_train_args: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VideoToRadianceResult {
    pub frames_dir: PathBuf,
    pub colmap_dir: PathBuf,
    pub nerfstudio_dir: PathBuf,
    pub export_dir: Option<PathBuf>,
    pub splat_ply: Option<PathBuf>,
    pub view_set: Option<CameraViewSet>,
    pub gaussian_stats: Option<GaussianSceneStats>,
    pub completed: Vec<String>,
    pub skipped: Vec<String>,
}

pub struct VideoToRadiancePipeline;

impl VideoToRadiancePipeline {
    pub fn expected_layout(request: &VideoToRadianceRequest) -> VideoToRadianceResult {
        let export_dir = request.work_dir.join("export");
        VideoToRadianceResult {
            frames_dir: request.work_dir.join("frames"),
            colmap_dir: request.work_dir.join("colmap"),
            nerfstudio_dir: request.work_dir.join("nerfstudio"),
            export_dir: Some(export_dir.clone()),
            splat_ply: Some(export_dir.join("splat.ply")),
            view_set: None,
            gaussian_stats: None,
            completed: Vec::new(),
            skipped: Vec::new(),
        }
    }

    pub fn build_frame_extraction_args(request: &VideoToRadianceRequest) -> Result<Vec<OsString>> {
        request.validate()?;
        let frames_dir = Self::expected_layout(request).frames_dir;
        let mut args = vec![
            OsString::from("-y"),
            OsString::from("-i"),
            request.input.as_os_str().to_os_string(),
            OsString::from("-vf"),
            OsString::from(format!(
                "select='not(mod(n\\,{}))'",
                request.frame_sample_every
            )),
            OsString::from("-vsync"),
            OsString::from("vfr"),
        ];
        if let Some(max_frames) = request.max_frames {
            args.push(OsString::from("-frames:v"));
            args.push(OsString::from(max_frames.to_string()));
        }
        args.push(frames_dir.join("frame_%06d.png").as_os_str().to_os_string());
        Ok(args)
    }

    pub fn build_colmap_args(request: &VideoToRadianceRequest) -> Result<Vec<Vec<OsString>>> {
        request.validate()?;
        let layout = Self::expected_layout(request);
        let database = layout.colmap_dir.join("database.db");
        let sparse = layout.colmap_dir.join("sparse");
        let mut feature = vec![
            OsString::from("feature_extractor"),
            OsString::from("--database_path"),
            database.as_os_str().to_os_string(),
            OsString::from("--image_path"),
            layout.frames_dir.as_os_str().to_os_string(),
        ];
        feature.extend(request.extra_colmap_args.iter().cloned());
        Ok(vec![
            feature,
            vec![
                OsString::from("exhaustive_matcher"),
                OsString::from("--database_path"),
                database.as_os_str().to_os_string(),
            ],
            vec![
                OsString::from("mapper"),
                OsString::from("--database_path"),
                database.as_os_str().to_os_string(),
                OsString::from("--image_path"),
                layout.frames_dir.as_os_str().to_os_string(),
                OsString::from("--output_path"),
                sparse.as_os_str().to_os_string(),
            ],
            vec![
                OsString::from("model_converter"),
                OsString::from("--input_path"),
                sparse.join("0").as_os_str().to_os_string(),
                OsString::from("--output_path"),
                sparse.join("0").as_os_str().to_os_string(),
                OsString::from("--output_type"),
                OsString::from("TXT"),
            ],
        ])
    }

    pub fn build_ns_process_data_args(request: &VideoToRadianceRequest) -> Result<Vec<OsString>> {
        request.validate()?;
        let layout = Self::expected_layout(request);
        Ok(vec![
            OsString::from("images"),
            OsString::from("--data"),
            layout.frames_dir.as_os_str().to_os_string(),
            OsString::from("--output-dir"),
            layout.nerfstudio_dir.as_os_str().to_os_string(),
        ])
    }

    pub fn build_ns_train_args(request: &VideoToRadianceRequest) -> Result<Vec<OsString>> {
        request.validate()?;
        let layout = Self::expected_layout(request);
        let mut args = vec![
            OsString::from(request.method.as_str()),
            OsString::from("--data"),
            layout.nerfstudio_dir.as_os_str().to_os_string(),
            OsString::from("--output-dir"),
            request.work_dir.join("training").as_os_str().to_os_string(),
        ];
        args.extend(request.extra_train_args.iter().cloned());
        Ok(args)
    }

    pub fn build_ns_export_args(request: &VideoToRadianceRequest) -> Result<Vec<OsString>> {
        request.validate()?;
        let layout = Self::expected_layout(request);
        Ok(vec![
            OsString::from("gaussian-splat"),
            OsString::from("--load-config"),
            request
                .work_dir
                .join("training")
                .join(request.method.as_str())
                .join("config.yml")
                .as_os_str()
                .to_os_string(),
            OsString::from("--output-dir"),
            layout
                .export_dir
                .expect("expected layout has export dir")
                .as_os_str()
                .to_os_string(),
        ])
    }

    pub fn run(request: VideoToRadianceRequest) -> Result<VideoToRadianceResult> {
        request.validate()?;
        let mut result = Self::expected_layout(&request);
        fs::create_dir_all(&request.work_dir)?;
        fs::create_dir_all(&result.frames_dir)?;
        fs::create_dir_all(&result.colmap_dir)?;
        fs::create_dir_all(result.colmap_dir.join("sparse"))?;
        fs::create_dir_all(&result.nerfstudio_dir)?;
        if let Some(export_dir) = &result.export_dir {
            fs::create_dir_all(export_dir)?;
        }

        run_command(
            &request.ffmpeg_command,
            Self::build_frame_extraction_args(&request)?,
        )?;
        result.completed.push("frame_extraction".to_string());

        for args in Self::build_colmap_args(&request)? {
            run_command(&request.colmap_command, args)?;
        }
        result.completed.push("colmap".to_string());

        let colmap_text_dir = result.colmap_dir.join("sparse").join("0");
        match read_colmap_text_dir(&colmap_text_dir)
            .and_then(|dataset| colmap_to_view_set(&dataset))
        {
            Ok(view_set) => result.view_set = Some(view_set),
            Err(error) => result.skipped.push(format!("COLMAP text import: {error}")),
        }

        run_command(
            &request.ns_process_data_command,
            Self::build_ns_process_data_args(&request)?,
        )?;
        result.completed.push("nerfstudio_process_data".to_string());

        if request.run_training {
            run_command(
                &request.ns_train_command,
                Self::build_ns_train_args(&request)?,
            )?;
            result.completed.push("nerfstudio_training".to_string());
            run_command(
                &request.ns_export_command,
                Self::build_ns_export_args(&request)?,
            )?;
            result.completed.push("nerfstudio_export".to_string());
            if let Some(path) = &result.splat_ply {
                match read_gaussian_splat_ply(path).and_then(|scene| Ok(scene.stats()?)) {
                    Ok(stats) => result.gaussian_stats = Some(stats),
                    Err(error) => result.skipped.push(format!("splat import: {error}")),
                }
            }
        } else {
            result
                .skipped
                .push("training/export: pass --run-training".to_string());
        }

        Ok(result)
    }
}

fn run_command(command: &Path, args: Vec<OsString>) -> Result<()> {
    let status = Command::new(command)
        .args(args)
        .stdin(Stdio::null())
        .status()
        .map_err(|err| {
            DetectError::Source(format!(
                "failed to start radiance command `{}`: {err}",
                command.display()
            ))
        })?;
    if !status.success() {
        return Err(DetectError::Source(format!(
            "radiance command `{}` exited with status {status}",
            command.display()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> VideoToRadianceRequest {
        VideoToRadianceRequest {
            input: PathBuf::from("input.mp4"),
            work_dir: PathBuf::from("work"),
            ..VideoToRadianceRequest::default()
        }
    }

    fn strings(values: Vec<OsString>) -> Vec<String> {
        values
            .into_iter()
            .map(|value| value.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn expected_layout_uses_stable_paths() {
        let layout = VideoToRadiancePipeline::expected_layout(&request());

        assert_eq!(layout.frames_dir, PathBuf::from("work/frames"));
        assert_eq!(layout.colmap_dir, PathBuf::from("work/colmap"));
        assert_eq!(layout.nerfstudio_dir, PathBuf::from("work/nerfstudio"));
        assert_eq!(
            layout.splat_ply,
            Some(PathBuf::from("work/export/splat.ply"))
        );
    }

    #[test]
    fn builds_frame_extraction_args() {
        let mut request = request();
        request.frame_sample_every = 12;
        request.max_frames = Some(20);

        let args = strings(VideoToRadiancePipeline::build_frame_extraction_args(&request).unwrap());

        assert_eq!(args[0], "-y");
        assert!(args.contains(&"input.mp4".to_string()));
        assert!(args.contains(&"select='not(mod(n\\,12))'".to_string()));
        assert!(args.contains(&"-frames:v".to_string()));
        assert_eq!(args.last().unwrap(), "work/frames/frame_%06d.png");
    }

    #[test]
    fn builds_colmap_args() {
        let args = VideoToRadiancePipeline::build_colmap_args(&request()).unwrap();
        let commands = args
            .into_iter()
            .map(|args| args[0].to_string_lossy().into_owned())
            .collect::<Vec<_>>();

        assert_eq!(
            commands,
            vec![
                "feature_extractor",
                "exhaustive_matcher",
                "mapper",
                "model_converter"
            ]
        );
    }

    #[test]
    fn builds_nerfstudio_args() {
        let mut request = request();
        request.method = RadianceTrainingMethod::Nerfacto;

        let process =
            strings(VideoToRadiancePipeline::build_ns_process_data_args(&request).unwrap());
        let train = strings(VideoToRadiancePipeline::build_ns_train_args(&request).unwrap());
        let export = strings(VideoToRadiancePipeline::build_ns_export_args(&request).unwrap());

        assert_eq!(process[0], "images");
        assert_eq!(train[0], "nerfacto");
        assert_eq!(export[0], "gaussian-splat");
    }
}
