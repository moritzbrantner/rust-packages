use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, Stdio};

use num_rational::Rational64;
use thiserror::Error;
use video_analysis_core::{
    DetectError, FramePosition, OwnedVideoFrame, PixelFormat, Result, VideoSource,
};

#[derive(Debug, Error)]
pub enum FfmpegError {
    #[error("ffprobe failed for `{path}`: {message}")]
    ProbeFailed { path: PathBuf, message: String },
    #[error("ffmpeg failed to start for `{path}`: {message}")]
    StartFailed { path: PathBuf, message: String },
    #[error("missing or invalid video metadata: {0}")]
    InvalidMetadata(String),
}

#[derive(Debug, Clone)]
pub struct VideoMetadata {
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub frame_rate: Rational64,
    pub duration_seconds: Option<f64>,
}

pub struct FfmpegVideoSource {
    metadata: VideoMetadata,
    child: Child,
    stdout: ChildStdout,
    next_frame_index: u64,
    frame_size: usize,
}

impl FfmpegVideoSource {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let metadata = probe(&path).map_err(|err| DetectError::Source(err.to_string()))?;
        let mut child = Command::new("ffmpeg")
            .arg("-v")
            .arg("error")
            .arg("-i")
            .arg(&path)
            .arg("-map")
            .arg("0:v:0")
            .arg("-f")
            .arg("rawvideo")
            .arg("-pix_fmt")
            .arg("rgb24")
            .arg("-vsync")
            .arg("0")
            .arg("pipe:1")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|err| {
                DetectError::Source(
                    FfmpegError::StartFailed {
                        path: path.clone(),
                        message: err.to_string(),
                    }
                    .to_string(),
                )
            })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            DetectError::Source("ffmpeg stdout pipe was not available".to_string())
        })?;
        let frame_size = metadata.width as usize * metadata.height as usize * 3;
        Ok(Self {
            metadata,
            child,
            stdout,
            next_frame_index: 0,
            frame_size,
        })
    }

    pub fn metadata(&self) -> &VideoMetadata {
        &self.metadata
    }
}

impl Drop for FfmpegVideoSource {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl VideoSource for FfmpegVideoSource {
    fn next_frame(&mut self) -> Result<Option<OwnedVideoFrame>> {
        let mut data = vec![0_u8; self.frame_size];
        let mut offset = 0;
        while offset < self.frame_size {
            match self.stdout.read(&mut data[offset..]) {
                Ok(0) if offset == 0 => return Ok(None),
                Ok(0) => {
                    return Err(DetectError::Source(
                        "ffmpeg ended in the middle of a raw frame".to_string(),
                    ))
                }
                Ok(bytes) => offset += bytes,
                Err(err) if err.kind() == std::io::ErrorKind::Interrupted => {}
                Err(err) => return Err(DetectError::Io(err)),
            }
        }
        let position =
            FramePosition::from_frame_index(self.next_frame_index, self.metadata.frame_rate);
        self.next_frame_index += 1;
        Ok(Some(OwnedVideoFrame {
            position,
            width: self.metadata.width,
            height: self.metadata.height,
            pixel_format: PixelFormat::Rgb24,
            data,
            stride: self.metadata.width as usize * 3,
        }))
    }

    fn frame_rate(&self) -> Rational64 {
        self.metadata.frame_rate
    }
}

pub fn probe(path: impl AsRef<Path>) -> std::result::Result<VideoMetadata, FfmpegError> {
    let path = path.as_ref().to_path_buf();
    let output = Command::new("ffprobe")
        .arg("-v")
        .arg("error")
        .arg("-select_streams")
        .arg("v:0")
        .arg("-show_entries")
        .arg("stream=width,height,avg_frame_rate,duration")
        .arg("-of")
        .arg("default=noprint_wrappers=1:nokey=1")
        .arg(&path)
        .output()
        .map_err(|err| FfmpegError::ProbeFailed {
            path: path.clone(),
            message: err.to_string(),
        })?;
    if !output.status.success() {
        return Err(FfmpegError::ProbeFailed {
            path,
            message: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut lines = text.lines();
    let width = parse_u32(lines.next(), "width")?;
    let height = parse_u32(lines.next(), "height")?;
    let frame_rate = parse_rate(lines.next().unwrap_or_default())?;
    let duration_seconds = lines.next().and_then(|value| value.parse::<f64>().ok());
    Ok(VideoMetadata {
        path,
        width,
        height,
        frame_rate,
        duration_seconds,
    })
}

pub fn is_ffmpeg_available() -> bool {
    Command::new("ffmpeg")
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub fn is_ffprobe_available() -> bool {
    Command::new("ffprobe")
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub fn write_two_scene_test_video(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    let status = Command::new("ffmpeg")
        .arg("-y")
        .arg("-v")
        .arg("error")
        .arg("-f")
        .arg("lavfi")
        .arg("-i")
        .arg("color=c=black:s=64x64:d=0.5:r=10")
        .arg("-f")
        .arg("lavfi")
        .arg("-i")
        .arg("color=c=white:s=64x64:d=0.5:r=10")
        .arg("-filter_complex")
        .arg("[0:v][1:v]concat=n=2:v=1:a=0")
        .arg("-pix_fmt")
        .arg("yuv420p")
        .arg(path)
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(DetectError::Source(
            "ffmpeg failed to generate test video".to_string(),
        ))
    }
}

fn parse_u32(value: Option<&str>, name: &str) -> std::result::Result<u32, FfmpegError> {
    value
        .ok_or_else(|| FfmpegError::InvalidMetadata(format!("missing {name}")))?
        .parse::<u32>()
        .map_err(|err| FfmpegError::InvalidMetadata(format!("invalid {name}: {err}")))
}

fn parse_rate(value: &str) -> std::result::Result<Rational64, FfmpegError> {
    let Some((num, den)) = value.split_once('/') else {
        return Err(FfmpegError::InvalidMetadata(format!(
            "invalid frame rate `{value}`"
        )));
    };
    let num = num
        .parse::<i64>()
        .map_err(|err| FfmpegError::InvalidMetadata(format!("invalid frame rate: {err}")))?;
    let den = den
        .parse::<i64>()
        .map_err(|err| FfmpegError::InvalidMetadata(format!("invalid frame rate: {err}")))?;
    if num <= 0 || den <= 0 {
        return Err(FfmpegError::InvalidMetadata(
            "frame rate must be positive".to_string(),
        ));
    }
    Ok(Rational64::new(num, den))
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(feature = "ffmpeg-tests")]
    fn decodes_generated_two_scene_video() {
        use video_analysis_core::VideoSource;

        use super::*;

        if !(is_ffmpeg_available() && is_ffprobe_available()) {
            eprintln!("skipping FFmpeg test because ffmpeg/ffprobe is unavailable");
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("two-scenes.mp4");
        write_two_scene_test_video(&path).unwrap();
        let mut source = FfmpegVideoSource::open(&path).unwrap();
        assert_eq!(source.metadata().width, 64);
        assert_eq!(source.metadata().height, 64);
        let mut count = 0;
        while source.next_frame().unwrap().is_some() {
            count += 1;
        }
        assert!(
            count >= 8,
            "expected at least 8 decoded frames, got {count}"
        );
    }
}

pub fn ensure_parent_dir(path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}

pub fn touch_text(path: &Path, text: &str) -> std::io::Result<()> {
    ensure_parent_dir(path)?;
    let mut file = std::fs::File::create(path)?;
    file.write_all(text.as_bytes())
}
