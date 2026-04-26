#![doc = include_str!("../README.md")]

pub use math_signal_core::{
    BiquadCoefficients, BiquadDesign, FirKernel1d, FrameStride, InterpolationMode, ResampleRatio,
    ResampleSpec, SampleRate, WindowFunction, WindowSpec,
};

use tensor_data::{F32Tensor, F32TensorView};
use video_analysis_core::{AudioBuffer, AudioFrame, DetectError, Result, Timebase, Timestamp};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioFormatSpec {
    pub sample_rate: u32,
    pub channels: u16,
    pub frame_samples: Option<usize>,
}

impl AudioFormatSpec {
    pub fn new(sample_rate: u32, channels: u16) -> Result<Self> {
        let spec = Self {
            sample_rate,
            channels,
            frame_samples: None,
        };
        spec.validate()?;
        Ok(spec)
    }

    pub fn frame_samples(mut self, frame_samples: usize) -> Result<Self> {
        self.frame_samples = Some(frame_samples);
        self.validate()?;
        Ok(self)
    }

    pub fn validate(&self) -> Result<()> {
        if self.sample_rate == 0 || self.channels == 0 {
            return Err(DetectError::InvalidAudioFormat {
                sample_rate: self.sample_rate,
                channels: self.channels,
            });
        }
        if self.frame_samples == Some(0) {
            return Err(DetectError::InvalidArgument(
                "frame_samples must be greater than zero".to_string(),
            ));
        }
        Ok(())
    }

