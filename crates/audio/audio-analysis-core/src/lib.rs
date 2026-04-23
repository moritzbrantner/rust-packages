#![doc = include_str!("../README.md")]

use video_analysis_core::{AudioBuffer, AudioFrame, DetectError, Result, Timebase, Timestamp};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameSpec {
    pub frame_size: usize,
    pub hop_size: usize,
}

impl FrameSpec {
    pub fn new(frame_size: usize, hop_size: usize) -> Result<Self> {
        if frame_size == 0 || hop_size == 0 {
            return Err(DetectError::InvalidArgument(
                "frame_size and hop_size must be greater than zero".to_string(),
            ));
        }
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
        if samples_len < self.frame_size {
            return 0;
        }
        1 + (samples_len - self.frame_size) / self.hop_size
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowFunction {
    Rectangular,
    Hann,
    Hamming,
    Blackman,
}

impl WindowFunction {
    pub fn coefficient(self, index: usize, len: usize) -> f32 {
        if len <= 1 {
            return 1.0;
        }
        let phase = 2.0 * std::f32::consts::PI * index as f32 / (len - 1) as f32;
        match self {
            Self::Rectangular => 1.0,
            Self::Hann => 0.5 - 0.5 * phase.cos(),
            Self::Hamming => 0.54 - 0.46 * phase.cos(),
            Self::Blackman => 0.42 - 0.5 * phase.cos() + 0.08 * (2.0 * phase).cos(),
        }
    }

    pub fn weights(self, len: usize) -> Vec<f32> {
        (0..len).map(|index| self.coefficient(index, len)).collect()
    }

    pub fn apply(self, samples: &[f32]) -> Vec<f32> {
        samples
            .iter()
            .enumerate()
            .map(|(index, sample)| sample * self.coefficient(index, samples.len()))
            .collect()
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

fn timestamp_to_sample(timestamp: Timestamp, sample_rate: u32) -> Result<u64> {
    if sample_rate == 0 || timestamp.timebase.den == 0 {
        return Err(DetectError::InvalidAudioFormat {
            sample_rate,
            channels: 1,
        });
    }
    let samples = timestamp.seconds() * sample_rate as f64;
    if !samples.is_finite() || samples < 0.0 {
        return Err(DetectError::InvalidArgument(
            "audio timestamp must resolve to a finite non-negative sample index".to_string(),
        ));
    }
    Ok(samples.round() as u64)
}

fn sample_to_timestamp(sample: u64, sample_rate: u32) -> Timestamp {
    Timestamp::new(sample as i64, Timebase::new(1, sample_rate as i32))
}

#[cfg(test)]
mod tests {
    use super::*;
    use audio_analysis_test_support::{assert_approx_eq, assert_approx_slice};
    use proptest::prelude::*;
    use video_analysis_core::{AudioBuffer, AudioFrame, Timebase, Timestamp};

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
}
