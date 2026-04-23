use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, Stdio};

use num_rational::Rational64;
use thiserror::Error;
use video_analysis_core::{
    AudioBuffer, AudioSampleFormat, DetectError, FramePosition, OwnedAudioFrame, OwnedVideoFrame,
    PixelFormat, Result, Timebase, Timestamp, VideoSource,
};
use video_analysis_ingest::{
    AudioFrameSource, AudioStreamInfo, MediaSample, MediaSource, MediaSourceInfo, SourceMode,
    VideoFrameSource, VideoStreamInfo,
};

#[derive(Debug, Error)]
pub enum FfmpegError {
    #[error("ffprobe failed for `{input}`: {message}")]
    ProbeFailed { input: String, message: String },
    #[error("ffmpeg failed to start for `{input}`: {message}")]
    StartFailed { input: String, message: String },
    #[error("missing or invalid video metadata: {0}")]
    InvalidMetadata(String),
}

#[derive(Debug, Clone)]
pub struct VideoMetadata {
    pub input: String,
    pub path: Option<PathBuf>,
    pub mode: SourceMode,
    pub width: u32,
    pub height: u32,
    pub frame_rate: Rational64,
    pub duration_seconds: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct AudioMetadata {
    pub input: String,
    pub path: Option<PathBuf>,
    pub mode: SourceMode,
    pub sample_rate: u32,
    pub channels: u16,
    pub duration_seconds: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct FfmpegSourceOptions {
    pub mode: SourceMode,
    pub realtime: bool,
    pub extra_input_args: Vec<String>,
}

impl FfmpegSourceOptions {
    pub fn recorded() -> Self {
        Self {
            mode: SourceMode::Recorded,
            realtime: false,
            extra_input_args: Vec::new(),
        }
    }

    pub fn live() -> Self {
        Self {
            mode: SourceMode::Live,
            realtime: true,
            extra_input_args: Vec::new(),
        }
    }

    pub fn extra_input_arg(mut self, arg: impl Into<String>) -> Self {
        self.extra_input_args.push(arg.into());
        self
    }
}

#[derive(Debug, Clone)]
pub struct FfmpegAudioSourceOptions {
    pub mode: SourceMode,
    pub realtime: bool,
    pub samples_per_chunk: usize,
    pub extra_input_args: Vec<String>,
}

impl FfmpegAudioSourceOptions {
    pub fn recorded() -> Self {
        Self {
            mode: SourceMode::Recorded,
            realtime: false,
            samples_per_chunk: 1024,
            extra_input_args: Vec::new(),
        }
    }

    pub fn live() -> Self {
        Self {
            mode: SourceMode::Live,
            realtime: true,
            samples_per_chunk: 1024,
            extra_input_args: Vec::new(),
        }
    }

    pub fn samples_per_chunk(mut self, samples: usize) -> Self {
        self.samples_per_chunk = samples.max(1);
        self
    }

    pub fn extra_input_arg(mut self, arg: impl Into<String>) -> Self {
        self.extra_input_args.push(arg.into());
        self
    }
}

pub struct FfmpegVideoSource {
    metadata: VideoMetadata,
    source_info: MediaSourceInfo,
    child: Child,
    stdout: ChildStdout,
    next_frame_index: u64,
    frame_size: usize,
}

impl FfmpegVideoSource {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::open_path_with_options(path, FfmpegSourceOptions::recorded())
    }

    pub fn open_path_with_options(
        path: impl AsRef<Path>,
        options: FfmpegSourceOptions,
    ) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let input = path.to_string_lossy().into_owned();
        let metadata = probe_path_with_mode(&path, options.mode)
            .map_err(|err| DetectError::Source(err.to_string()))?;
        Self::spawn(input, metadata, options)
    }

    pub fn open_input(input: impl Into<String>) -> Result<Self> {
        Self::open_input_with_options(input, FfmpegSourceOptions::recorded())
    }

    pub fn open_live(input: impl Into<String>) -> Result<Self> {
        Self::open_input_with_options(input, FfmpegSourceOptions::live())
    }

    pub fn open_input_with_options(
        input: impl Into<String>,
        options: FfmpegSourceOptions,
    ) -> Result<Self> {
        let input = input.into();
        let metadata = probe_input_with_mode(&input, None, options.mode)
            .map_err(|err| DetectError::Source(err.to_string()))?;
        Self::spawn(input, metadata, options)
    }

