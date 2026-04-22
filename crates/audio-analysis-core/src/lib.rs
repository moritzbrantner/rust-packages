use video_analysis_core::{AudioBuffer, AudioFrame, DetectError, Result, Timestamp};

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

#[cfg(test)]
mod tests {
    use super::*;
    use video_analysis_core::{AudioBuffer, AudioFrame, Timebase, Timestamp};

    fn ts() -> Timestamp {
        Timestamp::new(0, Timebase::new(1, 48_000))
    }

    #[test]
    fn mixes_interleaved_stereo_to_mono() {
        let buffer = AudioBuffer::F32(vec![1.0, -1.0, 0.5, 0.25]);
        let mono = interleaved_to_mono(&buffer, 2, ChannelMix::Average).unwrap();
        assert_eq!(mono, vec![0.0, 0.375]);
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
}
