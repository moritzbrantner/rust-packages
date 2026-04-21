use std::path::{Path, PathBuf};
use std::process::Command;

use video_analysis_core::{DetectError, Result, Scene};

pub const DEFAULT_TEMPLATE: &str = "$VIDEO_NAME-Scene-$SCENE_NUMBER.mp4";

#[derive(Debug, Clone)]
pub struct SplitOptions {
    pub output_dir: PathBuf,
    pub template: String,
    pub video_name: Option<String>,
    pub ffmpeg_args: Vec<String>,
}

impl Default for SplitOptions {
    fn default() -> Self {
        Self {
            output_dir: PathBuf::from("."),
            template: DEFAULT_TEMPLATE.to_string(),
            video_name: None,
            ffmpeg_args: vec![
                "-map".to_string(),
                "0:v:0".to_string(),
                "-map".to_string(),
                "0:a?".to_string(),
                "-map".to_string(),
                "0:s?".to_string(),
                "-c:v".to_string(),
                "libx264".to_string(),
                "-preset".to_string(),
                "veryfast".to_string(),
                "-crf".to_string(),
                "22".to_string(),
                "-c:a".to_string(),
                "aac".to_string(),
            ],
        }
    }
}

pub fn split_video_ffmpeg(
    input_video_path: impl AsRef<Path>,
    scenes: &[Scene],
    options: &SplitOptions,
) -> Result<Vec<PathBuf>> {
    let input_video_path = input_video_path.as_ref();
    if scenes.is_empty() {
        return Ok(Vec::new());
    }
    std::fs::create_dir_all(&options.output_dir)?;
    let video_name = options.video_name.clone().unwrap_or_else(|| {
        input_video_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("video")
            .to_string()
    });
    let digits = scenes.len().to_string().len().max(3);
    let mut outputs = Vec::new();
    for (index, scene) in scenes.iter().enumerate() {
        let scene_number = format!("{:0digits$}", index + 1);
        let output_name = options
            .template
            .replace("$VIDEO_NAME", &video_name)
            .replace("$SCENE_NUMBER", &scene_number)
            .replace("$START_FRAME", &scene.start.frame_index.to_string())
            .replace("$END_FRAME", &scene.end.frame_index.to_string());
        let output_path = options.output_dir.join(output_name);
        let start = scene.start.timestamp.seconds().to_string();
        let duration = (scene.end.timestamp.seconds() - scene.start.timestamp.seconds())
            .max(0.0)
            .to_string();
        let status = Command::new("ffmpeg")
            .arg("-y")
            .arg("-v")
            .arg("error")
            .arg("-ss")
            .arg(start)
            .arg("-i")
            .arg(input_video_path)
            .arg("-t")
            .arg(duration)
            .args(&options.ffmpeg_args)
            .arg(&output_path)
            .status()?;
        if !status.success() {
            return Err(DetectError::Source(format!(
                "ffmpeg failed while writing `{}`",
                output_path.display()
            )));
        }
        outputs.push(output_path);
    }
    Ok(outputs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_template_is_pyscenedetect_like() {
        assert_eq!(DEFAULT_TEMPLATE, "$VIDEO_NAME-Scene-$SCENE_NUMBER.mp4");
    }
}