    pub fn duration_seconds(&self, samples_per_channel: usize) -> Result<f64> {
        self.validate()?;
        Ok(samples_per_channel as f64 / self.sample_rate as f64)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelMix {
    Average,
    First,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MonoSamples {
    pub timestamp: Timestamp,
    pub sample_rate: u32,
    pub samples: Vec<f32>,
}

impl MonoSamples {
    pub fn duration_seconds(&self) -> f64 {
        if self.sample_rate == 0 {
            return 0.0;
        }
        self.samples.len() as f64 / self.sample_rate as f64
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AudioWaveformBatchView<'a> {
    pub sample_rate: u32,
    tensor: F32TensorView<'a>,
}

impl<'a> AudioWaveformBatchView<'a> {
    pub fn new(sample_rate: u32, tensor: F32TensorView<'a>) -> Result<Self> {
        let batch = Self {
            sample_rate,
            tensor,
        };
        batch.validate()?;
        Ok(batch)
    }

    pub fn from_dims(
        sample_rate: u32,
        dims: impl Into<Vec<usize>>,
        values: &'a [f32],
    ) -> Result<Self> {
        Self::new(sample_rate, F32TensorView::from_dims(dims, values)?)
    }

    pub fn tensor(&self) -> &F32TensorView<'a> {
        &self.tensor
    }

    pub fn batch_size(&self) -> usize {
        self.tensor.shape().dimensions()[0]
    }

    pub fn channel_count(&self) -> usize {
        self.tensor.shape().dimensions()[1]
    }

    pub fn time_steps(&self) -> usize {
        self.tensor.shape().dimensions()[2]
    }

    pub fn duration_seconds(&self) -> f64 {
        self.time_steps() as f64 / self.sample_rate as f64
    }

    pub fn waveform(&self, batch_index: usize, channel_index: usize) -> Result<&'a [f32]> {
        if batch_index >= self.batch_size() || channel_index >= self.channel_count() {
            return Err(DetectError::InvalidArgument(format!(
                "waveform index [{batch_index}, {channel_index}] is out of bounds for [{}, {}]",
                self.batch_size(),
                self.channel_count()
            )));
        }
        let time_steps = self.time_steps();
        let start = batch_index * self.channel_count() * time_steps + channel_index * time_steps;
        Ok(&self.tensor.values()[start..start + time_steps])
    }

    fn validate(&self) -> Result<()> {
        AudioFormatSpec::new(self.sample_rate, 1)?;
        self.tensor.validate()?;
        if self.tensor.shape().rank() != 3 {
            return Err(DetectError::InvalidArgument(
                "audio waveform batches must use rank 3 [B,C,T] tensors".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OwnedAudioWaveformBatch {
    pub sample_rate: u32,
    tensor: F32Tensor,
}

impl OwnedAudioWaveformBatch {
    pub fn new(sample_rate: u32, tensor: F32Tensor) -> Result<Self> {
        let batch = Self {
            sample_rate,
            tensor,
        };
        batch.as_view()?;
        Ok(batch)
    }

    pub fn from_audio_frames(frames: &[video_analysis_core::OwnedAudioFrame]) -> Result<Self> {
        if frames.is_empty() {
            return Err(DetectError::InvalidArgument(
                "audio waveform batches must contain at least one frame".to_string(),
            ));
        }
        let first = &frames[0];
        let sample_rate = first.sample_rate;
        let channels = first.channels as usize;
        let time_steps = first.samples_per_channel();
        let mut values = Vec::with_capacity(frames.len() * channels * time_steps);

        for frame in frames {
            if frame.sample_rate != sample_rate
                || frame.channels as usize != channels
                || frame.samples_per_channel() != time_steps
            {
                return Err(DetectError::InvalidArgument(
                    "all audio frames in a batch must share sample rate, channel count, and samples per channel"
                        .to_string(),
                ));
            }
            let normalized = normalized_samples(&frame.data);
            for channel in 0..channels {
                for time_index in 0..time_steps {
                    values.push(normalized[time_index * channels + channel]);
                }
            }
        }

        Self::new(
            sample_rate,
            F32Tensor::from_dims([frames.len(), channels, time_steps], values)?,
        )
    }

    pub fn tensor(&self) -> &F32Tensor {
        &self.tensor
    }

    pub fn as_view(&self) -> Result<AudioWaveformBatchView<'_>> {
        AudioWaveformBatchView::new(self.sample_rate, self.tensor.as_view())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameSpec {
    pub frame_size: usize,
    pub hop_size: usize,
}

impl FrameSpec {
    pub fn new(frame_size: usize, hop_size: usize) -> Result<Self> {
        FrameStride::new(frame_size, hop_size)?;
        Ok(Self {
            frame_size,
            hop_size,
        })
    }

    pub fn frames<'a>(&self, samples: &'a [f32]) -> AudioFrames<'a> {
        AudioFrames {
            samples,
            spec: *self,
            offset: 0,
        }
    }

    pub fn frame_count(&self, samples_len: usize) -> usize {
        FrameStride::from(*self).frame_count(samples_len)
    }
}

impl From<FrameSpec> for FrameStride {
    fn from(value: FrameSpec) -> Self {
        Self {
            frame_size: value.frame_size,
            hop_size: value.hop_size,
        }
    }
}

impl TryFrom<FrameStride> for FrameSpec {
    type Error = DetectError;

    fn try_from(value: FrameStride) -> Result<Self> {
        Self::new(value.frame_size, value.hop_size)
    }
}

#[derive(Debug, Clone)]
pub struct AudioFrames<'a> {
    samples: &'a [f32],
    spec: FrameSpec,
    offset: usize,
}

impl<'a> Iterator for AudioFrames<'a> {
    type Item = (usize, &'a [f32]);

    fn next(&mut self) -> Option<Self::Item> {
        let end = self.offset.checked_add(self.spec.frame_size)?;
        if end > self.samples.len() {
            return None;
        }
        let offset = self.offset;
        self.offset += self.spec.hop_size;
        Some((offset, &self.samples[offset..end]))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamingFrameConfig {
    pub frame_size: usize,
    pub hop_size: usize,
    pub channel_mix: ChannelMix,
    pub max_buffered_samples: usize,
}

impl StreamingFrameConfig {
    pub fn new(frame_size: usize, hop_size: usize) -> Result<Self> {
        FrameSpec::new(frame_size, hop_size)?;
        Ok(Self {
            frame_size,
            hop_size,
            channel_mix: ChannelMix::Average,
            max_buffered_samples: frame_size.saturating_add(hop_size).max(frame_size),
        })
    }

    pub fn channel_mix(mut self, mix: ChannelMix) -> Self {
        self.channel_mix = mix;
        self
    }

    pub fn max_buffered_samples(mut self, samples: usize) -> Self {
        self.max_buffered_samples = samples.max(self.frame_size);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AudioWindow {
    pub timestamp: Timestamp,
    pub sample_rate: u32,
    pub start_sample: u64,
    pub samples: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StreamingFrameBuffer {
    config: StreamingFrameConfig,
    sample_rate: Option<u32>,
    channels: Option<u16>,
    buffer: Vec<f32>,
    buffered_start_sample: u64,
    next_window_start_sample: Option<u64>,
}

impl StreamingFrameBuffer {
    pub fn new(config: StreamingFrameConfig) -> Result<Self> {
        FrameSpec::new(config.frame_size, config.hop_size)?;
        if config.max_buffered_samples < config.frame_size {
            return Err(DetectError::InvalidArgument(
                "max_buffered_samples must be at least frame_size".to_string(),
            ));
        }
        Ok(Self {
            config,
            sample_rate: None,
            channels: None,
            buffer: Vec::new(),
            buffered_start_sample: 0,
            next_window_start_sample: None,
        })
    }

    pub fn push_frame(&mut self, frame: &AudioFrame<'_>) -> Result<Vec<AudioWindow>> {
        self.validate_stream_format(frame)?;
        let frame_start_sample = timestamp_to_sample(frame.timestamp, frame.sample_rate)?;
        if self.next_window_start_sample.is_none() {
            self.buffered_start_sample = frame_start_sample;
            self.next_window_start_sample = Some(frame_start_sample);
        }

        let buffered_end_sample = self.buffered_start_sample + self.buffer.len() as u64;
        if frame_start_sample > buffered_end_sample {
            self.buffer.clear();
            self.buffered_start_sample = frame_start_sample;
            self.next_window_start_sample = Some(frame_start_sample);
        } else if frame_start_sample < buffered_end_sample {
            return Err(DetectError::InvalidArgument(
                "streaming audio frames must not overlap".to_string(),
            ));
        }

        self.buffer.extend(interleaved_to_mono(
            frame.data,
            frame.channels,
            self.config.channel_mix,
        )?);

        let mut windows = Vec::new();
        let mut next_start = self
            .next_window_start_sample
            .expect("next window start is initialized above");
        let buffered_end_sample = self.buffered_start_sample + self.buffer.len() as u64;
        while next_start + self.config.frame_size as u64 <= buffered_end_sample {
            let offset = (next_start - self.buffered_start_sample) as usize;
            let end = offset + self.config.frame_size;
            windows.push(AudioWindow {
                timestamp: sample_to_timestamp(next_start, frame.sample_rate),
                sample_rate: frame.sample_rate,
                start_sample: next_start,
                samples: self.buffer[offset..end].to_vec(),
            });
            next_start += self.config.hop_size as u64;
        }
        self.next_window_start_sample = Some(next_start);
        self.trim_consumed();
        self.enforce_buffer_bound()?;
        Ok(windows)
    }

    pub fn reset(&mut self) {
        self.sample_rate = None;
        self.channels = None;
        self.buffer.clear();
        self.buffered_start_sample = 0;
        self.next_window_start_sample = None;
    }

    pub fn buffered_samples(&self) -> usize {
        self.buffer.len()
    }

    fn validate_stream_format(&mut self, frame: &AudioFrame<'_>) -> Result<()> {
        match (self.sample_rate, self.channels) {
            (None, None) => {
                self.sample_rate = Some(frame.sample_rate);
                self.channels = Some(frame.channels);
                Ok(())
            }
            (Some(sample_rate), Some(channels))
                if sample_rate == frame.sample_rate && channels == frame.channels =>
            {
                Ok(())
            }
            _ => Err(DetectError::InvalidArgument(
                "streaming audio sample_rate and channels must remain stable".to_string(),
            )),
        }
    }

    fn trim_consumed(&mut self) {
        let Some(next_start) = self.next_window_start_sample else {
            return;
        };
        if next_start <= self.buffered_start_sample {
            return;
        }
        let drop = (next_start - self.buffered_start_sample).min(self.buffer.len() as u64) as usize;
        if drop > 0 {
            self.buffer.drain(0..drop);
            self.buffered_start_sample += drop as u64;
        }
    }

    fn enforce_buffer_bound(&mut self) -> Result<()> {
        if self.buffer.len() <= self.config.max_buffered_samples {
            return Ok(());
        }
        Err(DetectError::InvalidArgument(format!(
            "streaming audio buffer exceeded max_buffered_samples ({})",
            self.config.max_buffered_samples
        )))
    }
}

pub fn mono_samples(frame: &AudioFrame<'_>) -> Result<MonoSamples> {
    mono_samples_with_mix(frame, ChannelMix::Average)
}

pub fn mono_samples_with_mix(frame: &AudioFrame<'_>, mix: ChannelMix) -> Result<MonoSamples> {
    let samples = interleaved_to_mono(frame.data, frame.channels, mix)?;
    Ok(MonoSamples {
        timestamp: frame.timestamp,
        sample_rate: frame.sample_rate,
        samples,
    })
}

pub fn interleaved_to_mono(
    buffer: &AudioBuffer,
    channels: u16,
    mix: ChannelMix,
) -> Result<Vec<f32>> {
    if channels == 0 {
        return Err(DetectError::InvalidAudioFormat {
            sample_rate: 1,
            channels,
        });
    }
    let channels = channels as usize;
    if !buffer.len().is_multiple_of(channels) {
        return Err(DetectError::InvalidArgument(format!(
            "audio buffer length {} is not divisible by channel count {channels}",
            buffer.len()
        )));
    }
    let normalized = normalized_samples(buffer);
    Ok(match mix {
        ChannelMix::First => normalized
            .chunks_exact(channels)
            .map(|frame| frame[0])
            .collect(),
        ChannelMix::Average => normalized
            .chunks_exact(channels)
            .map(|frame| frame.iter().sum::<f32>() / channels as f32)
            .collect(),
    })
}

pub fn normalized_samples(buffer: &AudioBuffer) -> Vec<f32> {
    match buffer {
        AudioBuffer::U8(values) => values
            .iter()
            .map(|value| (*value as f32 - 128.0) / 128.0)
            .collect(),
        AudioBuffer::I16(values) => values
            .iter()
            .map(|value| *value as f32 / i16::MAX as f32)
            .collect(),
        AudioBuffer::I32(values) => values
            .iter()
            .map(|value| *value as f32 / i32::MAX as f32)
            .collect(),
        AudioBuffer::F32(values) => values.clone(),
    }
}

pub fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    (samples.iter().map(|sample| sample * sample).sum::<f32>() / samples.len() as f32).sqrt()
}

pub fn peak(samples: &[f32]) -> f32 {
    samples
        .iter()
        .map(|sample| sample.abs())
        .fold(0.0_f32, f32::max)
}

pub fn mean_absolute(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    samples.iter().map(|sample| sample.abs()).sum::<f32>() / samples.len() as f32
}

pub fn zero_pad_to(mut samples: Vec<f32>, target_len: usize) -> Vec<f32> {
    samples.resize(target_len, 0.0);
    samples
}

pub fn seconds_to_samples(seconds: f64, sample_rate: u32) -> Result<u64> {
    AudioFormatSpec::new(sample_rate, 1)?;
    if !seconds.is_finite() || seconds < 0.0 {
        return Err(DetectError::InvalidArgument(
            "audio duration must be a finite non-negative value".to_string(),
        ));
    }
    let samples = seconds * sample_rate as f64;
    if !samples.is_finite() || samples < 0.0 {
        return Err(DetectError::InvalidArgument(
            "audio timestamp must resolve to a finite non-negative sample index".to_string(),
        ));
    }
    Ok(samples.round() as u64)
}

pub fn samples_to_seconds(samples: u64, sample_rate: u32) -> Result<f64> {
    AudioFormatSpec::new(sample_rate, 1)?;
    Ok(samples as f64 / sample_rate as f64)
}

pub fn timestamp_to_sample(timestamp: Timestamp, sample_rate: u32) -> Result<u64> {
    if timestamp.timebase.den == 0 {
        return Err(DetectError::InvalidAudioFormat {
            sample_rate,
            channels: 1,
        });
    }
    seconds_to_samples(timestamp.seconds(), sample_rate)
}

pub fn sample_to_timestamp(sample: u64, sample_rate: u32) -> Timestamp {
    Timestamp::new(sample as i64, Timebase::new(1, sample_rate as i32))
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use video_analysis_core::{AudioBuffer, AudioFrame, Timebase, Timestamp};

    fn assert_approx_eq(actual: f32, expected: f32, tolerance: f32) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "expected {actual} to be within {tolerance} of {expected}"
        );
    }

    fn assert_approx_slice(actual: &[f32], expected: &[f32], tolerance: f32) {
        assert_eq!(actual.len(), expected.len(), "slice lengths differ");
        for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
            assert!(
                (*actual - *expected).abs() <= tolerance,
                "index {index}: expected {actual} to be within {tolerance} of {expected}"
            );
        }
    }

    fn ts() -> Timestamp {
        Timestamp::new(0, Timebase::new(1, 48_000))
    }

    fn frame_at(sample: u64, samples: Vec<f32>) -> AudioBuffer {
        let _ = sample;
        AudioBuffer::F32(samples)
    }

    #[test]
    fn mixes_interleaved_stereo_to_mono() {
        let buffer = AudioBuffer::F32(vec![1.0, -1.0, 0.5, 0.25]);
        let mono = interleaved_to_mono(&buffer, 2, ChannelMix::Average).unwrap();
        assert_eq!(mono, vec![0.0, 0.375]);
    }

    #[test]
    fn normalizes_all_supported_sample_formats() {
        assert_approx_slice(
            &normalized_samples(&AudioBuffer::U8(vec![0, 128, 255])),
            &[-1.0, 0.0, 127.0 / 128.0],
            1.0e-6,
        );
        assert_approx_slice(
            &normalized_samples(&AudioBuffer::I16(vec![i16::MIN, 0, i16::MAX])),
            &[i16::MIN as f32 / i16::MAX as f32, 0.0, 1.0],
            1.0e-6,
        );
        assert_approx_slice(
            &normalized_samples(&AudioBuffer::I32(vec![i32::MIN, 0, i32::MAX])),
            &[i32::MIN as f32 / i32::MAX as f32, 0.0, 1.0],
            1.0e-6,
        );
        assert_eq!(
            normalized_samples(&AudioBuffer::F32(vec![-0.25, 0.0, 0.5])),
            vec![-0.25, 0.0, 0.5]
        );
    }

    #[test]
    fn first_channel_mix_uses_first_interleaved_sample() {
        let buffer = AudioBuffer::F32(vec![1.0, -1.0, 0.5, 0.25]);
        let mono = interleaved_to_mono(&buffer, 2, ChannelMix::First).unwrap();
        assert_eq!(mono, vec![1.0, 0.5]);
    }

    #[test]
    fn batches_existing_audio_frames_into_channel_major_waveforms() {
        let first = video_analysis_core::OwnedAudioFrame::new(
            ts(),
            48_000,
            1,
            AudioBuffer::F32(vec![0.1, 0.2]),
        )
        .unwrap();
        let second = video_analysis_core::OwnedAudioFrame::new(
            ts(),
            48_000,
            1,
            AudioBuffer::F32(vec![0.3, 0.4]),
        )
        .unwrap();

        let batch = OwnedAudioWaveformBatch::from_audio_frames(&[first, second]).unwrap();
        let view = batch.as_view().unwrap();
        assert_eq!(view.batch_size(), 2);
        assert_eq!(view.waveform(1, 0).unwrap(), &[0.3, 0.4]);
    }

    #[test]
    fn mono_mix_rejects_invalid_channel_layouts() {
        assert!(interleaved_to_mono(&AudioBuffer::F32(vec![1.0]), 0, ChannelMix::Average).is_err());
        assert!(interleaved_to_mono(
            &AudioBuffer::F32(vec![1.0, 2.0, 3.0]),
            2,
            ChannelMix::Average
        )
        .is_err());
    }

    #[test]
    fn frame_spec_validates_sizes_and_counts_frames() {
        assert!(FrameSpec::new(0, 1).is_err());
        assert!(FrameSpec::new(4, 0).is_err());
        let spec = FrameSpec::new(4, 2).unwrap();
        assert_eq!(spec.frame_count(3), 0);
        assert_eq!(spec.frame_count(4), 1);
        assert_eq!(spec.frame_count(6), 2);
        assert_eq!(spec.frame_count(7), 2);
    }

    #[test]
    fn frame_spec_iterates_over_hops() {
        let spec = FrameSpec::new(4, 2).unwrap();
        let samples = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
        let frames = spec.frames(&samples).collect::<Vec<_>>();
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0], (0, &[0.0, 1.0, 2.0, 3.0][..]));
        assert_eq!(frames[1], (2, &[2.0, 3.0, 4.0, 5.0][..]));
    }

    #[test]
    fn audio_frame_to_mono_preserves_timing() {
        let buffer = AudioBuffer::I16(vec![0, i16::MAX]);
        let frame = AudioFrame::new(ts(), 48_000, 1, &buffer).unwrap();
        let mono = mono_samples(&frame).unwrap();
        assert_eq!(mono.timestamp, ts());
        assert_eq!(mono.sample_rate, 48_000);
        assert_eq!(mono.samples, vec![0.0, 1.0]);
    }

    #[test]
    fn hann_window_tapers_edges() {
        let windowed = WindowFunction::Hann.apply(&[1.0, 1.0, 1.0, 1.0]);
        assert!(windowed[0].abs() < 0.000_001);
        assert!(windowed[1] > 0.7);
        assert!(windowed[2] > 0.7);
        assert!(windowed[3].abs() < 0.000_001);
    }

    #[test]
    fn streaming_buffer_emits_windows_inside_one_chunk() {
        let config = StreamingFrameConfig::new(4, 2).unwrap();
        let mut buffer = StreamingFrameBuffer::new(config).unwrap();
        let samples = AudioBuffer::F32(vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0]);
        let frame = AudioFrame::new(ts(), 48_000, 1, &samples).unwrap();

        let windows = buffer.push_frame(&frame).unwrap();

        assert_eq!(windows.len(), 2);
        assert_eq!(windows[0].start_sample, 0);
        assert_eq!(windows[0].samples, vec![0.0, 1.0, 2.0, 3.0]);
        assert_eq!(windows[1].start_sample, 2);
        assert_eq!(windows[1].samples, vec![2.0, 3.0, 4.0, 5.0]);
    }

    #[test]
    fn streaming_buffer_emits_windows_across_chunks() {
        let config = StreamingFrameConfig::new(4, 2).unwrap();
        let mut buffer = StreamingFrameBuffer::new(config).unwrap();
        let first = AudioBuffer::F32(vec![0.0, 1.0, 2.0]);
        let second = AudioBuffer::F32(vec![3.0, 4.0, 5.0]);
        let first_frame = AudioFrame::new(ts(), 48_000, 1, &first).unwrap();
        let second_frame = AudioFrame::new(
            Timestamp::new(3, Timebase::new(1, 48_000)),
            48_000,
            1,
            &second,
        )
        .unwrap();

        assert!(buffer.push_frame(&first_frame).unwrap().is_empty());
        let windows = buffer.push_frame(&second_frame).unwrap();

        assert_eq!(windows.len(), 2);
        assert_eq!(windows[0].samples, vec![0.0, 1.0, 2.0, 3.0]);
        assert_eq!(windows[1].samples, vec![2.0, 3.0, 4.0, 5.0]);
    }

    #[test]
    fn streaming_buffer_preserves_window_timestamps() {
        let config = StreamingFrameConfig::new(4, 2).unwrap();
        let mut buffer = StreamingFrameBuffer::new(config).unwrap();
        let samples = AudioBuffer::F32(vec![0.0; 6]);
        let frame = AudioFrame::new(
            Timestamp::new(10, Timebase::new(1, 48_000)),
            48_000,
            1,
            &samples,
        )
        .unwrap();

        let windows = buffer.push_frame(&frame).unwrap();

        assert_eq!(
            windows
                .iter()
                .map(|window| window.timestamp)
                .collect::<Vec<_>>(),
            vec![
                Timestamp::new(10, Timebase::new(1, 48_000)),
                Timestamp::new(12, Timebase::new(1, 48_000)),
            ]
        );
    }

    #[test]
    fn streaming_buffer_requires_stable_format() {
        let config = StreamingFrameConfig::new(4, 2).unwrap();
        let mut buffer = StreamingFrameBuffer::new(config).unwrap();
        let first = AudioBuffer::F32(vec![0.0; 4]);
        let second = AudioBuffer::F32(vec![0.0; 4]);
        let first_frame = AudioFrame::new(ts(), 48_000, 1, &first).unwrap();
        let second_frame = AudioFrame::new(
            Timestamp::new(4, Timebase::new(1, 44_100)),
            44_100,
            1,
            &second,
        )
        .unwrap();

        buffer.push_frame(&first_frame).unwrap();

        assert!(buffer.push_frame(&second_frame).is_err());
    }

    #[test]
    fn streaming_buffer_keeps_retained_samples_bounded() {
        let config = StreamingFrameConfig::new(8, 8)
            .unwrap()
            .max_buffered_samples(8);
        let mut buffer = StreamingFrameBuffer::new(config).unwrap();
        let samples = AudioBuffer::F32(vec![0.0; 32]);
        let frame = AudioFrame::new(ts(), 48_000, 1, &samples).unwrap();

        assert!(buffer.push_frame(&frame).is_ok());
        assert!(buffer.buffered_samples() <= 8);
    }

    #[test]
    fn streaming_buffer_reset_allows_new_format() {
        let config = StreamingFrameConfig::new(4, 2).unwrap();
        let mut buffer = StreamingFrameBuffer::new(config).unwrap();
        let first = AudioBuffer::F32(vec![0.0; 4]);
        let second = AudioBuffer::F32(vec![0.0; 4]);
        buffer
            .push_frame(&AudioFrame::new(ts(), 48_000, 1, &first).unwrap())
            .unwrap();
        buffer.reset();
        assert!(buffer
            .push_frame(
                &AudioFrame::new(
                    Timestamp::new(0, Timebase::new(1, 44_100)),
                    44_100,
                    1,
                    &second
                )
                .unwrap()
            )
            .is_ok());
    }

    proptest! {
        #[test]
        fn generated_interleaved_mono_length_matches_samples_per_channel(
            channels in 1_u16..=8,
            frames in 0_usize..64,
            samples in proptest::collection::vec(-1.0_f32..1.0, 0..512),
        ) {
            let channels = channels as usize;
            let len = frames * channels;
            let mut values = samples;
            values.resize(len, 0.0);
            let mono = interleaved_to_mono(&AudioBuffer::F32(values), channels as u16, ChannelMix::Average).unwrap();
            prop_assert_eq!(mono.len(), frames);
        }

        #[test]
        fn streaming_windows_do_not_depend_on_chunk_partition(
            len in 16_usize..96,
            chunk_size in 1_usize..24,
        ) {
            let samples = (0..len).map(|value| value as f32).collect::<Vec<_>>();
            let config = StreamingFrameConfig::new(8, 4).unwrap();

            let all_buffer = AudioBuffer::F32(samples.clone());
            let all_frame = AudioFrame::new(ts(), 48_000, 1, &all_buffer).unwrap();
            let mut all = StreamingFrameBuffer::new(config).unwrap();
            let expected = all.push_frame(&all_frame).unwrap();

            let mut chunked = StreamingFrameBuffer::new(config).unwrap();
            let mut actual = Vec::new();
            let mut start = 0;
            while start < samples.len() {
                let end = (start + chunk_size).min(samples.len());
                let buffer = frame_at(start as u64, samples[start..end].to_vec());
                let frame = AudioFrame::new(
                    Timestamp::new(start as i64, Timebase::new(1, 48_000)),
                    48_000,
                    1,
                    &buffer,
                )
                .unwrap();
                actual.extend(chunked.push_frame(&frame).unwrap());
                start = end;
            }

            prop_assert_eq!(actual, expected);
        }
    }

    #[test]
    fn scalar_level_helpers_are_empty_safe() {
        assert_approx_eq(rms(&[1.0, -1.0]), 1.0, 1.0e-6);
        assert_eq!(peak(&[]), 0.0);
        assert_eq!(mean_absolute(&[]), 0.0);
    }

    #[test]
    fn audio_format_spec_validates_and_reports_duration() {
        let spec = AudioFormatSpec::new(48_000, 2)
            .unwrap()
            .frame_samples(2_048)
            .unwrap();
        assert_eq!(spec.duration_seconds(4_800).unwrap(), 0.1);
        assert!(AudioFormatSpec::new(0, 2).is_err());
        assert!(AudioFormatSpec::new(48_000, 0).is_err());
        assert!(AudioFormatSpec::new(48_000, 2)
            .unwrap()
            .frame_samples(0)
            .is_err());
    }

    #[test]
    fn sample_and_timestamp_helpers_round_trip() {
        let timestamp = Timestamp::new(2_205, Timebase::new(1, 44_100));
        let sample = timestamp_to_sample(timestamp, 44_100).unwrap();
        assert_eq!(sample, 2_205);
        assert_eq!(sample_to_timestamp(sample, 44_100), timestamp);
        assert_eq!(seconds_to_samples(0.5, 16_000).unwrap(), 8_000);
        assert_eq!(samples_to_seconds(8_000, 16_000).unwrap(), 0.5);
        assert!(seconds_to_samples(-1.0, 16_000).is_err());
    }

    #[test]
    fn streaming_buffer_detects_overlapping_chunks() {
        let config = StreamingFrameConfig::new(4, 2).unwrap();
        let mut buffer = StreamingFrameBuffer::new(config).unwrap();
        let first = AudioBuffer::F32(vec![0.0, 1.0, 2.0, 3.0]);
        let second = AudioBuffer::F32(vec![2.0, 3.0, 4.0, 5.0]);
        let first_frame = AudioFrame::new(ts(), 48_000, 1, &first).unwrap();
        let overlapping = AudioFrame::new(
            Timestamp::new(2, Timebase::new(1, 48_000)),
            48_000,
            1,
            &second,
        )
        .unwrap();
        buffer.push_frame(&first_frame).unwrap();
        assert!(buffer.push_frame(&overlapping).is_err());
    }
}
