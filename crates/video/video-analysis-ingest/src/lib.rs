#![doc = include_str!("../README.md")]

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use num_rational::Rational64;
use video_analysis_core::{
    AudioAnalysis, AudioAnalysisResult, AudioPipeline, AudioSampleFormat, DetectionResult,
    FrameAnalysis, OwnedAudioFrame, OwnedTextSegment, OwnedVideoFrame, PixelFormat,
    RealtimeVideoAnalysisResult, RealtimeVideoFrameAnalysis, RealtimeVideoPipeline, Result,
    ScenePipeline, TextAnalysis, TextAnalysisResult, TextPipeline, VideoAnalysisPipeline,
    VideoAnalysisResult, VideoFrameAnalysis,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceMode {
    Recorded,
    Live,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MediaSourceInfo {
    pub input: String,
    pub mode: SourceMode,
    pub video: Option<VideoStreamInfo>,
    pub audio: Vec<AudioStreamInfo>,
    pub text: Vec<TextStreamInfo>,
}

impl MediaSourceInfo {
    pub fn recorded(input: impl Into<String>) -> Self {
        Self {
            input: input.into(),
            mode: SourceMode::Recorded,
            video: None,
            audio: Vec::new(),
            text: Vec::new(),
        }
    }

    pub fn live(input: impl Into<String>) -> Self {
        Self {
            input: input.into(),
            mode: SourceMode::Live,
            video: None,
            audio: Vec::new(),
            text: Vec::new(),
        }
    }

    pub fn with_video(mut self, video: VideoStreamInfo) -> Self {
        self.video = Some(video);
        self
    }

    pub fn with_audio(mut self, audio: AudioStreamInfo) -> Self {
        self.audio.push(audio);
        self
    }

    pub fn with_text(mut self, text: TextStreamInfo) -> Self {
        self.text.push(text);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VideoStreamInfo {
    pub width: u32,
    pub height: u32,
    pub frame_rate: Option<Rational64>,
    pub pixel_format: PixelFormat,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioStreamInfo {
    pub sample_rate: u32,
    pub channels: u16,
    pub sample_format: AudioSampleFormat,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextStreamInfo {
    pub format: TextFormat,
    pub language: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextFormat {
    Plain,
    Lines,
    Transcript,
    Subtitles,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MediaSample {
    Video(OwnedVideoFrame),
    Audio(OwnedAudioFrame),
    Text(OwnedTextSegment),
}

pub trait MediaSource {
    fn source_info(&self) -> &MediaSourceInfo;
    fn next_sample(&mut self) -> Result<Option<MediaSample>>;

    fn mode(&self) -> SourceMode {
        self.source_info().mode
    }

    fn is_live(&self) -> bool {
        self.mode() == SourceMode::Live
    }
}

pub trait VideoFrameSource {
    fn source_info(&self) -> &MediaSourceInfo;
    fn next_video_frame(&mut self) -> Result<Option<OwnedVideoFrame>>;

    fn frame_rate(&self) -> Option<Rational64> {
        self.source_info()
            .video
            .as_ref()
            .and_then(|video| video.frame_rate)
    }

    fn is_live(&self) -> bool {
        self.source_info().mode == SourceMode::Live
    }
}

pub trait AudioFrameSource {
    fn source_info(&self) -> &MediaSourceInfo;
    fn next_audio_frame(&mut self) -> Result<Option<OwnedAudioFrame>>;

    fn is_live(&self) -> bool {
        self.source_info().mode == SourceMode::Live
    }
}

pub trait TextSegmentSource {
    fn source_info(&self) -> &MediaSourceInfo;
    fn next_text_segment(&mut self) -> Result<Option<OwnedTextSegment>>;

    fn is_live(&self) -> bool {
        self.source_info().mode == SourceMode::Live
    }
}

pub fn analyze_video_source<S, F>(
    source: &mut S,
    pipeline: &mut ScenePipeline,
    mut on_frame: F,
) -> Result<DetectionResult>
where
    S: VideoFrameSource,
    F: FnMut(&FrameAnalysis) -> Result<()>,
{
    pipeline.reset();
    while let Some(frame) = source.next_video_frame()? {
        let analysis = pipeline.process_frame(frame)?;
        on_frame(&analysis)?;
    }
    pipeline.finish_detection()
}

pub fn analyze_video_frames<S, F>(
    source: &mut S,
    pipeline: &mut VideoAnalysisPipeline,
    mut on_frame: F,
) -> Result<VideoAnalysisResult>
where
    S: VideoFrameSource,
    F: FnMut(&VideoFrameAnalysis) -> Result<()>,
{
    pipeline.reset();
    while let Some(frame) = source.next_video_frame()? {
        let analysis = pipeline.process_frame(frame)?;
        on_frame(&analysis)?;
    }
    pipeline.finish_analysis()
}

pub fn analyze_realtime_video_source<S, F>(
    source: &mut S,
    pipeline: &mut RealtimeVideoPipeline,
    mut on_frame: F,
) -> Result<RealtimeVideoAnalysisResult>
where
    S: VideoFrameSource,
    F: FnMut(&RealtimeVideoFrameAnalysis) -> Result<()>,
{
    pipeline.reset();
    while let Some(frame) = source.next_video_frame()? {
        let analysis = pipeline.process_frame(frame)?;
        on_frame(&analysis)?;
    }
    pipeline.finish_analysis()
}

pub fn analyze_audio_source<S, F>(
    source: &mut S,
    pipeline: &mut AudioPipeline,
    mut on_frame: F,
) -> Result<AudioAnalysisResult>
where
    S: AudioFrameSource,
    F: FnMut(&AudioAnalysis) -> Result<()>,
{
    pipeline.reset();
    while let Some(frame) = source.next_audio_frame()? {
        let analysis = pipeline.process_frame(frame)?;
        on_frame(&analysis)?;
    }
    pipeline.finish_analysis()
}

pub fn analyze_text_source<S, F>(
    source: &mut S,
    pipeline: &mut TextPipeline,
    mut on_segment: F,
) -> Result<TextAnalysisResult>
where
    S: TextSegmentSource,
    F: FnMut(&TextAnalysis) -> Result<()>,
{
    pipeline.reset();
    while let Some(segment) = source.next_text_segment()? {
        let analysis = pipeline.process_segment(segment)?;
        on_segment(&analysis)?;
    }
    pipeline.finish_analysis()
}

pub struct TextLineSource<R> {
    source_info: MediaSourceInfo,
    reader: R,
    next_segment_index: u64,
    language: Option<String>,
}

impl<R: BufRead> TextLineSource<R> {
    pub fn recorded(input: impl Into<String>, reader: R) -> Self {
        Self::new(SourceMode::Recorded, input, reader)
    }

    pub fn live(input: impl Into<String>, reader: R) -> Self {
        Self::new(SourceMode::Live, input, reader)
    }

    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        let language = language.into();
        self.language = Some(language.clone());
        if let Some(text) = self.source_info.text.first_mut() {
            text.language = Some(language);
        }
        self
    }

    fn new(mode: SourceMode, input: impl Into<String>, reader: R) -> Self {
        let input = input.into();
        let source_info = MediaSourceInfo {
            input,
            mode,
            video: None,
            audio: Vec::new(),
            text: vec![TextStreamInfo {
                format: TextFormat::Lines,
                language: None,
            }],
        };
        Self {
            source_info,
            reader,
            next_segment_index: 0,
            language: None,
        }
    }
}

impl TextLineSource<BufReader<File>> {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let file = File::open(path)?;
        Ok(Self::recorded(
            path.to_string_lossy().into_owned(),
            BufReader::new(file),
        ))
    }
}

impl<R: BufRead> TextSegmentSource for TextLineSource<R> {
    fn source_info(&self) -> &MediaSourceInfo {
        &self.source_info
    }

    fn next_text_segment(&mut self) -> Result<Option<OwnedTextSegment>> {
        let mut line = String::new();
        let bytes = self.reader.read_line(&mut line)?;
        if bytes == 0 {
            return Ok(None);
        }
        let text = line.trim_end_matches(['\r', '\n']).to_string();
        let segment_index = self.next_segment_index;
        self.next_segment_index += 1;
        let mut segment = OwnedTextSegment::new(segment_index, text);
        if let Some(language) = &self.language {
            segment = segment.language(language.clone());
        }
        Ok(Some(segment))
    }
}

impl<R: BufRead> MediaSource for TextLineSource<R> {
    fn source_info(&self) -> &MediaSourceInfo {
        &self.source_info
    }

    fn next_sample(&mut self) -> Result<Option<MediaSample>> {
        self.next_text_segment()
            .map(|segment| segment.map(MediaSample::Text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use video_analysis_core::{AudioBuffer, FramePosition, PixelFormat, Timebase, Timestamp};

    #[test]
    fn audio_frame_reports_samples_per_channel_and_duration() {
        let frame = OwnedAudioFrame {
            timestamp: Timestamp::new(0, Timebase::new(1, 48_000)),
            sample_rate: 48_000,
            channels: 2,
            data: AudioBuffer::F32(vec![0.0; 960]),
        };

        assert_eq!(frame.sample_format(), AudioSampleFormat::F32);
        assert_eq!(frame.samples_per_channel(), 480);
        assert_eq!(frame.duration_seconds(), 0.01);
    }

    #[test]
    fn media_source_info_builders_track_mode_and_streams() {
        let info = MediaSourceInfo::live("rtsp://example/stream").with_video(VideoStreamInfo {
            width: 1920,
            height: 1080,
            frame_rate: Some(Rational64::new(30, 1)),
            pixel_format: PixelFormat::Rgb24,
        });

        assert_eq!(info.mode, SourceMode::Live);
        assert_eq!(info.video.unwrap().width, 1920);
    }

    #[test]
    fn media_sample_can_hold_video_frames() {
        let frame = OwnedVideoFrame {
            position: FramePosition::from_frame_index(0, Rational64::new(30, 1)),
            width: 1,
            height: 1,
            pixel_format: PixelFormat::Rgb24,
            data: vec![0, 0, 0],
            stride: 3,
        };

        assert!(matches!(MediaSample::Video(frame), MediaSample::Video(_)));
    }

    #[test]
    fn text_line_source_yields_segments() {
        let input = std::io::Cursor::new("first\nsecond\n");
        let mut source = TextLineSource::recorded("memory", input).with_language("en");

        let first = source.next_text_segment().unwrap().unwrap();
        let second = source.next_text_segment().unwrap().unwrap();

        assert_eq!(first.segment_index, 0);
        assert_eq!(first.text, "first");
        assert_eq!(first.language.as_deref(), Some("en"));
        assert_eq!(second.segment_index, 1);
        assert!(source.next_text_segment().unwrap().is_none());
    }
}
