use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::ops::{Add, Sub};

use num_rational::Rational64;
use num_traits::ToPrimitive;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DetectError {
    #[error("unsupported pixel format: {0:?}")]
    UnsupportedPixelFormat(PixelFormat),
    #[error("unsupported audio sample format: {0:?}")]
    UnsupportedAudioSampleFormat(AudioSampleFormat),
    #[error("invalid frame buffer: expected at least {expected} bytes, got {actual}")]
    InvalidFrameBuffer { expected: usize, actual: usize },
    #[error("invalid dimensions: {width}x{height}")]
    InvalidDimensions { width: u32, height: u32 },
    #[error("invalid audio format: sample_rate={sample_rate}, channels={channels}")]
    InvalidAudioFormat { sample_rate: u32, channels: u16 },
    #[error("video source error: {0}")]
    Source(String),
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, DetectError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timebase {
    pub num: i32,
    pub den: i32,
}

impl Timebase {
    pub const fn new(num: i32, den: i32) -> Self {
        Self { num, den }
    }

    pub fn seconds_per_tick(self) -> f64 {
        self.num as f64 / self.den as f64
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp {
    pub pts: i64,
    pub timebase: Timebase,
}

impl Timestamp {
    pub const fn new(pts: i64, timebase: Timebase) -> Self {
        Self { pts, timebase }
    }

    pub fn seconds(self) -> f64 {
        self.pts as f64 * self.timebase.seconds_per_tick()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FramePosition {
    pub frame_index: u64,
    pub timestamp: Timestamp,
}

impl FramePosition {
    pub fn from_frame_index(frame_index: u64, fps: Rational64) -> Self {
        let den = *fps.numer() as i32;
        let num = *fps.denom() as i32;
        Self {
            frame_index,
            timestamp: Timestamp::new(frame_index as i64, Timebase::new(num, den)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FrameTimecode {
    pub frame_index: u64,
    pub fps: Rational64,
}

impl FrameTimecode {
    pub fn from_frames(frame_index: u64, fps: Rational64) -> Self {
        Self { frame_index, fps }
    }

    pub fn from_seconds(seconds: f64, fps: Rational64) -> Result<Self> {
        if seconds < 0.0 {
            return Err(DetectError::InvalidArgument(
                "seconds must be greater than or equal to zero".to_string(),
            ));
        }
        let fps_float = fps.to_f64().ok_or_else(|| {
            DetectError::InvalidArgument("frame rate cannot be represented".to_string())
        })?;
        Ok(Self {
            frame_index: (seconds * fps_float).round() as u64,
            fps,
        })
    }

    pub fn parse(input: &str, fps: Rational64) -> Result<Self> {
        if input.chars().all(|c| c.is_ascii_digit()) {
            let frame_index = input.parse::<u64>().map_err(|err| {
                DetectError::InvalidArgument(format!("invalid frame number `{input}`: {err}"))
            })?;
            return Ok(Self { frame_index, fps });
        }
        if !input.contains(':') {
            let seconds = input.parse::<f64>().map_err(|err| {
                DetectError::InvalidArgument(format!("invalid seconds `{input}`: {err}"))
            })?;
            return Self::from_seconds(seconds, fps);
        }

        let parts: Vec<&str> = input.split(':').collect();
        if parts.len() != 3 {
            return Err(DetectError::InvalidArgument(format!(
                "invalid timecode `{input}`"
            )));
        }
        let hours = parts[0].parse::<f64>().map_err(|err| {
            DetectError::InvalidArgument(format!("invalid hours in `{input}`: {err}"))
        })?;
        let minutes = parts[1].parse::<f64>().map_err(|err| {
            DetectError::InvalidArgument(format!("invalid minutes in `{input}`: {err}"))
        })?;
        let seconds = parts[2].parse::<f64>().map_err(|err| {
            DetectError::InvalidArgument(format!("invalid seconds in `{input}`: {err}"))
        })?;
        Self::from_seconds((hours * 3600.0) + (minutes * 60.0) + seconds, fps)
    }

    pub fn seconds(self) -> f64 {
        self.frame_index as f64 / self.fps.to_f64().unwrap_or(1.0)
    }

    pub fn timecode(self, precision: usize) -> String {
        let factor = 10_f64.powi(precision as i32);
        let total = (self.seconds() * factor).round() / factor;
        let hours = (total / 3600.0).floor() as u64;
        let minutes = ((total - hours as f64 * 3600.0) / 60.0).floor() as u64;
        let seconds = total - hours as f64 * 3600.0 - minutes as f64 * 60.0;
        if precision == 0 {
            format!("{hours:02}:{minutes:02}:{:02}", seconds.round() as u64)
        } else {
            let whole = seconds.floor() as u64;
            let frac = ((seconds - whole as f64) * factor).round() as u64;
            format!("{hours:02}:{minutes:02}:{whole:02}.{frac:0precision$}")
        }
    }

    pub fn position(self) -> FramePosition {
        FramePosition::from_frame_index(self.frame_index, self.fps)
    }
}

impl Add<u64> for FrameTimecode {
    type Output = FrameTimecode;

    fn add(self, rhs: u64) -> Self::Output {
        Self {
            frame_index: self.frame_index + rhs,
            fps: self.fps,
        }
    }
}

impl Sub<u64> for FrameTimecode {
    type Output = FrameTimecode;

    fn sub(self, rhs: u64) -> Self::Output {
        Self {
            frame_index: self.frame_index.saturating_sub(rhs),
            fps: self.fps,
        }
    }
}

impl fmt::Display for FrameTimecode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.timecode(3))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    Rgb24,
    Bgr24,
}

#[derive(Debug, Clone, Copy)]
pub struct VideoFrame<'a> {
    pub position: FramePosition,
    pub width: u32,
    pub height: u32,
    pub pixel_format: PixelFormat,
    pub data: &'a [u8],
    pub stride: usize,
}

impl<'a> VideoFrame<'a> {
    pub fn rgb24(position: FramePosition, width: u32, height: u32, data: &'a [u8]) -> Result<Self> {
        Self::packed(
            position,
            width,
            height,
            PixelFormat::Rgb24,
            data,
            width as usize * 3,
        )
    }

    pub fn bgr24(position: FramePosition, width: u32, height: u32, data: &'a [u8]) -> Result<Self> {
        Self::packed(
            position,
            width,
            height,
            PixelFormat::Bgr24,
            data,
            width as usize * 3,
        )
    }

    pub fn packed(
        position: FramePosition,
        width: u32,
        height: u32,
        pixel_format: PixelFormat,
        data: &'a [u8],
        stride: usize,
    ) -> Result<Self> {
        if width == 0 || height == 0 {
            return Err(DetectError::InvalidDimensions { width, height });
        }
        let expected = stride * height as usize;
        if data.len() < expected {
            return Err(DetectError::InvalidFrameBuffer {
                expected,
                actual: data.len(),
            });
        }
        Ok(Self {
            position,
            width,
            height,
            pixel_format,
            data,
            stride,
        })
    }

    pub fn pixel_rgb(&self, x: u32, y: u32) -> [u8; 3] {
        let i = y as usize * self.stride + x as usize * 3;
        match self.pixel_format {
            PixelFormat::Rgb24 => [self.data[i], self.data[i + 1], self.data[i + 2]],
            PixelFormat::Bgr24 => [self.data[i + 2], self.data[i + 1], self.data[i]],
        }
    }

    pub fn pixel_count(&self) -> usize {
        self.width as usize * self.height as usize
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Scene {
    pub start: FramePosition,
    pub end: FramePosition,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Cut {
    pub position: FramePosition,
    pub detector: &'static str,
    pub score: Option<f32>,
}

pub trait MetricsSink {
    fn set_metric(&mut self, frame_index: u64, key: &'static str, value: f64);
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct MetricsStore {
    rows: BTreeMap<u64, BTreeMap<&'static str, f64>>,
    keys: BTreeSet<&'static str>,
}

impl MetricsStore {
    pub fn rows(&self) -> &BTreeMap<u64, BTreeMap<&'static str, f64>> {
        &self.rows
    }

    pub fn keys(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.keys.iter().copied()
    }

    pub fn get(&self, frame_index: u64, key: &'static str) -> Option<f64> {
        self.rows
            .get(&frame_index)
            .and_then(|row| row.get(key))
            .copied()
    }
}

impl MetricsSink for MetricsStore {
    fn set_metric(&mut self, frame_index: u64, key: &'static str, value: f64) {
        self.keys.insert(key);
        self.rows.entry(frame_index).or_default().insert(key, value);
    }
}

pub trait SceneDetector {
    fn name(&self) -> &'static str;
    fn metric_keys(&self) -> &'static [&'static str];
    fn event_buffer_len(&self) -> usize {
        0
    }
    fn process_frame(
        &mut self,
        frame: &VideoFrame<'_>,
        metrics: Option<&mut dyn MetricsSink>,
    ) -> Result<Vec<Cut>>;
    fn finish(
        &mut self,
        _last_position: FramePosition,
        _metrics: Option<&mut dyn MetricsSink>,
    ) -> Result<Vec<Cut>> {
        Ok(Vec::new())
    }
}

pub trait VideoSource {
    fn next_frame(&mut self) -> Result<Option<OwnedVideoFrame>>;
    fn frame_rate(&self) -> Rational64;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedVideoFrame {
    pub position: FramePosition,
    pub width: u32,
    pub height: u32,
    pub pixel_format: PixelFormat,
    pub data: Vec<u8>,
    pub stride: usize,
}

impl OwnedVideoFrame {
    pub fn as_frame(&self) -> VideoFrame<'_> {
        VideoFrame {
            position: self.position,
            width: self.width,
            height: self.height,
            pixel_format: self.pixel_format,
            data: &self.data,
            stride: self.stride,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioSampleFormat {
    U8,
    I16,
    I32,
    F32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AudioBuffer {
    U8(Vec<u8>),
    I16(Vec<i16>),
    I32(Vec<i32>),
    F32(Vec<f32>),
}

impl AudioBuffer {
    pub fn len(&self) -> usize {
        match self {
            Self::U8(values) => values.len(),
            Self::I16(values) => values.len(),
            Self::I32(values) => values.len(),
            Self::F32(values) => values.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn sample_format(&self) -> AudioSampleFormat {
        match self {
            Self::U8(_) => AudioSampleFormat::U8,
            Self::I16(_) => AudioSampleFormat::I16,
            Self::I32(_) => AudioSampleFormat::I32,
            Self::F32(_) => AudioSampleFormat::F32,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AudioFrame<'a> {
    pub timestamp: Timestamp,
    pub sample_rate: u32,
    pub channels: u16,
    pub data: &'a AudioBuffer,
}

impl<'a> AudioFrame<'a> {
    pub fn new(
        timestamp: Timestamp,
        sample_rate: u32,
        channels: u16,
        data: &'a AudioBuffer,
    ) -> Result<Self> {
        if sample_rate == 0 || channels == 0 {
            return Err(DetectError::InvalidAudioFormat {
                sample_rate,
                channels,
            });
        }
        Ok(Self {
            timestamp,
            sample_rate,
            channels,
            data,
        })
    }

    pub fn sample_format(&self) -> AudioSampleFormat {
        self.data.sample_format()
    }

    pub fn sample_count(&self) -> usize {
        self.data.len()
    }

    pub fn samples_per_channel(&self) -> usize {
        self.sample_count() / self.channels as usize
    }

    pub fn duration_seconds(&self) -> f64 {
        self.samples_per_channel() as f64 / self.sample_rate as f64
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OwnedAudioFrame {
    pub timestamp: Timestamp,
    pub sample_rate: u32,
    pub channels: u16,
    pub data: AudioBuffer,
}

impl OwnedAudioFrame {
    pub fn new(
        timestamp: Timestamp,
        sample_rate: u32,
        channels: u16,
        data: AudioBuffer,
    ) -> Result<Self> {
        AudioFrame::new(timestamp, sample_rate, channels, &data)?;
        Ok(Self {
            timestamp,
            sample_rate,
            channels,
            data,
        })
    }

    pub fn as_frame(&self) -> Result<AudioFrame<'_>> {
        AudioFrame::new(self.timestamp, self.sample_rate, self.channels, &self.data)
    }

    pub fn sample_format(&self) -> AudioSampleFormat {
        self.data.sample_format()
    }

    pub fn samples_per_channel(&self) -> usize {
        if self.channels == 0 {
            return 0;
        }
        self.data.len() / self.channels as usize
    }

    pub fn duration_seconds(&self) -> f64 {
        if self.sample_rate == 0 {
            return 0.0;
        }
        self.samples_per_channel() as f64 / self.sample_rate as f64
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TextSegment<'a> {
    pub segment_index: u64,
    pub timestamp: Option<Timestamp>,
    pub text: &'a str,
    pub language: Option<&'a str>,
    pub is_final: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedTextSegment {
    pub segment_index: u64,
    pub timestamp: Option<Timestamp>,
    pub text: String,
    pub language: Option<String>,
    pub is_final: bool,
}

impl OwnedTextSegment {
    pub fn new(segment_index: u64, text: impl Into<String>) -> Self {
        Self {
            segment_index,
            timestamp: None,
            text: text.into(),
            language: None,
            is_final: true,
        }
    }

    pub fn timestamp(mut self, timestamp: Timestamp) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    pub fn language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }

    pub fn finality(mut self, is_final: bool) -> Self {
        self.is_final = is_final;
        self
    }

    pub fn as_segment(&self) -> TextSegment<'_> {
        TextSegment {
            segment_index: self.segment_index,
            timestamp: self.timestamp,
            text: &self.text,
            language: self.language.as_deref(),
            is_final: self.is_final,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct DetectionResult {
    pub scenes: Vec<Scene>,
    pub cuts: Vec<Cut>,
    pub metrics: MetricsStore,
    pub frames_processed: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FrameAnalysis {
    pub position: FramePosition,
    pub cuts: Vec<Cut>,
    pub frames_processed: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundingBox {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl BoundingBox {
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Result<Self> {
        if width == 0 || height == 0 {
            return Err(DetectError::InvalidArgument(
                "bounding box must have non-zero width and height".to_string(),
            ));
        }
        Ok(Self {
            x,
            y,
            width,
            height,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObservationKind {
    Text,
    Face,
    Object,
    Scene,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Observation {
    pub timestamp: Option<Timestamp>,
    pub frame: Option<FramePosition>,
    pub scene_index: Option<u64>,
    pub analyzer: String,
    pub kind: ObservationKind,
    pub label: Option<String>,
    pub text: Option<String>,
    pub score: Option<f32>,
    pub region: Option<BoundingBox>,
    pub track_id: Option<String>,
    pub attributes: BTreeMap<String, String>,
}

impl Observation {
    pub fn new(analyzer: impl Into<String>, kind: ObservationKind) -> Self {
        Self {
            timestamp: None,
            frame: None,
            scene_index: None,
            analyzer: analyzer.into(),
            kind,
            label: None,
            text: None,
            score: None,
            region: None,
            track_id: None,
            attributes: BTreeMap::new(),
        }
    }

    pub fn at_frame(mut self, position: FramePosition) -> Self {
        self.timestamp = Some(position.timestamp);
        self.frame = Some(position);
        self
    }

    pub fn at_timestamp(mut self, timestamp: Timestamp) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    pub fn in_scene(mut self, scene_index: u64) -> Self {
        self.scene_index = Some(scene_index);
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    pub fn score(mut self, score: f32) -> Self {
        self.score = Some(score);
        self
    }

    pub fn region(mut self, region: BoundingBox) -> Self {
        self.region = Some(region);
        self
    }

    pub fn track_id(mut self, track_id: impl Into<String>) -> Self {
        self.track_id = Some(track_id.into());
        self
    }

    pub fn attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }

    pub fn to_text_segment(&self, segment_index: u64) -> Option<OwnedTextSegment> {
        let text = self.text.as_ref()?;
        let mut segment = OwnedTextSegment::new(segment_index, text.clone());
        if let Some(timestamp) = self.timestamp {
            segment = segment.timestamp(timestamp);
        }
        if let Some(language) = self.attributes.get("language") {
            segment = segment.language(language.clone());
        }
        Some(segment)
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct VideoAnalysisResult {
    pub observations: Vec<Observation>,
    pub frames_processed: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VideoFrameAnalysis {
    pub position: FramePosition,
    pub observations: Vec<Observation>,
    pub frames_processed: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SceneAnalysis {
    pub scene_index: u64,
    pub scene: Scene,
    pub observations: Vec<Observation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RealtimeVideoFrameAnalysis {
    pub position: FramePosition,
    pub scene: FrameAnalysis,
    pub observations: Vec<Observation>,
    pub completed_scenes: Vec<SceneAnalysis>,
    pub frames_processed: u64,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct RealtimeVideoAnalysisResult {
    pub detection: DetectionResult,
    pub observations: Vec<Observation>,
    pub scenes: Vec<SceneAnalysis>,
    pub frames_processed: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnalysisEvent {
    pub timestamp: Option<Timestamp>,
    pub analyzer: String,
    pub label: String,
    pub score: Option<f32>,
}

impl AnalysisEvent {
    pub fn new(analyzer: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            timestamp: None,
            analyzer: analyzer.into(),
            label: label.into(),
            score: None,
        }
    }

    pub fn at_timestamp(mut self, timestamp: Timestamp) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    pub fn score(mut self, score: f32) -> Self {
        self.score = Some(score);
        self
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct AudioAnalysisResult {
    pub events: Vec<AnalysisEvent>,
    pub frames_processed: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AudioAnalysis {
    pub timestamp: Timestamp,
    pub events: Vec<AnalysisEvent>,
    pub frames_processed: u64,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct TextAnalysisResult {
    pub events: Vec<AnalysisEvent>,
    pub segments_processed: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextAnalysis {
    pub segment_index: u64,
    pub events: Vec<AnalysisEvent>,
    pub segments_processed: u64,
}

pub struct ScenePipeline {
    detectors: Vec<Box<dyn SceneDetector>>,
    start_in_scene: bool,
    crop: Option<CropRegion>,
    auto_downscale_min_width: Option<u32>,
    state: ScenePipelineState,
}

#[derive(Debug, Default, Clone)]
struct ScenePipelineState {
    cuts: Vec<Cut>,
    metrics: MetricsStore,
    first_position: Option<FramePosition>,
    last_position: Option<FramePosition>,
    frames_processed: u64,
    finished: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CropRegion {
    pub x0: u32,
    pub y0: u32,
    pub x1: u32,
    pub y1: u32,
}

impl CropRegion {
    pub fn new(x0: u32, y0: u32, x1: u32, y1: u32) -> Result<Self> {
        if x0 == x1 || y0 == y1 {
            return Err(DetectError::InvalidArgument(
                "crop region must have non-zero width and height".to_string(),
            ));
        }
        Ok(Self { x0, y0, x1, y1 })
    }
}

impl ScenePipeline {
    pub fn builder() -> ScenePipelineBuilder {
        ScenePipelineBuilder::default()
    }

    pub fn detect<S: VideoSource>(&mut self, source: &mut S) -> Result<DetectionResult> {
        self.reset();
        while let Some(frame) = source.next_frame()? {
            self.process_frame(frame)?;
        }
        self.finish_detection()
    }

    pub fn process_frame(&mut self, frame: OwnedVideoFrame) -> Result<FrameAnalysis> {
        let frame = self.prepare_frame(frame)?;
        self.process_frame_ref(&frame.as_frame())
    }

    pub fn process_frame_ref(&mut self, frame: &VideoFrame<'_>) -> Result<FrameAnalysis> {
        if self.state.finished {
            return Err(DetectError::InvalidArgument(
                "cannot process frames after finish_detection; call reset first".to_string(),
            ));
        }
        self.validate_frame_options(frame)?;
        self.state.first_position.get_or_insert(frame.position);
        self.state.last_position = Some(frame.position);

        let mut cuts = Vec::new();
        for detector in &mut self.detectors {
            let mut new_cuts = detector.process_frame(frame, Some(&mut self.state.metrics))?;
            cuts.append(&mut new_cuts);
        }
        let cuts = self.record_cuts(cuts);
        self.state.frames_processed += 1;

        Ok(FrameAnalysis {
            position: frame.position,
            cuts,
            frames_processed: self.state.frames_processed,
        })
    }

    pub fn finish_detection(&mut self) -> Result<DetectionResult> {
        if !self.state.finished {
            if let Some(last) = self.state.last_position {
                let mut cuts = Vec::new();
                for detector in &mut self.detectors {
                    let mut new_cuts = detector.finish(last, Some(&mut self.state.metrics))?;
                    cuts.append(&mut new_cuts);
                }
                self.record_cuts(cuts);
            }
            self.state.finished = true;
        }

        let scenes = if let (Some(start), Some(end)) =
            (self.state.first_position, self.state.last_position)
        {
            scenes_from_cuts(&self.state.cuts, start, end, self.start_in_scene)
        } else {
            Vec::new()
        };

        Ok(DetectionResult {
            scenes,
            cuts: self.state.cuts.clone(),
            metrics: self.state.metrics.clone(),
            frames_processed: self.state.frames_processed,
        })
    }

    pub fn reset(&mut self) {
        self.state = ScenePipelineState::default();
    }

    pub fn metrics(&self) -> &MetricsStore {
        &self.state.metrics
    }

    pub fn cuts(&self) -> &[Cut] {
        &self.state.cuts
    }

    pub fn frames_processed(&self) -> u64 {
        self.state.frames_processed
    }

    fn record_cuts(&mut self, mut cuts: Vec<Cut>) -> Vec<Cut> {
        cuts.sort_by_key(|cut| cut.position.frame_index);
        cuts.dedup_by_key(|cut| cut.position.frame_index);
        let mut accepted = Vec::new();
        for cut in cuts {
            if self
                .state
                .cuts
                .iter()
                .any(|existing| existing.position.frame_index == cut.position.frame_index)
            {
                continue;
            }
            self.state.cuts.push(cut.clone());
            accepted.push(cut);
        }
        self.state.cuts.sort_by_key(|cut| cut.position.frame_index);
        accepted
    }

    fn prepare_frame(&self, frame: OwnedVideoFrame) -> Result<OwnedVideoFrame> {
        self.validate_frame_options(&frame.as_frame())?;
        Ok(frame)
    }

    fn validate_frame_options(&self, frame: &VideoFrame<'_>) -> Result<()> {
        let _ = self.auto_downscale_min_width;
        if let Some(crop) = self.crop {
            if crop.x0 >= frame.width || crop.y0 >= frame.height {
                return Err(DetectError::InvalidArgument(
                    "crop starts outside frame boundary".to_string(),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct ScenePipelineBuilder {
    detectors: Vec<Box<dyn SceneDetector>>,
    start_in_scene: bool,
    crop: Option<CropRegion>,
    auto_downscale_min_width: Option<u32>,
}

impl ScenePipelineBuilder {
    pub fn detector<D: SceneDetector + 'static>(mut self, detector: D) -> Self {
        self.detectors.push(Box::new(detector));
        self
    }

    pub fn start_in_scene(mut self, value: bool) -> Self {
        self.start_in_scene = value;
        self
    }

    pub fn crop(mut self, value: Option<CropRegion>) -> Self {
        self.crop = value;
        self
    }

    pub fn auto_downscale_min_width(mut self, value: u32) -> Self {
        self.auto_downscale_min_width = Some(value);
        self
    }

    pub fn build(self) -> Result<ScenePipeline> {
        if self.detectors.is_empty() {
            return Err(DetectError::InvalidArgument(
                "at least one detector is required".to_string(),
            ));
        }
        Ok(ScenePipeline {
            detectors: self.detectors,
            start_in_scene: self.start_in_scene,
            crop: self.crop,
            auto_downscale_min_width: self.auto_downscale_min_width,
            state: ScenePipelineState::default(),
        })
    }
}

pub trait VideoAnalyzer {
    fn name(&self) -> &str;

    fn process_frame(&mut self, frame: &VideoFrame<'_>) -> Result<Vec<Observation>>;

    fn finish(&mut self, _last_position: Option<FramePosition>) -> Result<Vec<Observation>> {
        Ok(Vec::new())
    }
}

pub struct VideoAnalysisPipeline {
    analyzers: Vec<Box<dyn VideoAnalyzer>>,
    state: VideoAnalysisPipelineState,
}

#[derive(Debug, Default, Clone)]
struct VideoAnalysisPipelineState {
    observations: Vec<Observation>,
    last_position: Option<FramePosition>,
    frames_processed: u64,
    finished: bool,
}

impl VideoAnalysisPipeline {
    pub fn builder() -> VideoAnalysisPipelineBuilder {
        VideoAnalysisPipelineBuilder::default()
    }

    pub fn process_frame(&mut self, frame: OwnedVideoFrame) -> Result<VideoFrameAnalysis> {
        self.process_frame_ref(&frame.as_frame())
    }

    pub fn process_frame_ref(&mut self, frame: &VideoFrame<'_>) -> Result<VideoFrameAnalysis> {
        if self.state.finished {
            return Err(DetectError::InvalidArgument(
                "cannot process video frames after finish_analysis; call reset first".to_string(),
            ));
        }
        self.state.last_position = Some(frame.position);

        let mut observations = Vec::new();
        for analyzer in &mut self.analyzers {
            let mut new_observations = analyzer.process_frame(frame)?;
            for observation in &mut new_observations {
                if observation.timestamp.is_none() {
                    observation.timestamp = Some(frame.position.timestamp);
                }
                if observation.frame.is_none() {
                    observation.frame = Some(frame.position);
                }
            }
            observations.append(&mut new_observations);
        }
        self.state.observations.extend(observations.iter().cloned());
        self.state.frames_processed += 1;

        Ok(VideoFrameAnalysis {
            position: frame.position,
            observations,
            frames_processed: self.state.frames_processed,
        })
    }

    pub fn finish_analysis(&mut self) -> Result<VideoAnalysisResult> {
        if !self.state.finished {
            let mut observations = Vec::new();
            for analyzer in &mut self.analyzers {
                let mut new_observations = analyzer.finish(self.state.last_position)?;
                observations.append(&mut new_observations);
            }
            self.state.observations.extend(observations);
            self.state.finished = true;
        }
        Ok(VideoAnalysisResult {
            observations: self.state.observations.clone(),
            frames_processed: self.state.frames_processed,
        })
    }

    pub fn reset(&mut self) {
        self.state = VideoAnalysisPipelineState::default();
    }

    pub fn observations(&self) -> &[Observation] {
        &self.state.observations
    }

    pub fn frames_processed(&self) -> u64 {
        self.state.frames_processed
    }
}

#[derive(Default)]
pub struct VideoAnalysisPipelineBuilder {
    analyzers: Vec<Box<dyn VideoAnalyzer>>,
}

impl VideoAnalysisPipelineBuilder {
    pub fn analyzer<A: VideoAnalyzer + 'static>(mut self, analyzer: A) -> Self {
        self.analyzers.push(Box::new(analyzer));
        self
    }

    pub fn build(self) -> Result<VideoAnalysisPipeline> {
        if self.analyzers.is_empty() {
            return Err(DetectError::InvalidArgument(
                "at least one video analyzer is required".to_string(),
            ));
        }
        Ok(VideoAnalysisPipeline {
            analyzers: self.analyzers,
            state: VideoAnalysisPipelineState::default(),
        })
    }
}

pub struct RealtimeVideoPipeline {
    scene_pipeline: ScenePipeline,
    video_pipeline: Option<VideoAnalysisPipeline>,
    state: RealtimeVideoPipelineState,
}

#[derive(Debug, Default, Clone)]
struct RealtimeVideoPipelineState {
    observations: Vec<Observation>,
    completed_scenes: Vec<SceneAnalysis>,
    open_scene_observations: Vec<Observation>,
    active_scene_start: Option<FramePosition>,
    active_scene_index: u64,
    first_position: Option<FramePosition>,
    last_position: Option<FramePosition>,
    frames_processed: u64,
    finished: bool,
}

impl RealtimeVideoPipeline {
    pub fn builder() -> RealtimeVideoPipelineBuilder {
        RealtimeVideoPipelineBuilder::default()
    }

    pub fn new(
        scene_pipeline: ScenePipeline,
        video_pipeline: Option<VideoAnalysisPipeline>,
    ) -> Self {
        Self {
            scene_pipeline,
            video_pipeline,
            state: RealtimeVideoPipelineState::default(),
        }
    }

    pub fn process_frame(&mut self, frame: OwnedVideoFrame) -> Result<RealtimeVideoFrameAnalysis> {
        self.process_frame_ref(&frame.as_frame())
    }

    pub fn process_frame_ref(
        &mut self,
        frame: &VideoFrame<'_>,
    ) -> Result<RealtimeVideoFrameAnalysis> {
        if self.state.finished {
            return Err(DetectError::InvalidArgument(
                "cannot process video frames after finish_analysis; call reset first".to_string(),
            ));
        }

        self.state.first_position.get_or_insert(frame.position);
        self.state.last_position = Some(frame.position);
        self.state.active_scene_start.get_or_insert(frame.position);

        let scene = self.scene_pipeline.process_frame_ref(frame)?;
        let completed_scenes = self.close_scenes(&scene.cuts);
        let mut observations = if let Some(pipeline) = &mut self.video_pipeline {
            pipeline.process_frame_ref(frame)?.observations
        } else {
            Vec::new()
        };
        self.annotate_observations(&mut observations, frame.position);
        self.state
            .open_scene_observations
            .extend(observations.iter().cloned());
        self.state.observations.extend(observations.iter().cloned());
        self.state.frames_processed += 1;

        Ok(RealtimeVideoFrameAnalysis {
            position: frame.position,
            scene,
            observations,
            completed_scenes,
            frames_processed: self.state.frames_processed,
        })
    }

    pub fn finish_analysis(&mut self) -> Result<RealtimeVideoAnalysisResult> {
        let detection = self.scene_pipeline.finish_detection()?;
        if !self.state.finished {
            let pending_cuts = detection
                .cuts
                .iter()
                .filter(|cut| {
                    self.state
                        .active_scene_start
                        .map(|start| cut.position.frame_index > start.frame_index)
                        .unwrap_or(false)
                })
                .cloned()
                .collect::<Vec<_>>();
            self.close_scenes(&pending_cuts);

            if let Some(pipeline) = &mut self.video_pipeline {
                let already_observed = self.state.observations.len();
                let result = pipeline.finish_analysis()?;
                let mut final_observations = result
                    .observations
                    .into_iter()
                    .skip(already_observed)
                    .collect::<Vec<_>>();
                self.annotate_observations_without_frame(&mut final_observations);
                self.state
                    .open_scene_observations
                    .extend(final_observations.iter().cloned());
                self.state
                    .observations
                    .extend(final_observations.iter().cloned());
            }
            self.state.finished = true;
        }

        let scenes = self.scene_analyses_for(&detection.scenes);
        Ok(RealtimeVideoAnalysisResult {
            detection,
            observations: self.state.observations.clone(),
            scenes,
            frames_processed: self.state.frames_processed,
        })
    }

    pub fn reset(&mut self) {
        self.scene_pipeline.reset();
        if let Some(pipeline) = &mut self.video_pipeline {
            pipeline.reset();
        }
        self.state = RealtimeVideoPipelineState::default();
    }

    pub fn observations(&self) -> &[Observation] {
        &self.state.observations
    }

    pub fn completed_scenes(&self) -> &[SceneAnalysis] {
        &self.state.completed_scenes
    }

    pub fn frames_processed(&self) -> u64 {
        self.state.frames_processed
    }

    fn close_scenes(&mut self, cuts: &[Cut]) -> Vec<SceneAnalysis> {
        let mut completed = Vec::new();
        for cut in cuts {
            let Some(start) = self.state.active_scene_start else {
                continue;
            };
            if cut.position.frame_index <= start.frame_index {
                continue;
            }
            let scene = Scene {
                start,
                end: cut.position,
            };
            let observations = std::mem::take(&mut self.state.open_scene_observations);
            let analysis = SceneAnalysis {
                scene_index: self.state.active_scene_index,
                scene,
                observations,
            };
            self.state.completed_scenes.push(analysis.clone());
            completed.push(analysis);
            self.state.active_scene_index += 1;
            self.state.active_scene_start = Some(cut.position);
        }
        completed
    }

    fn annotate_observations(&self, observations: &mut [Observation], position: FramePosition) {
        for observation in observations {
            if observation.timestamp.is_none() {
                observation.timestamp = Some(position.timestamp);
            }
            if observation.frame.is_none() {
                observation.frame = Some(position);
            }
            if observation.scene_index.is_none() {
                observation.scene_index = Some(self.state.active_scene_index);
            }
        }
    }

    fn annotate_observations_without_frame(&self, observations: &mut [Observation]) {
        for observation in observations {
            if observation.scene_index.is_none() {
                observation.scene_index = Some(self.state.active_scene_index);
            }
        }
    }

    fn scene_analyses_for(&self, scenes: &[Scene]) -> Vec<SceneAnalysis> {
        scenes
            .iter()
            .enumerate()
            .map(|(index, scene)| {
                let scene_index = index as u64;
                let observations = self
                    .state
                    .observations
                    .iter()
                    .filter(|observation| observation.scene_index == Some(scene_index))
                    .cloned()
                    .collect();
                SceneAnalysis {
                    scene_index,
                    scene: scene.clone(),
                    observations,
                }
            })
            .collect()
    }
}

#[derive(Default)]
pub struct RealtimeVideoPipelineBuilder {
    scene_pipeline: Option<ScenePipeline>,
    scene_builder: ScenePipelineBuilder,
    video_analyzers: Vec<Box<dyn VideoAnalyzer>>,
}

impl RealtimeVideoPipelineBuilder {
    pub fn scene_pipeline(mut self, pipeline: ScenePipeline) -> Self {
        self.scene_pipeline = Some(pipeline);
        self
    }

    pub fn scene_detector<D: SceneDetector + 'static>(mut self, detector: D) -> Self {
        self.scene_builder = self.scene_builder.detector(detector);
        self
    }

    pub fn video_analyzer<A: VideoAnalyzer + 'static>(mut self, analyzer: A) -> Self {
        self.video_analyzers.push(Box::new(analyzer));
        self
    }

    pub fn start_in_scene(mut self, value: bool) -> Self {
        self.scene_builder = self.scene_builder.start_in_scene(value);
        self
    }

    pub fn crop(mut self, value: Option<CropRegion>) -> Self {
        self.scene_builder = self.scene_builder.crop(value);
        self
    }

    pub fn auto_downscale_min_width(mut self, value: u32) -> Self {
        self.scene_builder = self.scene_builder.auto_downscale_min_width(value);
        self
    }

    pub fn build(self) -> Result<RealtimeVideoPipeline> {
        let scene_pipeline = match self.scene_pipeline {
            Some(pipeline) => pipeline,
            None => self.scene_builder.build()?,
        };
        let video_pipeline = if self.video_analyzers.is_empty() {
            None
        } else {
            Some(VideoAnalysisPipeline {
                analyzers: self.video_analyzers,
                state: VideoAnalysisPipelineState::default(),
            })
        };
        Ok(RealtimeVideoPipeline::new(scene_pipeline, video_pipeline))
    }
}

pub trait AudioAnalyzer {
    fn name(&self) -> &str;

    fn process_frame(&mut self, frame: &AudioFrame<'_>) -> Result<Vec<AnalysisEvent>>;

    fn finish(&mut self, _last_timestamp: Option<Timestamp>) -> Result<Vec<AnalysisEvent>> {
        Ok(Vec::new())
    }
}

pub struct AudioPipeline {
    analyzers: Vec<Box<dyn AudioAnalyzer>>,
    state: AudioPipelineState,
}

#[derive(Debug, Default, Clone)]
struct AudioPipelineState {
    events: Vec<AnalysisEvent>,
    last_timestamp: Option<Timestamp>,
    frames_processed: u64,
    finished: bool,
}

impl AudioPipeline {
    pub fn builder() -> AudioPipelineBuilder {
        AudioPipelineBuilder::default()
    }

    pub fn process_frame(&mut self, frame: OwnedAudioFrame) -> Result<AudioAnalysis> {
        if self.state.finished {
            return Err(DetectError::InvalidArgument(
                "cannot process audio frames after finish_analysis; call reset first".to_string(),
            ));
        }
        let frame_ref = frame.as_frame()?;
        self.state.last_timestamp = Some(frame_ref.timestamp);

        let mut events = Vec::new();
        for analyzer in &mut self.analyzers {
            let mut new_events = analyzer.process_frame(&frame_ref)?;
            events.append(&mut new_events);
        }
        self.state.events.extend(events.iter().cloned());
        self.state.frames_processed += 1;

        Ok(AudioAnalysis {
            timestamp: frame_ref.timestamp,
            events,
            frames_processed: self.state.frames_processed,
        })
    }

    pub fn finish_analysis(&mut self) -> Result<AudioAnalysisResult> {
        if !self.state.finished {
            let mut events = Vec::new();
            for analyzer in &mut self.analyzers {
                let mut new_events = analyzer.finish(self.state.last_timestamp)?;
                events.append(&mut new_events);
            }
            self.state.events.extend(events);
            self.state.finished = true;
        }
        Ok(AudioAnalysisResult {
            events: self.state.events.clone(),
            frames_processed: self.state.frames_processed,
        })
    }

    pub fn reset(&mut self) {
        self.state = AudioPipelineState::default();
    }

    pub fn events(&self) -> &[AnalysisEvent] {
        &self.state.events
    }

    pub fn frames_processed(&self) -> u64 {
        self.state.frames_processed
    }
}

#[derive(Default)]
pub struct AudioPipelineBuilder {
    analyzers: Vec<Box<dyn AudioAnalyzer>>,
}

impl AudioPipelineBuilder {
    pub fn analyzer<A: AudioAnalyzer + 'static>(mut self, analyzer: A) -> Self {
        self.analyzers.push(Box::new(analyzer));
        self
    }

    pub fn build(self) -> Result<AudioPipeline> {
        if self.analyzers.is_empty() {
            return Err(DetectError::InvalidArgument(
                "at least one audio analyzer is required".to_string(),
            ));
        }
        Ok(AudioPipeline {
            analyzers: self.analyzers,
            state: AudioPipelineState::default(),
        })
    }
}

pub trait TextAnalyzer {
    fn name(&self) -> &str;

    fn process_segment(&mut self, segment: &TextSegment<'_>) -> Result<Vec<AnalysisEvent>>;

    fn finish(&mut self, _last_segment_index: Option<u64>) -> Result<Vec<AnalysisEvent>> {
        Ok(Vec::new())
    }
}

pub struct TextPipeline {
    analyzers: Vec<Box<dyn TextAnalyzer>>,
    state: TextPipelineState,
}

#[derive(Debug, Default, Clone)]
struct TextPipelineState {
    events: Vec<AnalysisEvent>,
    last_segment_index: Option<u64>,
    segments_processed: u64,
    finished: bool,
}

impl TextPipeline {
    pub fn builder() -> TextPipelineBuilder {
        TextPipelineBuilder::default()
    }

    pub fn process_segment(&mut self, segment: OwnedTextSegment) -> Result<TextAnalysis> {
        if self.state.finished {
            return Err(DetectError::InvalidArgument(
                "cannot process text segments after finish_analysis; call reset first".to_string(),
            ));
        }
        let segment_ref = segment.as_segment();
        self.state.last_segment_index = Some(segment_ref.segment_index);

        let mut events = Vec::new();
        for analyzer in &mut self.analyzers {
            let mut new_events = analyzer.process_segment(&segment_ref)?;
            events.append(&mut new_events);
        }
        self.state.events.extend(events.iter().cloned());
        self.state.segments_processed += 1;

        Ok(TextAnalysis {
            segment_index: segment_ref.segment_index,
            events,
            segments_processed: self.state.segments_processed,
        })
    }

    pub fn finish_analysis(&mut self) -> Result<TextAnalysisResult> {
        if !self.state.finished {
            let mut events = Vec::new();
            for analyzer in &mut self.analyzers {
                let mut new_events = analyzer.finish(self.state.last_segment_index)?;
                events.append(&mut new_events);
            }
            self.state.events.extend(events);
            self.state.finished = true;
        }
        Ok(TextAnalysisResult {
            events: self.state.events.clone(),
            segments_processed: self.state.segments_processed,
        })
    }

    pub fn reset(&mut self) {
        self.state = TextPipelineState::default();
    }

    pub fn events(&self) -> &[AnalysisEvent] {
        &self.state.events
    }

    pub fn segments_processed(&self) -> u64 {
        self.state.segments_processed
    }
}

#[derive(Default)]
pub struct TextPipelineBuilder {
    analyzers: Vec<Box<dyn TextAnalyzer>>,
}

impl TextPipelineBuilder {
    pub fn analyzer<A: TextAnalyzer + 'static>(mut self, analyzer: A) -> Self {
        self.analyzers.push(Box::new(analyzer));
        self
    }

    pub fn build(self) -> Result<TextPipeline> {
        if self.analyzers.is_empty() {
            return Err(DetectError::InvalidArgument(
                "at least one text analyzer is required".to_string(),
            ));
        }
        Ok(TextPipeline {
            analyzers: self.analyzers,
            state: TextPipelineState::default(),
        })
    }
}

pub fn scenes_from_cuts(
    cuts: &[Cut],
    start: FramePosition,
    last_frame: FramePosition,
    start_in_scene: bool,
) -> Vec<Scene> {
    if cuts.is_empty() && !start_in_scene {
        return Vec::new();
    }
    let mut scenes = Vec::new();
    let mut scene_start = start;
    for cut in cuts {
        if cut.position.frame_index <= scene_start.frame_index {
            continue;
        }
        scenes.push(Scene {
            start: scene_start,
            end: cut.position,
        });
        scene_start = cut.position;
    }
    scenes.push(Scene {
        start: scene_start,
        end: FramePosition {
            frame_index: last_frame.frame_index + 1,
            timestamp: Timestamp::new(last_frame.timestamp.pts + 1, last_frame.timestamp.timebase),
        },
    });
    scenes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fps() -> Rational64 {
        Rational64::new(30, 1)
    }

    fn pos(frame: u64) -> FramePosition {
        FramePosition::from_frame_index(frame, fps())
    }

    fn owned_frame(frame_index: u64) -> OwnedVideoFrame {
        OwnedVideoFrame {
            position: pos(frame_index),
            width: 1,
            height: 1,
            pixel_format: PixelFormat::Rgb24,
            data: vec![0, 0, 0],
            stride: 3,
        }
    }

    #[test]
    fn timecode_formats_and_clamps_subtraction() {
        let tc = FrameTimecode::from_seconds(10.0, fps()).unwrap();
        assert_eq!(tc.frame_index, 300);
        assert_eq!(tc.timecode(3), "00:00:10.000");
        assert_eq!((FrameTimecode::from_frames(5, fps()) - 10).frame_index, 0);
    }

    #[test]
    fn timecode_parse_accepts_frames_seconds_and_hms() {
        assert_eq!(FrameTimecode::parse("42", fps()).unwrap().frame_index, 42);
        assert_eq!(FrameTimecode::parse("2.0", fps()).unwrap().frame_index, 60);
        assert_eq!(
            FrameTimecode::parse("00:01:00.000", fps())
                .unwrap()
                .frame_index,
            1800
        );
    }

    #[test]
    fn scenes_are_empty_without_cuts_unless_start_in_scene() {
        assert!(scenes_from_cuts(&[], pos(0), pos(9), false).is_empty());
        let scenes = scenes_from_cuts(&[], pos(0), pos(9), true);
        assert_eq!(scenes.len(), 1);
        assert_eq!(scenes[0].end.frame_index, 10);
    }

    #[test]
    fn scenes_are_contiguous_from_unique_cuts() {
        let cuts = vec![Cut {
            position: pos(5),
            detector: "test",
            score: None,
        }];
        let scenes = scenes_from_cuts(&cuts, pos(0), pos(9), true);
        assert_eq!(scenes.len(), 2);
        assert_eq!(scenes[0].start.frame_index, 0);
        assert_eq!(scenes[0].end.frame_index, 5);
        assert_eq!(scenes[1].start.frame_index, 5);
        assert_eq!(scenes[1].end.frame_index, 10);
    }

    #[test]
    fn metrics_store_tracks_keys_and_values() {
        let mut metrics = MetricsStore::default();
        metrics.set_metric(7, "content_val", 12.5);
        assert_eq!(metrics.get(7, "content_val"), Some(12.5));
        assert_eq!(metrics.keys().collect::<Vec<_>>(), vec!["content_val"]);
    }

    #[test]
    fn pipeline_processes_frames_incrementally() {
        struct CutOnFrame(u64);

        impl SceneDetector for CutOnFrame {
            fn name(&self) -> &'static str {
                "test"
            }

            fn metric_keys(&self) -> &'static [&'static str] {
                &[]
            }

            fn process_frame(
                &mut self,
                frame: &VideoFrame<'_>,
                _metrics: Option<&mut dyn MetricsSink>,
            ) -> Result<Vec<Cut>> {
                Ok((frame.position.frame_index == self.0)
                    .then(|| Cut {
                        position: frame.position,
                        detector: self.name(),
                        score: Some(1.0),
                    })
                    .into_iter()
                    .collect())
            }
        }

        let mut pipeline = ScenePipeline::builder()
            .detector(CutOnFrame(1))
            .start_in_scene(true)
            .build()
            .unwrap();

        assert!(pipeline
            .process_frame(owned_frame(0))
            .unwrap()
            .cuts
            .is_empty());
        let analysis = pipeline.process_frame(owned_frame(1)).unwrap();
        assert_eq!(analysis.frames_processed, 2);
        assert_eq!(analysis.cuts[0].position.frame_index, 1);

        let result = pipeline.finish_detection().unwrap();
        assert_eq!(result.frames_processed, 2);
        assert_eq!(result.cuts.len(), 1);
        assert_eq!(result.scenes.len(), 2);
    }

    #[test]
    fn video_pipeline_emits_frame_observations() {
        struct OcrAnalyzer;

        impl VideoAnalyzer for OcrAnalyzer {
            fn name(&self) -> &str {
                "ocr"
            }

            fn process_frame(&mut self, frame: &VideoFrame<'_>) -> Result<Vec<Observation>> {
                Ok(vec![Observation::new(self.name(), ObservationKind::Text)
                    .at_frame(frame.position)
                    .text("EXIT")
                    .score(0.9)])
            }
        }

        let mut pipeline = VideoAnalysisPipeline::builder()
            .analyzer(OcrAnalyzer)
            .build()
            .unwrap();

        let analysis = pipeline.process_frame(owned_frame(3)).unwrap();
        assert_eq!(analysis.frames_processed, 1);
        assert_eq!(analysis.observations[0].text.as_deref(), Some("EXIT"));
        assert_eq!(
            analysis.observations[0].to_text_segment(0).unwrap().text,
            "EXIT"
        );
        assert_eq!(pipeline.finish_analysis().unwrap().observations.len(), 1);
    }

    #[test]
    fn realtime_video_pipeline_groups_observations_by_completed_scene() {
        struct CutOnFrame(u64);

        impl SceneDetector for CutOnFrame {
            fn name(&self) -> &'static str {
                "test"
            }

            fn metric_keys(&self) -> &'static [&'static str] {
                &[]
            }

            fn process_frame(
                &mut self,
                frame: &VideoFrame<'_>,
                _metrics: Option<&mut dyn MetricsSink>,
            ) -> Result<Vec<Cut>> {
                Ok((frame.position.frame_index == self.0)
                    .then(|| Cut {
                        position: frame.position,
                        detector: self.name(),
                        score: Some(1.0),
                    })
                    .into_iter()
                    .collect())
            }
        }

        struct ObjectAnalyzer;

        impl VideoAnalyzer for ObjectAnalyzer {
            fn name(&self) -> &str {
                "objects"
            }

            fn process_frame(&mut self, frame: &VideoFrame<'_>) -> Result<Vec<Observation>> {
                Ok(vec![Observation::new(self.name(), ObservationKind::Object)
                    .at_frame(frame.position)
                    .label(format!("frame-{}", frame.position.frame_index))])
            }
        }

        let mut pipeline = RealtimeVideoPipeline::builder()
            .scene_detector(CutOnFrame(2))
            .video_analyzer(ObjectAnalyzer)
            .start_in_scene(true)
            .build()
            .unwrap();

        assert!(pipeline
            .process_frame(owned_frame(0))
            .unwrap()
            .completed_scenes
            .is_empty());
        pipeline.process_frame(owned_frame(1)).unwrap();
        let analysis = pipeline.process_frame(owned_frame(2)).unwrap();

        assert_eq!(analysis.completed_scenes.len(), 1);
        assert_eq!(analysis.completed_scenes[0].scene.start.frame_index, 0);
        assert_eq!(analysis.completed_scenes[0].scene.end.frame_index, 2);
        assert_eq!(analysis.completed_scenes[0].observations.len(), 2);
        assert_eq!(analysis.observations[0].scene_index, Some(1));

        let result = pipeline.finish_analysis().unwrap();
        assert_eq!(result.detection.scenes.len(), 2);
        assert_eq!(result.scenes[0].observations.len(), 2);
        assert_eq!(result.scenes[1].observations.len(), 1);
    }

    #[test]
    fn audio_pipeline_processes_frames_incrementally() {
        struct LoudnessAnalyzer;

        impl AudioAnalyzer for LoudnessAnalyzer {
            fn name(&self) -> &str {
                "loudness"
            }

            fn process_frame(&mut self, frame: &AudioFrame<'_>) -> Result<Vec<AnalysisEvent>> {
                let AudioBuffer::F32(samples) = frame.data else {
                    return Ok(Vec::new());
                };
                let mean = samples.iter().map(|sample| sample.abs()).sum::<f32>()
                    / samples.len().max(1) as f32;
                Ok((mean > 0.5)
                    .then(|| {
                        AnalysisEvent::new(self.name(), "loud")
                            .at_timestamp(frame.timestamp)
                            .score(mean)
                    })
                    .into_iter()
                    .collect())
            }
        }

        let mut pipeline = AudioPipeline::builder()
            .analyzer(LoudnessAnalyzer)
            .build()
            .unwrap();
        let frame = OwnedAudioFrame::new(
            Timestamp::new(0, Timebase::new(1, 48_000)),
            48_000,
            1,
            AudioBuffer::F32(vec![1.0, 0.5]),
        )
        .unwrap();

        let analysis = pipeline.process_frame(frame).unwrap();
        assert_eq!(analysis.frames_processed, 1);
        assert_eq!(analysis.events[0].label, "loud");
        assert_eq!(pipeline.finish_analysis().unwrap().events.len(), 1);
    }

    #[test]
    fn text_pipeline_processes_segments_incrementally() {
        struct KeywordAnalyzer;

        impl TextAnalyzer for KeywordAnalyzer {
            fn name(&self) -> &str {
                "keyword"
            }

            fn process_segment(&mut self, segment: &TextSegment<'_>) -> Result<Vec<AnalysisEvent>> {
                Ok(segment
                    .text
                    .contains("cut")
                    .then(|| {
                        let mut event = AnalysisEvent::new(self.name(), "keyword").score(1.0);
                        if let Some(timestamp) = segment.timestamp {
                            event = event.at_timestamp(timestamp);
                        }
                        event
                    })
                    .into_iter()
                    .collect())
            }
        }

        let mut pipeline = TextPipeline::builder()
            .analyzer(KeywordAnalyzer)
            .build()
            .unwrap();
        let segment = OwnedTextSegment::new(0, "find this cut point");

        let analysis = pipeline.process_segment(segment).unwrap();
        assert_eq!(analysis.segments_processed, 1);
        assert_eq!(analysis.events[0].analyzer, "keyword");
        assert_eq!(pipeline.finish_analysis().unwrap().segments_processed, 1);
    }

    #[test]
    fn crop_rejects_empty_regions() {
        assert!(CropRegion::new(0, 0, 0, 10).is_err());
    }
}
