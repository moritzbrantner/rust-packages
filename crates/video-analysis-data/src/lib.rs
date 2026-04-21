use std::collections::BTreeMap;
use std::mem;

use video_analysis_core::{
    AudioBuffer, AudioFrame, DetectError, Result, TextSegment, Timestamp, VideoFrame,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DataStreamKind {
    Video,
    Audio,
    Text,
    Number,
    Vector,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BucketMode {
    FixedDuration { seconds: f64 },
    RecordCount { records: u64 },
    ByteSize { bytes: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BucketConfig {
    pub mode: BucketMode,
    pub max_vector_dimensions: usize,
}

impl BucketConfig {
    pub fn fixed_duration_seconds(seconds: f64) -> Result<Self> {
        let config = Self {
            mode: BucketMode::FixedDuration { seconds },
            ..Self::default()
        };
        config.validate()?;
        Ok(config)
    }

    pub fn record_count(records: u64) -> Result<Self> {
        let config = Self {
            mode: BucketMode::RecordCount { records },
            ..Self::default()
        };
        config.validate()?;
        Ok(config)
    }

    pub fn byte_size(bytes: u64) -> Result<Self> {
        let config = Self {
            mode: BucketMode::ByteSize { bytes },
            ..Self::default()
        };
        config.validate()?;
        Ok(config)
    }

    pub fn max_vector_dimensions(mut self, dimensions: usize) -> Self {
        self.max_vector_dimensions = dimensions;
        self
    }

    fn validate(self) -> Result<()> {
        match self.mode {
            BucketMode::FixedDuration { seconds } if seconds <= 0.0 || !seconds.is_finite() => {
                Err(DetectError::InvalidArgument(
                    "bucket duration must be a finite positive value".to_string(),
                ))
            }
            BucketMode::RecordCount { records } if records == 0 => Err(
                DetectError::InvalidArgument("bucket record count must be positive".to_string()),
            ),
            BucketMode::ByteSize { bytes } if bytes == 0 => Err(DetectError::InvalidArgument(
                "bucket byte size must be positive".to_string(),
            )),
            _ => Ok(()),
        }
    }
}

impl Default for BucketConfig {
    fn default() -> Self {
        Self {
            mode: BucketMode::RecordCount { records: 1_000 },
            max_vector_dimensions: 256,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DataPayload<'a> {
    Video {
        width: u32,
        height: u32,
        bytes: usize,
    },
    Audio {
        sample_rate: u32,
        channels: u16,
        samples_per_channel: usize,
        bytes: usize,
    },
    Text {
        text: &'a str,
    },
    Number(f64),
    Vector(&'a [f32]),
    Custom {
        label: &'a str,
        bytes: usize,
    },
}

impl DataPayload<'_> {
    pub fn estimated_bytes(&self) -> usize {
        match self {
            Self::Video { bytes, .. } => *bytes,
            Self::Audio { bytes, .. } => *bytes,
            Self::Text { text } => text.len(),
            Self::Number(_) => mem::size_of::<f64>(),
            Self::Vector(values) => mem::size_of_val(*values),
            Self::Custom { bytes, .. } => *bytes,
        }
    }

    pub fn kind(&self) -> DataStreamKind {
        match self {
            Self::Video { .. } => DataStreamKind::Video,
            Self::Audio { .. } => DataStreamKind::Audio,
            Self::Text { .. } => DataStreamKind::Text,
            Self::Number(_) => DataStreamKind::Number,
            Self::Vector(_) => DataStreamKind::Vector,
            Self::Custom { .. } => DataStreamKind::Custom,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DataRecord<'a> {
    pub stream_id: &'a str,
    pub sequence: u64,
    pub timestamp: Option<Timestamp>,
    pub payload: DataPayload<'a>,
}

impl<'a> DataRecord<'a> {
    pub fn video_frame(stream_id: &'a str, sequence: u64, frame: &VideoFrame<'_>) -> Self {
        Self {
            stream_id,
            sequence,
            timestamp: Some(frame.position.timestamp),
            payload: DataPayload::Video {
                width: frame.width,
                height: frame.height,
                bytes: frame.stride * frame.height as usize,
            },
        }
    }

    pub fn audio_frame(stream_id: &'a str, sequence: u64, frame: &AudioFrame<'_>) -> Self {
        Self {
            stream_id,
            sequence,
            timestamp: Some(frame.timestamp),
            payload: DataPayload::Audio {
                sample_rate: frame.sample_rate,
                channels: frame.channels,
                samples_per_channel: frame.samples_per_channel(),
                bytes: audio_buffer_bytes(frame.data),
            },
        }
    }

    pub fn text_segment(stream_id: &'a str, segment: &TextSegment<'a>) -> Self {
        Self {
            stream_id,
            sequence: segment.segment_index,
            timestamp: segment.timestamp,
            payload: DataPayload::Text { text: segment.text },
        }
    }

    pub fn number(
        stream_id: &'a str,
        sequence: u64,
        timestamp: Option<Timestamp>,
        value: f64,
    ) -> Self {
        Self {
            stream_id,
            sequence,
            timestamp,
            payload: DataPayload::Number(value),
        }
    }

    pub fn vector(
        stream_id: &'a str,
        sequence: u64,
        timestamp: Option<Timestamp>,
        values: &'a [f32],
    ) -> Self {
        Self {
            stream_id,
            sequence,
            timestamp,
            payload: DataPayload::Vector(values),
        }
    }

    pub fn custom(
        stream_id: &'a str,
        sequence: u64,
        timestamp: Option<Timestamp>,
        label: &'a str,
        bytes: usize,
    ) -> Self {
        Self {
            stream_id,
            sequence,
            timestamp,
            payload: DataPayload::Custom { label, bytes },
        }
    }

    pub fn kind(&self) -> DataStreamKind {
        self.payload.kind()
    }

    pub fn estimated_bytes(&self) -> usize {
        self.payload.estimated_bytes()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DataBucket {
    pub bucket_index: u64,
    pub mode: BucketMode,
    pub records: u64,
    pub estimated_bytes: u64,
    pub start_timestamp: Option<Timestamp>,
    pub end_timestamp: Option<Timestamp>,
    pub streams: BTreeMap<String, StreamSummary>,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct StreamSummary {
    pub records: u64,
    pub estimated_bytes: u64,
    pub first_sequence: Option<u64>,
    pub last_sequence: Option<u64>,
    pub first_timestamp: Option<Timestamp>,
    pub last_timestamp: Option<Timestamp>,
    pub payload_counts: BTreeMap<DataStreamKind, u64>,
    pub video: VideoSummary,
    pub audio: AudioSummary,
    pub text: TextSummary,
    pub numeric: NumericSummary,
    pub vector: VectorSummary,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct VideoSummary {
    pub frames: u64,
    pub pixels: u64,
    pub max_width: u32,
    pub max_height: u32,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct AudioSummary {
    pub frames: u64,
    pub samples_per_channel: u64,
    pub max_channels: u16,
    pub sample_rates: BTreeMap<u32, u64>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TextSummary {
    pub segments: u64,
    pub bytes: u64,
    pub chars: u64,
    pub words: u64,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct NumericSummary {
    pub count: u64,
    pub finite_count: u64,
    pub non_finite_count: u64,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub mean: Option<f64>,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct VectorSummary {
    pub count: u64,
    pub finite_count: u64,
    pub non_finite_count: u64,
    pub dimensions: Option<usize>,
    pub mismatched_dimensions: u64,
    pub tracked_dimensions: usize,
    pub tracked_mean_count: u64,
    pub mean: Vec<f64>,
    pub min_norm: Option<f64>,
    pub max_norm: Option<f64>,
    pub mean_norm: Option<f64>,
}

pub struct BucketAggregator {
    config: BucketConfig,
    active: Option<BucketAccumulator>,
    next_bucket_index: u64,
}

impl BucketAggregator {
    pub fn new(config: BucketConfig) -> Result<Self> {
        config.validate()?;
        Ok(Self {
            config,
            active: None,
            next_bucket_index: 0,
        })
    }

    pub fn push(&mut self, record: DataRecord<'_>) -> Result<Vec<DataBucket>> {
        let mut completed = Vec::new();

        if self.starts_new_bucket(&record)? {
            if let Some(active) = self.active.take() {
                completed.push(active.finish());
            }
        }

        if self.active.is_none() {
            let bucket_index = self.bucket_index_for(&record)?;
            self.active = Some(BucketAccumulator::new(bucket_index, self.config.mode));
            if !matches!(self.config.mode, BucketMode::FixedDuration { .. }) {
                self.next_bucket_index += 1;
            }
        }

        let active = self.active.as_mut().expect("active bucket exists");
        active.push(record, self.config.max_vector_dimensions);

        if self.active_is_complete() {
            let active = self.active.take().expect("active bucket exists");
            completed.push(active.finish());
        }

        Ok(completed)
    }

    pub fn finish(&mut self) -> Option<DataBucket> {
        self.active.take().map(BucketAccumulator::finish)
    }

    pub fn config(&self) -> BucketConfig {
        self.config
    }

    fn starts_new_bucket(&self, record: &DataRecord<'_>) -> Result<bool> {
        let Some(active) = &self.active else {
            return Ok(false);
        };

        match self.config.mode {
            BucketMode::FixedDuration { .. } => {
                let next_index = self.bucket_index_for(record)?;
                if next_index < active.bucket_index {
                    return Err(DetectError::InvalidArgument(
                        "records must be ordered by bucket timestamp".to_string(),
                    ));
                }
                Ok(next_index > active.bucket_index)
            }
            BucketMode::RecordCount { records } => Ok(active.records >= records),
            BucketMode::ByteSize { bytes } => Ok(active.records > 0
                && active.estimated_bytes + record.estimated_bytes() as u64 > bytes),
        }
    }

    fn active_is_complete(&self) -> bool {
        let Some(active) = &self.active else {
            return false;
        };

        match self.config.mode {
            BucketMode::FixedDuration { .. } => false,
            BucketMode::RecordCount { records } => active.records >= records,
            BucketMode::ByteSize { bytes } => active.estimated_bytes >= bytes,
        }
    }

    fn bucket_index_for(&self, record: &DataRecord<'_>) -> Result<u64> {
        match self.config.mode {
            BucketMode::FixedDuration { seconds } => {
                let timestamp = record.timestamp.ok_or_else(|| {
                    DetectError::InvalidArgument(
                        "fixed-duration buckets require record timestamps".to_string(),
                    )
                })?;
                let record_seconds = timestamp.seconds();
                if record_seconds < 0.0 || !record_seconds.is_finite() {
                    return Err(DetectError::InvalidArgument(
                        "record timestamp must be finite and non-negative".to_string(),
                    ));
                }
                Ok((record_seconds / seconds).floor() as u64)
            }
            BucketMode::RecordCount { .. } | BucketMode::ByteSize { .. } => {
                Ok(self.next_bucket_index)
            }
        }
    }
}

#[derive(Debug, Clone)]
struct BucketAccumulator {
    bucket_index: u64,
    mode: BucketMode,
    records: u64,
    estimated_bytes: u64,
    start_timestamp: Option<Timestamp>,
    end_timestamp: Option<Timestamp>,
    streams: BTreeMap<String, StreamSummary>,
}

impl BucketAccumulator {
    fn new(bucket_index: u64, mode: BucketMode) -> Self {
        Self {
            bucket_index,
            mode,
            records: 0,
            estimated_bytes: 0,
            start_timestamp: None,
            end_timestamp: None,
            streams: BTreeMap::new(),
        }
    }

    fn push(&mut self, record: DataRecord<'_>, max_vector_dimensions: usize) {
        let estimated_bytes = record.estimated_bytes() as u64;
        self.records += 1;
        self.estimated_bytes += estimated_bytes;
        update_first_timestamp(&mut self.start_timestamp, record.timestamp);
        update_last_timestamp(&mut self.end_timestamp, record.timestamp);

        let stream = self
            .streams
            .entry(record.stream_id.to_string())
            .or_default();
        stream.push(record, estimated_bytes, max_vector_dimensions);
    }

    fn finish(self) -> DataBucket {
        DataBucket {
            bucket_index: self.bucket_index,
            mode: self.mode,
            records: self.records,
            estimated_bytes: self.estimated_bytes,
            start_timestamp: self.start_timestamp,
            end_timestamp: self.end_timestamp,
            streams: self.streams,
        }
    }
}

impl StreamSummary {
    fn push(&mut self, record: DataRecord<'_>, estimated_bytes: u64, max_vector_dimensions: usize) {
        self.records += 1;
        self.estimated_bytes += estimated_bytes;
        self.first_sequence.get_or_insert(record.sequence);
        self.last_sequence = Some(record.sequence);
        update_first_timestamp(&mut self.first_timestamp, record.timestamp);
        update_last_timestamp(&mut self.last_timestamp, record.timestamp);
        *self.payload_counts.entry(record.kind()).or_default() += 1;

        match record.payload {
            DataPayload::Video { width, height, .. } => {
                self.video.frames += 1;
                self.video.pixels += width as u64 * height as u64;
                self.video.max_width = self.video.max_width.max(width);
                self.video.max_height = self.video.max_height.max(height);
            }
            DataPayload::Audio {
                sample_rate,
                channels,
                samples_per_channel,
                ..
            } => {
                self.audio.frames += 1;
                self.audio.samples_per_channel += samples_per_channel as u64;
                self.audio.max_channels = self.audio.max_channels.max(channels);
                *self.audio.sample_rates.entry(sample_rate).or_default() += 1;
            }
            DataPayload::Text { text } => {
                self.text.segments += 1;
                self.text.bytes += text.len() as u64;
                self.text.chars += text.chars().count() as u64;
                self.text.words += text.split_whitespace().count() as u64;
            }
            DataPayload::Number(value) => self.numeric.push(value),
            DataPayload::Vector(values) => self.vector.push(values, max_vector_dimensions),
            DataPayload::Custom { .. } => {}
        }
    }
}

impl NumericSummary {
    fn push(&mut self, value: f64) {
        self.count += 1;
        if !value.is_finite() {
            self.non_finite_count += 1;
            return;
        }

        self.finite_count += 1;
        self.min = Some(self.min.map_or(value, |min| min.min(value)));
        self.max = Some(self.max.map_or(value, |max| max.max(value)));
        let previous = self.mean.unwrap_or(value);
        self.mean = Some(previous + (value - previous) / self.finite_count as f64);
    }
}

impl VectorSummary {
    fn push(&mut self, values: &[f32], max_vector_dimensions: usize) {
        self.count += 1;
        match self.dimensions {
            Some(dimensions) if dimensions != values.len() => self.mismatched_dimensions += 1,
            None => self.dimensions = Some(values.len()),
            _ => {}
        }

        let mut norm_squared = 0.0;
        let mut all_finite = true;
        for value in values {
            let value = *value as f64;
            if !value.is_finite() {
                all_finite = false;
                continue;
            }
            norm_squared += value * value;
        }

        if !all_finite {
            self.non_finite_count += 1;
            return;
        }

        self.finite_count += 1;
        let norm = norm_squared.sqrt();
        self.min_norm = Some(self.min_norm.map_or(norm, |min| min.min(norm)));
        self.max_norm = Some(self.max_norm.map_or(norm, |max| max.max(norm)));
        let previous_norm = self.mean_norm.unwrap_or(norm);
        self.mean_norm = Some(previous_norm + (norm - previous_norm) / self.finite_count as f64);

        if values.len() > max_vector_dimensions {
            return;
        }

        if self.mean.is_empty() {
            self.tracked_dimensions = values.len();
            self.mean.resize(values.len(), 0.0);
        }

        if self.tracked_dimensions != values.len() {
            return;
        }

        self.tracked_mean_count += 1;
        for (mean, value) in self.mean.iter_mut().zip(values) {
            let value = *value as f64;
            *mean += (value - *mean) / self.tracked_mean_count as f64;
        }
    }
}

fn audio_buffer_bytes(data: &AudioBuffer) -> usize {
    match data {
        AudioBuffer::U8(values) => mem::size_of_val(values.as_slice()),
        AudioBuffer::I16(values) => mem::size_of_val(values.as_slice()),
        AudioBuffer::I32(values) => mem::size_of_val(values.as_slice()),
        AudioBuffer::F32(values) => mem::size_of_val(values.as_slice()),
    }
}

fn update_first_timestamp(target: &mut Option<Timestamp>, candidate: Option<Timestamp>) {
    let Some(candidate) = candidate else {
        return;
    };
    match target {
        Some(current) if current.seconds() <= candidate.seconds() => {}
        _ => *target = Some(candidate),
    }
}

fn update_last_timestamp(target: &mut Option<Timestamp>, candidate: Option<Timestamp>) {
    let Some(candidate) = candidate else {
        return;
    };
    match target {
        Some(current) if current.seconds() >= candidate.seconds() => {}
        _ => *target = Some(candidate),
    }
}

#[cfg(test)]
mod tests {
    use video_analysis_core::{
        AudioBuffer, AudioFrame, FramePosition, OwnedTextSegment, PixelFormat, Timebase, VideoFrame,
    };

    use super::*;

    fn ts(seconds: i64) -> Timestamp {
        Timestamp::new(seconds, Timebase::new(1, 1))
    }

    #[test]
    fn count_buckets_aggregate_numbers_and_vectors() {
        let mut aggregator = BucketAggregator::new(BucketConfig::record_count(3).unwrap()).unwrap();

        assert!(aggregator
            .push(DataRecord::number("score", 0, Some(ts(0)), 1.0))
            .unwrap()
            .is_empty());
        assert!(aggregator
            .push(DataRecord::number("score", 1, Some(ts(1)), 3.0))
            .unwrap()
            .is_empty());

        let completed = aggregator
            .push(DataRecord::vector("embedding", 2, Some(ts(2)), &[1.0, 3.0]))
            .unwrap();

        assert_eq!(completed.len(), 1);
        let bucket = &completed[0];
        assert_eq!(bucket.records, 3);
        assert_eq!(bucket.streams["score"].numeric.mean, Some(2.0));
        assert_eq!(bucket.streams["embedding"].vector.mean, vec![1.0, 3.0]);
        assert!(aggregator.finish().is_none());
    }

    #[test]
    fn fixed_duration_buckets_flush_when_time_window_changes() {
        let config = BucketConfig::fixed_duration_seconds(2.0).unwrap();
        let mut aggregator = BucketAggregator::new(config).unwrap();

        assert!(aggregator
            .push(DataRecord::number("metric", 0, Some(ts(0)), 2.0))
            .unwrap()
            .is_empty());
        assert!(aggregator
            .push(DataRecord::number("metric", 1, Some(ts(1)), 4.0))
            .unwrap()
            .is_empty());

        let completed = aggregator
            .push(DataRecord::number("metric", 2, Some(ts(2)), 6.0))
            .unwrap();

        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].bucket_index, 0);
        assert_eq!(completed[0].records, 2);
        assert_eq!(completed[0].streams["metric"].numeric.mean, Some(3.0));

        let tail = aggregator.finish().unwrap();
        assert_eq!(tail.bucket_index, 1);
        assert_eq!(tail.records, 1);
    }

    #[test]
    fn byte_buckets_split_before_exceeding_target_size() {
        let mut aggregator = BucketAggregator::new(BucketConfig::byte_size(12).unwrap()).unwrap();

        assert!(aggregator
            .push(DataRecord::custom("blob", 0, None, "small", 8))
            .unwrap()
            .is_empty());

        let completed = aggregator
            .push(DataRecord::custom("blob", 1, None, "medium", 8))
            .unwrap();
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].records, 1);
        assert_eq!(completed[0].estimated_bytes, 8);

        let completed = aggregator
            .push(DataRecord::custom("blob", 2, None, "large", 16))
            .unwrap();
        assert_eq!(completed.len(), 2);
        assert_eq!(completed[0].records, 1);
        assert_eq!(completed[1].estimated_bytes, 16);
        assert!(aggregator.finish().is_none());
    }

    #[test]
    fn core_sample_adapters_track_payload_summaries_without_copying_data() {
        let position = FramePosition {
            frame_index: 4,
            timestamp: ts(4),
        };
        let pixels = [0_u8; 12];
        let frame = VideoFrame::packed(position, 2, 2, PixelFormat::Rgb24, &pixels, 6).unwrap();
        let audio_buffer = AudioBuffer::F32(vec![0.0, 1.0, 0.5, -0.5]);
        let audio = AudioFrame::new(ts(5), 48_000, 2, &audio_buffer).unwrap();
        let text = OwnedTextSegment::new(7, "hello large world").timestamp(ts(6));

        let mut aggregator = BucketAggregator::new(BucketConfig::record_count(3).unwrap()).unwrap();
        assert!(aggregator
            .push(DataRecord::video_frame("video:0", 4, &frame))
            .unwrap()
            .is_empty());
        assert!(aggregator
            .push(DataRecord::audio_frame("audio:0", 0, &audio))
            .unwrap()
            .is_empty());
        let completed = aggregator
            .push(DataRecord::text_segment("text:0", &text.as_segment()))
            .unwrap();

        let bucket = &completed[0];
        assert_eq!(bucket.streams["video:0"].video.pixels, 4);
        assert_eq!(bucket.streams["audio:0"].audio.samples_per_channel, 2);
        assert_eq!(bucket.streams["text:0"].text.words, 3);
        assert_eq!(
            bucket.estimated_bytes,
            12 + 16 + "hello large world".len() as u64
        );
    }

    #[test]
    fn vectors_over_dimension_limit_keep_norms_but_not_mean_vector() {
        let config = BucketConfig::record_count(1)
            .unwrap()
            .max_vector_dimensions(2);
        let mut aggregator = BucketAggregator::new(config).unwrap();

        let completed = aggregator
            .push(DataRecord::vector("embedding", 0, None, &[1.0, 2.0, 2.0]))
            .unwrap();

        let vector = &completed[0].streams["embedding"].vector;
        assert_eq!(vector.dimensions, Some(3));
        assert_eq!(vector.mean, Vec::<f64>::new());
        assert_eq!(vector.mean_norm, Some(3.0));
    }
}
