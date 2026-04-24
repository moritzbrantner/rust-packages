#![doc = include_str!("../README.md")]

use std::collections::BTreeMap;
use std::mem;

use serde::{Deserialize, Serialize};
use video_analysis_core::{
    AnalysisEvent, AudioBuffer, AudioFrame, AudioSampleFormat, BoundingBox, Cut, DetectionResult,
    FramePosition, MetricsStore, Observation, ObservationKind, PixelFormat, Scene, TextSegment,
    Timestamp, VideoFrame,
};
use video_analysis_posture::{Keypoint, Keypoint3d, Pose3dEstimate, PoseEstimate};

pub const DATASET_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatasetMetadata {
    pub schema_version: u32,
    pub name: Option<String>,
    pub source: Option<String>,
    pub created_at: Option<String>,
    pub attributes: BTreeMap<String, String>,
}

impl Default for DatasetMetadata {
    fn default() -> Self {
        Self {
            schema_version: DATASET_SCHEMA_VERSION,
            name: None,
            source: None,
            created_at: None,
            attributes: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalysisDataset {
    pub metadata: DatasetMetadata,
    pub records: Vec<DatasetRecord>,
}

impl AnalysisDataset {
    pub fn new(metadata: DatasetMetadata) -> Self {
        Self {
            metadata,
            records: Vec::new(),
        }
    }

    pub fn empty() -> Self {
        Self::new(DatasetMetadata::default())
    }

    pub fn push(&mut self, record: DatasetRecord) {
        self.records.push(record);
    }

    pub fn extend_records(&mut self, records: impl IntoIterator<Item = DatasetRecord>) {
        self.records.extend(records);
    }

    pub fn extend_detection_result(&mut self, result: &DetectionResult) {
        self.records
            .extend(result.scenes.iter().enumerate().map(|(index, scene)| {
                DatasetRecord::Scene(SceneRecord::from_scene(index as u64, scene))
            }));
        self.records.extend(
            result
                .cuts
                .iter()
                .map(|cut| DatasetRecord::Cut(CutRecord::from_cut(cut))),
        );
        self.records
            .extend(metric_records(&result.metrics).map(DatasetRecord::Metric));
    }

    pub fn extend_observations(&mut self, observations: impl IntoIterator<Item = Observation>) {
        self.records
            .extend(observations.into_iter().map(|observation| {
                DatasetRecord::Observation(ObservationRecord::from_observation(observation))
            }));
    }

    pub fn extend_events(&mut self, events: impl IntoIterator<Item = AnalysisEvent>) {
        self.records.extend(
            events
                .into_iter()
                .map(|event| DatasetRecord::Event(AnalysisEventRecord::from_event(event))),
        );
    }

    pub fn extend_pose_estimates(
        &mut self,
        analyzer: impl Into<String>,
        frame: Option<FramePosition>,
        poses: impl IntoIterator<Item = PoseEstimate>,
    ) {
        let analyzer = analyzer.into();
        self.records.extend(poses.into_iter().map(|pose| {
            DatasetRecord::Pose2d(Pose2dRecord::from_pose_estimate(
                analyzer.clone(),
                frame,
                pose,
            ))
        }));
    }

    pub fn extend_pose_3d_estimates(
        &mut self,
        analyzer: impl Into<String>,
        frame: Option<FramePosition>,
        poses: impl IntoIterator<Item = Pose3dEstimate>,
    ) {
        let analyzer = analyzer.into();
        self.records.extend(poses.into_iter().map(|pose| {
            DatasetRecord::Pose3d(Pose3dRecord::from_pose_3d_estimate(
                analyzer.clone(),
                frame,
                pose,
            ))
        }));
    }

    pub fn records(&self) -> impl Iterator<Item = &DatasetRecord> {
        self.records.iter()
    }

    pub fn scenes(&self) -> impl Iterator<Item = &SceneRecord> {
        self.records.iter().filter_map(|record| match record {
            DatasetRecord::Scene(scene) => Some(scene),
            _ => None,
        })
    }

    pub fn observations(&self) -> impl Iterator<Item = &ObservationRecord> {
        self.records.iter().filter_map(|record| match record {
            DatasetRecord::Observation(observation) => Some(observation),
            _ => None,
        })
    }

    pub fn events(&self) -> impl Iterator<Item = &AnalysisEventRecord> {
        self.records.iter().filter_map(|record| match record {
            DatasetRecord::Event(event) => Some(event),
            _ => None,
        })
    }

    pub fn features(&self) -> impl Iterator<Item = &FeatureRecord> {
        self.records.iter().filter_map(|record| match record {
            DatasetRecord::Feature(feature) => Some(feature),
            _ => None,
        })
    }

    pub fn tracks(&self) -> impl Iterator<Item = &TrackRecord> {
        self.records.iter().filter_map(|record| match record {
            DatasetRecord::Track(track) => Some(track),
            _ => None,
        })
    }

    pub fn poses_2d(&self) -> impl Iterator<Item = &Pose2dRecord> {
        self.records.iter().filter_map(|record| match record {
            DatasetRecord::Pose2d(pose) => Some(pose),
            _ => None,
        })
    }

    pub fn poses_3d(&self) -> impl Iterator<Item = &Pose3dRecord> {
        self.records.iter().filter_map(|record| match record {
            DatasetRecord::Pose3d(pose) => Some(pose),
            _ => None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DatasetRecord {
    VideoFrame(VideoFrameRecord),
    AudioFrame(AudioFrameRecord),
    TextSegment(TextSegmentRecord),
    Scene(SceneRecord),
    Cut(CutRecord),
    Observation(ObservationRecord),
    Event(AnalysisEventRecord),
    Metric(MetricRecord),
    Feature(FeatureRecord),
    Track(TrackRecord),
    Pose2d(Pose2dRecord),
    Pose3d(Pose3dRecord),
}

impl DatasetRecord {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::VideoFrame(_) => "video_frame",
            Self::AudioFrame(_) => "audio_frame",
            Self::TextSegment(_) => "text_segment",
            Self::Scene(_) => "scene",
            Self::Cut(_) => "cut",
            Self::Observation(_) => "observation",
            Self::Event(_) => "event",
            Self::Metric(_) => "metric",
            Self::Feature(_) => "feature",
            Self::Track(_) => "track",
            Self::Pose2d(_) => "pose_2d",
            Self::Pose3d(_) => "pose_3d",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TimestampRecord {
    pub pts: i64,
    pub timebase_num: i32,
    pub timebase_den: i32,
    pub seconds: f64,
}

impl From<Timestamp> for TimestampRecord {
    fn from(timestamp: Timestamp) -> Self {
        Self {
            pts: timestamp.pts,
            timebase_num: timestamp.timebase.num,
            timebase_den: timestamp.timebase.den,
            seconds: timestamp.seconds(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FramePositionRecord {
    pub frame_index: u64,
    pub timestamp: TimestampRecord,
}

impl From<FramePosition> for FramePositionRecord {
    fn from(position: FramePosition) -> Self {
        Self {
            frame_index: position.frame_index,
            timestamp: position.timestamp.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PixelFormatRecord {
    Rgb24,
    Bgr24,
}

impl From<PixelFormat> for PixelFormatRecord {
    fn from(format: PixelFormat) -> Self {
        match format {
            PixelFormat::Rgb24 => Self::Rgb24,
            PixelFormat::Bgr24 => Self::Bgr24,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioSampleFormatRecord {
    U8,
    I16,
    I32,
    F32,
}

impl From<AudioSampleFormat> for AudioSampleFormatRecord {
    fn from(format: AudioSampleFormat) -> Self {
        match format {
            AudioSampleFormat::U8 => Self::U8,
            AudioSampleFormat::I16 => Self::I16,
            AudioSampleFormat::I32 => Self::I32,
            AudioSampleFormat::F32 => Self::F32,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundingBoxRecord {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl From<BoundingBox> for BoundingBoxRecord {
    fn from(region: BoundingBox) -> Self {
        Self {
            x: region.x,
            y: region.y,
            width: region.width,
            height: region.height,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationKindRecord {
    Text,
    Face,
    Object,
    Scene,
    Custom(String),
}

impl From<ObservationKind> for ObservationKindRecord {
    fn from(kind: ObservationKind) -> Self {
        match kind {
            ObservationKind::Text => Self::Text,
            ObservationKind::Face => Self::Face,
            ObservationKind::Object => Self::Object,
            ObservationKind::Scene => Self::Scene,
            ObservationKind::Custom(value) => Self::Custom(value),
        }
    }
}

impl From<&ObservationKind> for ObservationKindRecord {
    fn from(kind: &ObservationKind) -> Self {
        kind.clone().into()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoFrameRecord {
    pub stream_id: String,
    pub sequence: u64,
    pub position: FramePositionRecord,
    pub width: u32,
    pub height: u32,
    pub pixel_format: PixelFormatRecord,
    pub stride: usize,
    pub bytes: usize,
}

impl VideoFrameRecord {
    pub fn from_frame(stream_id: impl Into<String>, sequence: u64, frame: &VideoFrame<'_>) -> Self {
        Self {
            stream_id: stream_id.into(),
            sequence,
            position: frame.position.into(),
            width: frame.width,
            height: frame.height,
            pixel_format: frame.pixel_format.into(),
            stride: frame.stride,
            bytes: frame.stride * frame.height as usize,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioFrameRecord {
    pub stream_id: String,
    pub sequence: u64,
    pub timestamp: TimestampRecord,
    pub sample_rate: u32,
    pub channels: u16,
    pub sample_format: AudioSampleFormatRecord,
    pub samples_per_channel: usize,
    pub samples: usize,
    pub bytes: usize,
    pub duration_seconds: f64,
}

impl AudioFrameRecord {
    pub fn from_frame(stream_id: impl Into<String>, sequence: u64, frame: &AudioFrame<'_>) -> Self {
        Self {
            stream_id: stream_id.into(),
            sequence,
            timestamp: frame.timestamp.into(),
            sample_rate: frame.sample_rate,
            channels: frame.channels,
            sample_format: frame.sample_format().into(),
            samples_per_channel: frame.samples_per_channel(),
            samples: frame.sample_count(),
            bytes: audio_buffer_bytes(frame.data),
            duration_seconds: frame.duration_seconds(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextSegmentRecord {
    pub stream_id: String,
    pub segment_index: u64,
    pub timestamp: Option<TimestampRecord>,
    pub text: String,
    pub language: Option<String>,
    pub is_final: bool,
}

impl TextSegmentRecord {
    pub fn from_segment(stream_id: impl Into<String>, segment: &TextSegment<'_>) -> Self {
        Self {
            stream_id: stream_id.into(),
            segment_index: segment.segment_index,
            timestamp: segment.timestamp.map(Into::into),
            text: segment.text.to_string(),
            language: segment.language.map(str::to_string),
            is_final: segment.is_final,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneRecord {
    pub scene_index: u64,
    pub start: FramePositionRecord,
    pub end: FramePositionRecord,
    pub duration_seconds: f64,
}

impl SceneRecord {
    pub fn from_scene(scene_index: u64, scene: &Scene) -> Self {
        let duration_seconds =
            (scene.end.timestamp.seconds() - scene.start.timestamp.seconds()).max(0.0);
        Self {
            scene_index,
            start: scene.start.into(),
            end: scene.end.into(),
            duration_seconds,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CutRecord {
    pub position: FramePositionRecord,
    pub detector: String,
    pub score: Option<f32>,
}

impl CutRecord {
    pub fn from_cut(cut: &Cut) -> Self {
        Self {
            position: cut.position.into(),
            detector: cut.detector.to_string(),
            score: cut.score,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObservationRecord {
    pub timestamp: Option<TimestampRecord>,
    pub frame: Option<FramePositionRecord>,
    pub scene_index: Option<u64>,
    pub analyzer: String,
    pub kind: ObservationKindRecord,
    pub label: Option<String>,
    pub text: Option<String>,
    pub score: Option<f32>,
    pub region: Option<BoundingBoxRecord>,
    pub track_id: Option<String>,
    pub attributes: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeypointRecord2d {
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub score: Option<f32>,
    pub visible: Option<bool>,
}

impl From<Keypoint> for KeypointRecord2d {
    fn from(keypoint: Keypoint) -> Self {
        Self {
            name: keypoint.name,
            x: keypoint.x,
            y: keypoint.y,
            score: keypoint.score,
            visible: keypoint.visible,
        }
    }
}

impl From<&Keypoint> for KeypointRecord2d {
    fn from(keypoint: &Keypoint) -> Self {
        keypoint.clone().into()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeypointRecord3d {
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub score: Option<f32>,
    pub visible: Option<bool>,
}

impl From<Keypoint3d> for KeypointRecord3d {
    fn from(keypoint: Keypoint3d) -> Self {
        Self {
            name: keypoint.name,
            x: keypoint.position.x,
            y: keypoint.position.y,
            z: keypoint.position.z,
            score: keypoint.score,
            visible: keypoint.visible,
        }
    }
}

impl From<&Keypoint3d> for KeypointRecord3d {
    fn from(keypoint: &Keypoint3d) -> Self {
        keypoint.clone().into()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pose2dRecord {
    pub frame: Option<FramePositionRecord>,
    pub analyzer: String,
    pub id: Option<String>,
    pub label: Option<String>,
    pub score: Option<f32>,
    pub region: Option<BoundingBoxRecord>,
    pub keypoints: Vec<KeypointRecord2d>,
    pub attributes: BTreeMap<String, String>,
}

impl Pose2dRecord {
    pub fn from_pose_estimate(
        analyzer: impl Into<String>,
        frame: Option<FramePosition>,
        pose: PoseEstimate,
    ) -> Self {
        Self {
            frame: frame.map(Into::into),
            analyzer: analyzer.into(),
            id: pose.id,
            label: pose.label,
            score: pose.score,
            region: pose.region.map(Into::into),
            keypoints: pose.keypoints.into_iter().map(Into::into).collect(),
            attributes: pose.attributes,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pose3dRecord {
    pub frame: Option<FramePositionRecord>,
    pub analyzer: String,
    pub id: Option<String>,
    pub label: Option<String>,
    pub score: Option<f32>,
    pub keypoints: Vec<KeypointRecord3d>,
    pub attributes: BTreeMap<String, String>,
}

impl Pose3dRecord {
    pub fn from_pose_3d_estimate(
        analyzer: impl Into<String>,
        frame: Option<FramePosition>,
        pose: Pose3dEstimate,
    ) -> Self {
        Self {
            frame: frame.map(Into::into),
            analyzer: analyzer.into(),
            id: pose.id,
            label: pose.label,
            score: pose.score,
            keypoints: pose.keypoints.into_iter().map(Into::into).collect(),
            attributes: pose.attributes,
        }
    }
}

impl ObservationRecord {
    pub fn from_observation(observation: Observation) -> Self {
        Self {
            timestamp: observation.timestamp.map(Into::into),
            frame: observation.frame.map(Into::into),
            scene_index: observation.scene_index,
            analyzer: observation.analyzer,
            kind: observation.kind.into(),
            label: observation.label,
            text: observation.text,
            score: observation.score,
            region: observation.region.map(Into::into),
            track_id: observation.track_id,
            attributes: observation.attributes,
        }
    }

    pub fn from_observation_ref(observation: &Observation) -> Self {
        Self {
            timestamp: observation.timestamp.map(Into::into),
            frame: observation.frame.map(Into::into),
            scene_index: observation.scene_index,
            analyzer: observation.analyzer.clone(),
            kind: (&observation.kind).into(),
            label: observation.label.clone(),
            text: observation.text.clone(),
            score: observation.score,
            region: observation.region.map(Into::into),
            track_id: observation.track_id.clone(),
            attributes: observation.attributes.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalysisEventRecord {
    pub timestamp: Option<TimestampRecord>,
    pub analyzer: String,
    pub label: String,
    pub score: Option<f32>,
}

impl AnalysisEventRecord {
    pub fn from_event(event: AnalysisEvent) -> Self {
        Self {
            timestamp: event.timestamp.map(Into::into),
            analyzer: event.analyzer,
            label: event.label,
            score: event.score,
        }
    }

    pub fn from_event_ref(event: &AnalysisEvent) -> Self {
        Self {
            timestamp: event.timestamp.map(Into::into),
            analyzer: event.analyzer.clone(),
            label: event.label.clone(),
            score: event.score,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetricRecord {
    pub frame_index: u64,
    pub key: String,
    pub value: f64,
}

impl MetricRecord {
    pub fn new(frame_index: u64, key: impl Into<String>, value: f64) -> Self {
        Self {
            frame_index,
            key: key.into(),
            value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeatureRecord {
    pub name: String,
    pub scope: Option<String>,
    pub timestamp: Option<TimestampRecord>,
    pub frame_index: Option<u64>,
    pub scene_index: Option<u64>,
    pub track_id: Option<String>,
    pub value: FeatureValue,
    pub attributes: BTreeMap<String, String>,
}

impl FeatureRecord {
    pub fn new(name: impl Into<String>, value: FeatureValue) -> Self {
        Self {
            name: name.into(),
            scope: None,
            timestamp: None,
            frame_index: None,
            scene_index: None,
            track_id: None,
            value,
            attributes: BTreeMap::new(),
        }
    }

    pub fn number(name: impl Into<String>, value: f64) -> Self {
        Self::new(name, FeatureValue::Number(value))
    }

    pub fn integer(name: impl Into<String>, value: i64) -> Self {
        Self::new(name, FeatureValue::Integer(value))
    }

    pub fn histogram(name: impl Into<String>, value: BTreeMap<String, u64>) -> Self {
        Self::new(name, FeatureValue::Histogram(value))
    }

    pub fn scope(mut self, scope: impl Into<String>) -> Self {
        self.scope = Some(scope.into());
        self
    }

    pub fn timestamp(mut self, timestamp: Timestamp) -> Self {
        self.timestamp = Some(timestamp.into());
        self
    }

    pub fn frame_index(mut self, frame_index: u64) -> Self {
        self.frame_index = Some(frame_index);
        self
    }

    pub fn scene_index(mut self, scene_index: u64) -> Self {
        self.scene_index = Some(scene_index);
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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrackRecord {
    pub track_id: String,
    pub kind: Option<ObservationKindRecord>,
    pub label: Option<String>,
    pub first_frame: Option<u64>,
    pub last_frame: Option<u64>,
    pub first_timestamp: Option<TimestampRecord>,
    pub last_timestamp: Option<TimestampRecord>,
    pub observations: u64,
    pub attributes: BTreeMap<String, String>,
}

impl TrackRecord {
    pub fn new(track_id: impl Into<String>) -> Self {
        Self {
            track_id: track_id.into(),
            kind: None,
            label: None,
            first_frame: None,
            last_frame: None,
            first_timestamp: None,
            last_timestamp: None,
            observations: 0,
            attributes: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeatureValue {
    Number(f64),
    Integer(i64),
    Boolean(bool),
    Text(String),
    Vector(Vec<f32>),
    Histogram(BTreeMap<String, u64>),
}

pub fn metric_records(metrics: &MetricsStore) -> impl Iterator<Item = MetricRecord> + '_ {
    metrics.rows().iter().flat_map(|(frame_index, row)| {
        row.iter()
            .map(move |(key, value)| MetricRecord::new(*frame_index, key.clone(), *value))
    })
}

fn audio_buffer_bytes(data: &AudioBuffer) -> usize {
    match data {
        AudioBuffer::U8(values) => mem::size_of_val(values.as_slice()),
        AudioBuffer::I16(values) => mem::size_of_val(values.as_slice()),
        AudioBuffer::I32(values) => mem::size_of_val(values.as_slice()),
        AudioBuffer::F32(values) => mem::size_of_val(values.as_slice()),
    }
}

#[cfg(test)]
mod tests {
    use num_rational::Rational64;
    use video_analysis_core::{
        AudioBuffer, AudioFrame, BoundingBox, Cut, DetectionResult, FramePosition, MetricsSink,
        MetricsStore, Observation, ObservationKind, PixelFormat, Scene, Timebase, Timestamp,
        VideoFrame,
    };
    use video_analysis_posture::{Keypoint, Keypoint3d, Pose3dEstimate, PoseEstimate};

    use super::*;

    fn position(frame_index: u64) -> FramePosition {
        FramePosition::from_frame_index(frame_index, Rational64::new(30, 1))
    }

    #[test]
    fn converts_core_mirror_types() {
        let timestamp = Timestamp::new(3, Timebase::new(1, 2));
        let timestamp_record = TimestampRecord::from(timestamp);
        assert_eq!(timestamp_record.pts, 3);
        assert_eq!(timestamp_record.seconds, 1.5);

        let frame = FramePositionRecord::from(position(9));
        assert_eq!(frame.frame_index, 9);
        assert_eq!(
            PixelFormatRecord::from(PixelFormat::Rgb24),
            PixelFormatRecord::Rgb24
        );
        assert_eq!(
            AudioSampleFormatRecord::from(AudioSampleFormat::F32),
            AudioSampleFormatRecord::F32
        );
        assert_eq!(
            BoundingBoxRecord::from(BoundingBox::new(1, 2, 3, 4).unwrap()).width,
            3
        );
        assert_eq!(
            ObservationKindRecord::from(ObservationKind::Custom("pose".to_string())),
            ObservationKindRecord::Custom("pose".to_string())
        );
    }

    #[test]
    fn records_media_without_payload_bytes() {
        let pixels = [1_u8; 12];
        let frame = VideoFrame::packed(position(1), 2, 2, PixelFormat::Rgb24, &pixels, 6).unwrap();
        let record = VideoFrameRecord::from_frame("video:0", 1, &frame);
        assert_eq!(record.bytes, 12);
        assert_eq!(record.width, 2);

        let audio_buffer = AudioBuffer::F32(vec![0.0, 1.0, 0.5, -0.5]);
        let audio = AudioFrame::new(
            Timestamp::new(0, Timebase::new(1, 48_000)),
            48_000,
            2,
            &audio_buffer,
        )
        .unwrap();
        let record = AudioFrameRecord::from_frame("audio:0", 0, &audio);
        assert_eq!(record.samples_per_channel, 2);
        assert_eq!(record.bytes, 16);
    }

    #[test]
    fn extends_detection_result_into_scene_cut_metric_records() {
        let mut metrics = MetricsStore::default();
        metrics.set_metric(4, "content", 27.5);
        let result = DetectionResult {
            scenes: vec![Scene {
                start: position(0),
                end: position(10),
            }],
            cuts: vec![Cut {
                position: position(10),
                detector: "content",
                score: Some(0.9),
            }],
            metrics,
            frames_processed: 11,
        };

        let mut dataset = AnalysisDataset::empty();
        dataset.extend_detection_result(&result);

        assert_eq!(dataset.scenes().count(), 1);
        assert_eq!(
            dataset
                .records
                .iter()
                .filter(|record| matches!(record, DatasetRecord::Cut(_)))
                .count(),
            1
        );
        assert_eq!(
            dataset
                .records
                .iter()
                .filter(|record| matches!(record, DatasetRecord::Metric(_)))
                .count(),
            1
        );
    }

    #[test]
    fn observations_and_events_keep_fields() {
        let observation = Observation::new("objects", ObservationKind::Object)
            .at_frame(position(3))
            .in_scene(1)
            .label("person")
            .score(0.8)
            .region(BoundingBox::new(1, 2, 3, 4).unwrap())
            .track_id("t1")
            .attribute("source", "test");
        let event = AnalysisEvent::new("audio", "speech")
            .at_timestamp(Timestamp::new(2, Timebase::new(1, 1)))
            .score(0.7);

        let mut dataset = AnalysisDataset::empty();
        dataset.extend_observations([observation]);
        dataset.extend_events([event]);

        let observation = dataset.observations().next().unwrap();
        assert_eq!(observation.label.as_deref(), Some("person"));
        assert_eq!(observation.track_id.as_deref(), Some("t1"));
        assert_eq!(observation.attributes["source"], "test");
        assert_eq!(dataset.events().next().unwrap().label, "speech");
    }

    #[test]
    fn serializes_and_deserializes_representative_record() {
        let record = DatasetRecord::Feature(
            FeatureRecord::histogram(
                "labels",
                BTreeMap::from([("person".to_string(), 2), ("car".to_string(), 1)]),
            )
            .scene_index(4),
        );

        let json = serde_json::to_string(&record).unwrap();
        let round_trip: DatasetRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(round_trip, record);
    }

    #[test]
    fn retains_structured_pose_records() {
        let mut dataset = AnalysisDataset::empty();
        dataset.extend_pose_estimates(
            "pose2d",
            Some(position(1)),
            [PoseEstimate::new([Keypoint::new("nose", 1.0, 2.0).unwrap()]).unwrap()],
        );
        dataset.extend_pose_3d_estimates(
            "pose3d",
            Some(position(2)),
            [Pose3dEstimate::new([Keypoint3d::new(
                "nose",
                three_d_processing_core::Point3::new(1.0, 2.0, 3.0),
            )
            .unwrap()])
            .unwrap()],
        );

        assert_eq!(dataset.poses_2d().count(), 1);
        assert_eq!(dataset.poses_3d().count(), 1);
    }
}