    fn spawn(input: String, metadata: VideoMetadata, options: FfmpegSourceOptions) -> Result<Self> {
        let source_info = MediaSourceInfo {
            input: metadata.input.clone(),
            mode: metadata.mode,
            video: Some(VideoStreamInfo {
                width: metadata.width,
                height: metadata.height,
                frame_rate: Some(metadata.frame_rate),
                pixel_format: PixelFormat::Rgb24,
            }),
            audio: Vec::new(),
            text: Vec::new(),
        };
        let mut command = Command::new("ffmpeg");
        command.arg("-v").arg("error");
        if options.realtime {
            command
                .arg("-fflags")
                .arg("nobuffer")
                .arg("-flags")
                .arg("low_delay");
        }
        for arg in &options.extra_input_args {
            command.arg(arg);
        }
        command
            .arg("-i")
            .arg(&input)
            .arg("-map")
            .arg("0:v:0")
            .arg("-an")
            .arg("-f")
            .arg("rawvideo")
            .arg("-pix_fmt")
            .arg("rgb24")
            .arg("-vsync")
            .arg("0")
            .arg("pipe:1")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = command.spawn().map_err(|err| {
            DetectError::Source(
                FfmpegError::StartFailed {
                    input: input.clone(),
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
            source_info,
            child,
            stdout,
            next_frame_index: 0,
            frame_size,
        })
    }

    pub fn metadata(&self) -> &VideoMetadata {
        &self.metadata
    }

    pub fn source_info(&self) -> &MediaSourceInfo {
        &self.source_info
    }

    fn read_next_frame(&mut self) -> Result<Option<OwnedVideoFrame>> {
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
}

impl Drop for FfmpegVideoSource {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl VideoSource for FfmpegVideoSource {
    fn next_frame(&mut self) -> Result<Option<OwnedVideoFrame>> {
        self.read_next_frame()
    }

    fn frame_rate(&self) -> Rational64 {
        self.metadata.frame_rate
    }
}

impl VideoFrameSource for FfmpegVideoSource {
    fn source_info(&self) -> &MediaSourceInfo {
        &self.source_info
    }

    fn next_video_frame(&mut self) -> Result<Option<OwnedVideoFrame>> {
        self.read_next_frame()
    }
}

impl MediaSource for FfmpegVideoSource {
    fn source_info(&self) -> &MediaSourceInfo {
        &self.source_info
    }

    fn next_sample(&mut self) -> Result<Option<MediaSample>> {
        self.read_next_frame()
            .map(|frame| frame.map(MediaSample::Video))
    }
}

pub struct FfmpegAudioSource {
    metadata: AudioMetadata,
    source_info: MediaSourceInfo,
    child: Child,
    stdout: ChildStdout,
    next_sample_index: u64,
    samples_per_chunk: usize,
}

impl FfmpegAudioSource {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::open_path_with_options(path, FfmpegAudioSourceOptions::recorded())
    }

    pub fn open_path_with_options(
        path: impl AsRef<Path>,
        options: FfmpegAudioSourceOptions,
    ) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let input = path.to_string_lossy().into_owned();
        let metadata = probe_audio_path_with_mode(&path, options.mode)
            .map_err(|err| DetectError::Source(err.to_string()))?;
        Self::spawn(input, metadata, options)
    }

    pub fn open_input(input: impl Into<String>) -> Result<Self> {
        Self::open_input_with_options(input, FfmpegAudioSourceOptions::recorded())
    }

    pub fn open_live(input: impl Into<String>) -> Result<Self> {
        Self::open_input_with_options(input, FfmpegAudioSourceOptions::live())
    }

    pub fn open_input_with_options(
        input: impl Into<String>,
        options: FfmpegAudioSourceOptions,
    ) -> Result<Self> {
        let input = input.into();
        let metadata = probe_audio_input_with_mode(&input, None, options.mode)
            .map_err(|err| DetectError::Source(err.to_string()))?;
        Self::spawn(input, metadata, options)
    }

    fn spawn(
        input: String,
        metadata: AudioMetadata,
        options: FfmpegAudioSourceOptions,
    ) -> Result<Self> {
        let source_info = MediaSourceInfo {
            input: metadata.input.clone(),
            mode: metadata.mode,
            video: None,
            audio: vec![AudioStreamInfo {
                sample_rate: metadata.sample_rate,
                channels: metadata.channels,
                sample_format: AudioSampleFormat::F32,
            }],
            text: Vec::new(),
        };
        let mut command = Command::new("ffmpeg");
        command.arg("-v").arg("error");
        if options.realtime {
            command
                .arg("-fflags")
                .arg("nobuffer")
                .arg("-flags")
                .arg("low_delay");
        }
        for arg in &options.extra_input_args {
            command.arg(arg);
        }
        command
            .arg("-i")
            .arg(&input)
            .arg("-map")
            .arg("0:a:0")
            .arg("-vn")
            .arg("-f")
            .arg("f32le")
            .arg("-acodec")
            .arg("pcm_f32le")
            .arg("pipe:1")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = command.spawn().map_err(|err| {
            DetectError::Source(
                FfmpegError::StartFailed {
                    input: input.clone(),
                    message: err.to_string(),
                }
                .to_string(),
            )
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            DetectError::Source("ffmpeg stdout pipe was not available".to_string())
        })?;
        Ok(Self {
            metadata,
            source_info,
            child,
            stdout,
            next_sample_index: 0,
            samples_per_chunk: options.samples_per_chunk.max(1),
        })
    }

    pub fn metadata(&self) -> &AudioMetadata {
        &self.metadata
    }

    pub fn source_info(&self) -> &MediaSourceInfo {
        &self.source_info
    }

    fn read_next_audio_frame(&mut self) -> Result<Option<OwnedAudioFrame>> {
        let bytes_per_sample = std::mem::size_of::<f32>();
        let bytes_per_sample_frame = self.metadata.channels as usize * bytes_per_sample;
        let target_bytes = self.samples_per_chunk * bytes_per_sample_frame;
        let mut bytes = vec![0_u8; target_bytes];
        let mut offset = 0;
        while offset < target_bytes {
            match self.stdout.read(&mut bytes[offset..]) {
                Ok(0) if offset == 0 => return Ok(None),
                Ok(0) => break,
                Ok(read) => offset += read,
                Err(err) if err.kind() == std::io::ErrorKind::Interrupted => {}
                Err(err) => return Err(DetectError::Io(err)),
            }
        }
        bytes.truncate(offset - (offset % bytes_per_sample_frame));
        if bytes.is_empty() {
            return Ok(None);
        }
        let samples = bytes
            .chunks_exact(4)
            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect::<Vec<_>>();
        let timestamp = Timestamp::new(
            self.next_sample_index as i64,
            Timebase::new(1, self.metadata.sample_rate as i32),
        );
        let samples_per_channel = samples.len() / self.metadata.channels as usize;
        self.next_sample_index += samples_per_channel as u64;
        OwnedAudioFrame::new(
            timestamp,
            self.metadata.sample_rate,
            self.metadata.channels,
            AudioBuffer::F32(samples),
        )
        .map(Some)
    }
}

impl Drop for FfmpegAudioSource {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl AudioFrameSource for FfmpegAudioSource {
    fn source_info(&self) -> &MediaSourceInfo {
        &self.source_info
    }

    fn next_audio_frame(&mut self) -> Result<Option<OwnedAudioFrame>> {
        self.read_next_audio_frame()
    }
}

impl MediaSource for FfmpegAudioSource {
    fn source_info(&self) -> &MediaSourceInfo {
        &self.source_info
    }

    fn next_sample(&mut self) -> Result<Option<MediaSample>> {
        self.read_next_audio_frame()
            .map(|frame| frame.map(MediaSample::Audio))
    }
}

pub fn probe(path: impl AsRef<Path>) -> std::result::Result<VideoMetadata, FfmpegError> {
    let path = path.as_ref().to_path_buf();
    probe_path_with_mode(path, SourceMode::Recorded)
}

pub fn probe_input(input: impl AsRef<str>) -> std::result::Result<VideoMetadata, FfmpegError> {
    probe_input_with_mode(input.as_ref(), None, SourceMode::Recorded)
}

pub fn probe_audio(path: impl AsRef<Path>) -> std::result::Result<AudioMetadata, FfmpegError> {
    let path = path.as_ref().to_path_buf();
    probe_audio_path_with_mode(path, SourceMode::Recorded)
}

pub fn probe_audio_input(
    input: impl AsRef<str>,
) -> std::result::Result<AudioMetadata, FfmpegError> {
    probe_audio_input_with_mode(input.as_ref(), None, SourceMode::Recorded)
}

fn probe_path_with_mode(
    path: impl AsRef<Path>,
    mode: SourceMode,
) -> std::result::Result<VideoMetadata, FfmpegError> {
    let path = path.as_ref().to_path_buf();
    let input = path.to_string_lossy().into_owned();
    probe_input_with_mode(&input, Some(path), mode)
}

fn probe_audio_path_with_mode(
    path: impl AsRef<Path>,
    mode: SourceMode,
) -> std::result::Result<AudioMetadata, FfmpegError> {
    let path = path.as_ref().to_path_buf();
    let input = path.to_string_lossy().into_owned();
    probe_audio_input_with_mode(&input, Some(path), mode)
}

fn probe_audio_input_with_mode(
    input: &str,
    path: Option<PathBuf>,
    mode: SourceMode,
) -> std::result::Result<AudioMetadata, FfmpegError> {
    let output = Command::new("ffprobe")
        .arg("-v")
        .arg("error")
        .arg("-select_streams")
        .arg("a:0")
        .arg("-show_entries")
        .arg("stream=sample_rate,channels,duration")
        .arg("-of")
        .arg("default=noprint_wrappers=1:nokey=1")
        .arg(input)
        .output()
        .map_err(|err| FfmpegError::ProbeFailed {
            input: input.to_string(),
            message: err.to_string(),
        })?;
    if !output.status.success() {
        return Err(FfmpegError::ProbeFailed {
            input: input.to_string(),
            message: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut lines = text.lines();
    let sample_rate = parse_u32(lines.next(), "sample_rate")?;
    let channels = parse_u16(lines.next(), "channels")?;
    if sample_rate == 0 || channels == 0 {
        return Err(FfmpegError::InvalidMetadata(
            "audio sample_rate and channels must be positive".to_string(),
        ));
    }
    let duration_seconds = lines.next().and_then(|value| value.parse::<f64>().ok());
    Ok(AudioMetadata {
        input: input.to_string(),
        path,
        mode,
        sample_rate,
        channels,
        duration_seconds,
    })
}

fn probe_input_with_mode(
    input: &str,
    path: Option<PathBuf>,
    mode: SourceMode,
) -> std::result::Result<VideoMetadata, FfmpegError> {
    let output = Command::new("ffprobe")
        .arg("-v")
        .arg("error")
        .arg("-select_streams")
        .arg("v:0")
        .arg("-show_entries")
        .arg("stream=width,height,avg_frame_rate,duration")
        .arg("-of")
        .arg("default=noprint_wrappers=1:nokey=1")
        .arg(input)
        .output()
        .map_err(|err| FfmpegError::ProbeFailed {
            input: input.to_string(),
            message: err.to_string(),
        })?;
    if !output.status.success() {
        return Err(FfmpegError::ProbeFailed {
            input: input.to_string(),
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
        input: input.to_string(),
        path,
        mode,
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

pub fn extract_wav(
    media_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
    sample_rate: u32,
) -> Result<PathBuf> {
    if sample_rate == 0 {
        return Err(DetectError::InvalidArgument(
            "sample_rate must be positive".to_string(),
        ));
    }
    let media_path = media_path.as_ref();
    let output_path = output_path.as_ref();
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let status = Command::new("ffmpeg")
        .arg("-y")
        .arg("-v")
        .arg("error")
        .arg("-i")
        .arg(media_path)
        .arg("-vn")
        .arg("-ac")
        .arg("1")
        .arg("-ar")
        .arg(sample_rate.to_string())
        .arg("-f")
        .arg("wav")
        .arg(output_path)
        .status()?;
    if !status.success() {
        return Err(DetectError::Source(format!(
            "ffmpeg failed to extract {sample_rate} Hz WAV audio"
        )));
    }
    Ok(output_path.to_path_buf())
}

#[cfg(feature = "test-utils")]
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

#[cfg(feature = "test-utils")]
pub fn write_test_audio(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    let status = Command::new("ffmpeg")
        .arg("-y")
        .arg("-v")
        .arg("error")
        .arg("-f")
        .arg("lavfi")
        .arg("-i")
        .arg("sine=frequency=1000:duration=0.1:sample_rate=48000")
        .arg("-ac")
        .arg("1")
        .arg(path)
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(DetectError::Source(
            "ffmpeg failed to generate test audio".to_string(),
        ))
    }
}

fn parse_u32(value: Option<&str>, name: &str) -> std::result::Result<u32, FfmpegError> {
    value
        .ok_or_else(|| FfmpegError::InvalidMetadata(format!("missing {name}")))?
        .parse::<u32>()
        .map_err(|err| FfmpegError::InvalidMetadata(format!("invalid {name}: {err}")))
}

fn parse_u16(value: Option<&str>, name: &str) -> std::result::Result<u16, FfmpegError> {
    value
        .ok_or_else(|| FfmpegError::InvalidMetadata(format!("missing {name}")))?
        .parse::<u16>()
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

    #[test]
    #[cfg(feature = "ffmpeg-tests")]
    fn decodes_generated_audio() {
        use video_analysis_ingest::AudioFrameSource;

        use super::*;

        if !(is_ffmpeg_available() && is_ffprobe_available()) {
            eprintln!("skipping FFmpeg test because ffmpeg/ffprobe is unavailable");
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tone.wav");
        write_test_audio(&path).unwrap();
        let mut source = FfmpegAudioSource::open_path_with_options(
            &path,
            FfmpegAudioSourceOptions::recorded().samples_per_chunk(512),
        )
        .unwrap();
        assert_eq!(source.metadata().sample_rate, 48_000);
        assert_eq!(source.metadata().channels, 1);
        let mut count = 0;
        while let Some(frame) = source.next_audio_frame().unwrap() {
            assert_eq!(frame.sample_rate, 48_000);
            count += 1;
        }
        assert!(count > 0, "expected decoded audio frames");
    }
}
