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
    #[error("invalid frame buffer: expected at least {expected} bytes, got {actual}")]
    InvalidFrameBuffer { expected: usize, actual: usize },
    #[error("invalid dimensions: {width}x{height}")]
    InvalidDimensions { width: u32, height: u32 },
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

#[derive(Debug, Default, Clone)]
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

#[derive(Debug, Clone)]
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

#[derive(Debug, Default, Clone)]
pub struct DetectionResult {
    pub scenes: Vec<Scene>,
    pub cuts: Vec<Cut>,
    pub metrics: MetricsStore,
    pub frames_processed: u64,
}

pub struct ScenePipeline {
    detectors: Vec<Box<dyn SceneDetector>>,
    start_in_scene: bool,
    crop: Option<CropRegion>,
    auto_downscale_min_width: Option<u32>,
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
        let mut cuts = Vec::new();
        let mut metrics = MetricsStore::default();
        let mut first_position = None;
        let mut last_position = None;
        let mut frames_processed = 0_u64;

        while let Some(frame) = source.next_frame()? {
            let frame = self.prepare_frame(frame)?;
            let frame_ref = frame.as_frame();
            first_position.get_or_insert(frame_ref.position);
            last_position = Some(frame_ref.position);
            for detector in &mut self.detectors {
                let mut new_cuts = detector.process_frame(&frame_ref, Some(&mut metrics))?;
                cuts.append(&mut new_cuts);
            }
            frames_processed += 1;
        }

        if let Some(last) = last_position {
            for detector in &mut self.detectors {
                let mut new_cuts = detector.finish(last, Some(&mut metrics))?;
                cuts.append(&mut new_cuts);
            }
        }

        cuts.sort_by_key(|cut| cut.position.frame_index);
        cuts.dedup_by_key(|cut| cut.position.frame_index);
        let scenes = if let (Some(start), Some(end)) = (first_position, last_position) {
            scenes_from_cuts(&cuts, start, end, self.start_in_scene)
        } else {
            Vec::new()
        };

        Ok(DetectionResult {
            scenes,
            cuts,
            metrics,
            frames_processed,
        })
    }

    fn prepare_frame(&self, frame: OwnedVideoFrame) -> Result<OwnedVideoFrame> {
        let _ = self.auto_downscale_min_width;
        if let Some(crop) = self.crop {
            if crop.x0 >= frame.width || crop.y0 >= frame.height {
                return Err(DetectError::InvalidArgument(
                    "crop starts outside frame boundary".to_string(),
                ));
            }
        }
        Ok(frame)
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
    fn crop_rejects_empty_regions() {
        assert!(CropRegion::new(0, 0, 0, 10).is_err());
    }
}
