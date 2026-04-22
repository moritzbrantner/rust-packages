use std::collections::VecDeque;

use audio_analysis_core::{interleaved_to_mono, normalized_samples, ChannelMix};
use video_analysis_core::{AudioBuffer, AudioFrame, DetectError, OwnedAudioFrame, Result};
use video_analysis_ingest::{AudioFrameSource, MediaSourceInfo};

pub trait AudioTransform {
    fn name(&self) -> &str;
    fn reset(&mut self);
    fn process_frame(&mut self, frame: &AudioFrame<'_>) -> Result<Option<OwnedAudioFrame>>;

    fn finish(&mut self) -> Result<Vec<OwnedAudioFrame>> {
        Ok(Vec::new())
    }
}

#[derive(Default)]
pub struct AudioProcessor {
    transforms: Vec<Box<dyn AudioTransform>>,
    output_channels: Option<u16>,
}

impl AudioProcessor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn transform<T: AudioTransform + 'static>(mut self, transform: T) -> Self {
        self.transforms.push(Box::new(transform));
        self
    }

    pub fn gain(self, linear: f32) -> Self {
        self.transform(Gain { linear })
    }

    pub fn gain_db(self, db: f32) -> Self {
        self.gain(10.0_f32.powf(db / 20.0))
    }

    pub fn hard_clip(self, min: f32, max: f32) -> Self {
        self.transform(HardClip { min, max })
    }

    pub fn mono(mut self, mix: ChannelMix) -> Self {
        self.output_channels = Some(1);
        self.transform(Mono { mix })
    }

    pub fn dc_block(self, coefficient: f32) -> Self {
        self.transform(DcBlock::new(coefficient))
    }

    pub fn biquad(self, spec: BiquadSpec) -> Self {
        self.transform(BiquadFilter::new(spec))
    }

    pub fn noise_gate(self, spec: NoiseGateSpec) -> Self {
        self.transform(NoiseGate { spec })
    }

    pub fn process_frame(&mut self, frame: OwnedAudioFrame) -> Result<Option<OwnedAudioFrame>> {
        let mut current = Some(frame);
        for transform in &mut self.transforms {
            let Some(frame) = current else {
                return Ok(None);
            };
            let frame_ref = frame.as_frame()?;
            current = transform.process_frame(&frame_ref)?;
        }
        Ok(current)
    }

    pub fn finish(&mut self) -> Result<Vec<OwnedAudioFrame>> {
        let mut output = Vec::new();
        for transform in &mut self.transforms {
            output.extend(transform.finish()?);
        }
        Ok(output)
    }

    pub fn reset(&mut self) {
        for transform in &mut self.transforms {
            transform.reset();
        }
    }

    pub fn output_channels(&self) -> Option<u16> {
        self.output_channels
    }
}

pub struct ProcessedAudioSource<S> {
    source: S,
    processor: AudioProcessor,
    source_info: MediaSourceInfo,
    pending: VecDeque<OwnedAudioFrame>,
    finished: bool,
}

impl<S: AudioFrameSource> ProcessedAudioSource<S> {
    pub fn new(source: S, processor: AudioProcessor) -> Self {
        let mut source_info = source.source_info().clone();
        if let Some(channels) = processor.output_channels() {
            for audio in &mut source_info.audio {
                audio.channels = channels;
            }
        }
        Self {
            source,
            processor,
            source_info,
            pending: VecDeque::new(),
            finished: false,
        }
    }

    pub fn inner(&self) -> &S {
        &self.source
    }

    pub fn inner_mut(&mut self) -> &mut S {
        &mut self.source
    }

    pub fn processor(&self) -> &AudioProcessor {
        &self.processor
    }

    pub fn processor_mut(&mut self) -> &mut AudioProcessor {
        &mut self.processor
    }
}

impl<S: AudioFrameSource> AudioFrameSource for ProcessedAudioSource<S> {
    fn source_info(&self) -> &MediaSourceInfo {
        &self.source_info
    }

    fn next_audio_frame(&mut self) -> Result<Option<OwnedAudioFrame>> {
        loop {
            if let Some(frame) = self.pending.pop_front() {
                return Ok(Some(frame));
            }
            if self.finished {
                return Ok(None);
            }
            match self.source.next_audio_frame()? {
                Some(frame) => {
                    if let Some(processed) = self.processor.process_frame(frame)? {
                        return Ok(Some(processed));
                    }
                }
                None => {
                    self.pending.extend(self.processor.finish()?);
                    self.finished = true;
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gain {
    pub linear: f32,
}

impl AudioTransform for Gain {
    fn name(&self) -> &str {
        "audio_gain"
    }

    fn reset(&mut self) {}

    fn process_frame(&mut self, frame: &AudioFrame<'_>) -> Result<Option<OwnedAudioFrame>> {
        if !self.linear.is_finite() {
            return Err(DetectError::InvalidArgument(
                "gain must be finite".to_string(),
            ));
        }
        map_samples(frame, |sample| sample * self.linear).map(Some)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HardClip {
    pub min: f32,
    pub max: f32,
}

impl AudioTransform for HardClip {
    fn name(&self) -> &str {
        "audio_hard_clip"
    }

    fn reset(&mut self) {}

    fn process_frame(&mut self, frame: &AudioFrame<'_>) -> Result<Option<OwnedAudioFrame>> {
        if !self.min.is_finite() || !self.max.is_finite() || self.min > self.max {
            return Err(DetectError::InvalidArgument(
                "clip bounds must be finite and min must be less than or equal to max".to_string(),
            ));
        }
        map_samples(frame, |sample| sample.clamp(self.min, self.max)).map(Some)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mono {
    pub mix: ChannelMix,
}

impl AudioTransform for Mono {
    fn name(&self) -> &str {
        "audio_mono"
    }

    fn reset(&mut self) {}

    fn process_frame(&mut self, frame: &AudioFrame<'_>) -> Result<Option<OwnedAudioFrame>> {
        let samples = interleaved_to_mono(frame.data, frame.channels, self.mix)?;
        OwnedAudioFrame::new(
            frame.timestamp,
            frame.sample_rate,
            1,
            AudioBuffer::F32(samples),
        )
        .map(Some)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DcBlock {
    pub coefficient: f32,
    states: Vec<DcBlockState>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct DcBlockState {
    previous_input: f32,
    previous_output: f32,
}

impl DcBlock {
    pub fn new(coefficient: f32) -> Self {
        Self {
            coefficient,
            states: Vec::new(),
        }
    }
}

impl AudioTransform for DcBlock {
    fn name(&self) -> &str {
        "audio_dc_block"
    }

    fn reset(&mut self) {
        self.states.clear();
    }

    fn process_frame(&mut self, frame: &AudioFrame<'_>) -> Result<Option<OwnedAudioFrame>> {
        if !self.coefficient.is_finite() || !(0.0..1.0).contains(&self.coefficient) {
            return Err(DetectError::InvalidArgument(
                "dc block coefficient must be finite and in the range [0, 1)".to_string(),
            ));
        }
        let samples = normalized_interleaved(frame)?;
        let channels = frame.channels as usize;
        self.states.resize(channels, DcBlockState::default());
        let mut output = Vec::with_capacity(samples.len());
        for frame_samples in samples.chunks_exact(channels) {
            for (channel, sample) in frame_samples.iter().enumerate() {
                let state = &mut self.states[channel];
                let filtered =
                    *sample - state.previous_input + self.coefficient * state.previous_output;
                state.previous_input = *sample;
                state.previous_output = filtered;
                output.push(filtered);
            }
        }
        owned_like(frame, output).map(Some)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BiquadKind {
    LowPass,
    HighPass,
    BandPass,
    Notch,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BiquadSpec {
    pub kind: BiquadKind,
    pub cutoff_hz: f32,
    pub q: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BiquadFilter {
    pub spec: BiquadSpec,
    states: Vec<BiquadState>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct BiquadState {
    z1: f32,
    z2: f32,
}

#[derive(Debug, Clone, Copy)]
struct BiquadCoefficients {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
}

impl BiquadFilter {
    pub fn new(spec: BiquadSpec) -> Self {
        Self {
            spec,
            states: Vec::new(),
        }
    }
}

impl AudioTransform for BiquadFilter {
    fn name(&self) -> &str {
        "audio_biquad"
    }

    fn reset(&mut self) {
        self.states.clear();
    }

    fn process_frame(&mut self, frame: &AudioFrame<'_>) -> Result<Option<OwnedAudioFrame>> {
        let coefficients = coefficients(self.spec, frame.sample_rate)?;
        let samples = normalized_interleaved(frame)?;
        let channels = frame.channels as usize;
        self.states.resize(channels, BiquadState::default());
        let mut output = Vec::with_capacity(samples.len());
        for frame_samples in samples.chunks_exact(channels) {
            for (channel, sample) in frame_samples.iter().enumerate() {
                let state = &mut self.states[channel];
                let filtered = coefficients.b0 * *sample + state.z1;
                state.z1 = coefficients.b1 * *sample - coefficients.a1 * filtered + state.z2;
                state.z2 = coefficients.b2 * *sample - coefficients.a2 * filtered;
                output.push(filtered);
            }
        }
        owned_like(frame, output).map(Some)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NoiseGateSpec {
    pub threshold: f32,
    pub attenuation: f32,
}

impl Default for NoiseGateSpec {
    fn default() -> Self {
        Self {
            threshold: 0.01,
            attenuation: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NoiseGate {
    pub spec: NoiseGateSpec,
}

impl AudioTransform for NoiseGate {
    fn name(&self) -> &str {
        "audio_noise_gate"
    }

    fn reset(&mut self) {}

    fn process_frame(&mut self, frame: &AudioFrame<'_>) -> Result<Option<OwnedAudioFrame>> {
        if !self.spec.threshold.is_finite()
            || self.spec.threshold < 0.0
            || !self.spec.attenuation.is_finite()
            || !(0.0..=1.0).contains(&self.spec.attenuation)
        {
            return Err(DetectError::InvalidArgument(
                "noise gate threshold must be finite and non-negative, and attenuation must be in [0, 1]"
                    .to_string(),
            ));
        }
        map_samples(frame, |sample| {
            if sample.abs() < self.spec.threshold {
                sample * self.spec.attenuation
            } else {
                sample
            }
        })
        .map(Some)
    }
}

pub fn collect_audio_frames<S: AudioFrameSource>(source: &mut S) -> Result<Vec<OwnedAudioFrame>> {
    let mut frames = Vec::new();
    while let Some(frame) = source.next_audio_frame()? {
        frames.push(frame);
    }
    Ok(frames)
}

pub fn process_audio_source<S: AudioFrameSource>(
    source: &mut S,
    processor: &mut AudioProcessor,
) -> Result<Vec<OwnedAudioFrame>> {
    let mut frames = Vec::new();
    while let Some(frame) = source.next_audio_frame()? {
        if let Some(processed) = processor.process_frame(frame)? {
            frames.push(processed);
        }
    }
    frames.extend(processor.finish()?);
    Ok(frames)
}

fn map_samples(frame: &AudioFrame<'_>, mut map: impl FnMut(f32) -> f32) -> Result<OwnedAudioFrame> {
    let samples = normalized_interleaved(frame)?
        .into_iter()
        .map(&mut map)
        .collect();
    owned_like(frame, samples)
}

fn normalized_interleaved(frame: &AudioFrame<'_>) -> Result<Vec<f32>> {
    if frame.channels == 0 {
        return Err(DetectError::InvalidAudioFormat {
            sample_rate: frame.sample_rate,
            channels: frame.channels,
        });
    }
    if !frame.data.len().is_multiple_of(frame.channels as usize) {
        return Err(DetectError::InvalidArgument(format!(
            "audio buffer length {} is not divisible by channel count {}",
            frame.data.len(),
            frame.channels
        )));
    }
    Ok(normalized_samples(frame.data))
}

fn owned_like(frame: &AudioFrame<'_>, samples: Vec<f32>) -> Result<OwnedAudioFrame> {
    OwnedAudioFrame::new(
        frame.timestamp,
        frame.sample_rate,
        frame.channels,
        AudioBuffer::F32(samples),
    )
}

fn coefficients(spec: BiquadSpec, sample_rate: u32) -> Result<BiquadCoefficients> {
    if !spec.cutoff_hz.is_finite() || spec.cutoff_hz <= 0.0 || !spec.q.is_finite() || spec.q <= 0.0
    {
        return Err(DetectError::InvalidArgument(
            "biquad cutoff must be finite and positive, and q must be finite and positive"
                .to_string(),
        ));
    }
    let nyquist = sample_rate as f32 * 0.5;
    if spec.cutoff_hz >= nyquist {
        return Err(DetectError::InvalidArgument(
            "biquad cutoff must be below Nyquist".to_string(),
        ));
    }

    let omega = 2.0 * std::f32::consts::PI * spec.cutoff_hz / sample_rate as f32;
    let sin = omega.sin();
    let cos = omega.cos();
    let alpha = sin / (2.0 * spec.q);
    let (b0, b1, b2, a0, a1, a2) = match spec.kind {
        BiquadKind::LowPass => (
            (1.0 - cos) * 0.5,
            1.0 - cos,
            (1.0 - cos) * 0.5,
            1.0 + alpha,
            -2.0 * cos,
            1.0 - alpha,
        ),
        BiquadKind::HighPass => (
            (1.0 + cos) * 0.5,
            -(1.0 + cos),
            (1.0 + cos) * 0.5,
            1.0 + alpha,
            -2.0 * cos,
            1.0 - alpha,
        ),
        BiquadKind::BandPass => (alpha, 0.0, -alpha, 1.0 + alpha, -2.0 * cos, 1.0 - alpha),
        BiquadKind::Notch => (1.0, -2.0 * cos, 1.0, 1.0 + alpha, -2.0 * cos, 1.0 - alpha),
    };

    Ok(BiquadCoefficients {
        b0: b0 / a0,
        b1: b1 / a0,
        b2: b2 / a0,
        a1: a1 / a0,
        a2: a2 / a0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use video_analysis_core::{AnalysisEvent, AudioAnalyzer, AudioPipeline, Timebase, Timestamp};
    use video_analysis_ingest::{analyze_audio_source, AudioStreamInfo, SourceMode};

    fn frame(samples: Vec<f32>, channels: u16) -> OwnedAudioFrame {
        OwnedAudioFrame::new(
            Timestamp::new(0, Timebase::new(1, 48_000)),
            48_000,
            channels,
            AudioBuffer::F32(samples),
        )
        .unwrap()
    }

    fn samples(frame: &OwnedAudioFrame) -> &[f32] {
        match &frame.data {
            AudioBuffer::F32(samples) => samples,
            _ => panic!("expected f32 samples"),
        }
    }

    fn sine(freq_hz: f32, sample_rate: u32, len: usize) -> Vec<f32> {
        (0..len)
            .map(|index| {
                let t = index as f32 / sample_rate as f32;
                (2.0 * std::f32::consts::PI * freq_hz * t).sin()
            })
            .collect()
    }

    fn mean_abs(samples: &[f32]) -> f32 {
        samples.iter().map(|sample| sample.abs()).sum::<f32>() / samples.len() as f32
    }

    #[test]
    fn gain_changes_samples_and_preserves_timing() {
        let input = frame(vec![0.25, -0.5], 1);
        let timestamp = input.timestamp;
        let mut processor = AudioProcessor::new().gain(2.0);

        let output = processor.process_frame(input).unwrap().unwrap();

        assert_eq!(output.timestamp, timestamp);
        assert_eq!(samples(&output), &[0.5, -1.0]);
    }

    #[test]
    fn clipping_clamps_samples() {
        let mut processor = AudioProcessor::new().hard_clip(-0.5, 0.5);
        let output = processor
            .process_frame(frame(vec![-1.0, -0.25, 0.75], 1))
            .unwrap()
            .unwrap();

        assert_eq!(samples(&output), &[-0.5, -0.25, 0.5]);
    }

    #[test]
    fn mono_conversion_changes_channel_count() {
        let mut processor = AudioProcessor::new().mono(ChannelMix::Average);
        let output = processor
            .process_frame(frame(vec![1.0, -1.0, 0.5, 0.25], 2))
            .unwrap()
            .unwrap();

        assert_eq!(output.channels, 1);
        assert_eq!(samples(&output), &[0.0, 0.375]);
    }

    #[test]
    fn dc_block_reduces_constant_offset_across_frames() {
        let mut processor = AudioProcessor::new().dc_block(0.95);
        let first = processor
            .process_frame(frame(vec![1.0; 64], 1))
            .unwrap()
            .unwrap();
        let second = processor
            .process_frame(frame(vec![1.0; 64], 1))
            .unwrap()
            .unwrap();

        assert!(mean_abs(samples(&second)) < mean_abs(samples(&first)));
    }

    #[test]
    fn low_pass_attenuates_high_frequency_more_than_low_frequency() {
        let spec = BiquadSpec {
            kind: BiquadKind::LowPass,
            cutoff_hz: 1_000.0,
            q: 0.707,
        };
        let mut low_filter = AudioProcessor::new().biquad(spec);
        let mut high_filter = AudioProcessor::new().biquad(spec);
        let low = low_filter
            .process_frame(frame(sine(200.0, 48_000, 4096), 1))
            .unwrap()
            .unwrap();
        let high = high_filter
            .process_frame(frame(sine(8_000.0, 48_000, 4096), 1))
            .unwrap()
            .unwrap();

        assert!(mean_abs(samples(&low)) > mean_abs(samples(&high)) * 4.0);
    }

    #[test]
    fn high_pass_attenuates_low_frequency_content() {
        let spec = BiquadSpec {
            kind: BiquadKind::HighPass,
            cutoff_hz: 1_000.0,
            q: 0.707,
        };
        let mut low_filter = AudioProcessor::new().biquad(spec);
        let mut high_filter = AudioProcessor::new().biquad(spec);
        let low = low_filter
            .process_frame(frame(sine(80.0, 48_000, 4096), 1))
            .unwrap()
            .unwrap();
        let high = high_filter
            .process_frame(frame(sine(8_000.0, 48_000, 4096), 1))
            .unwrap()
            .unwrap();

        assert!(mean_abs(samples(&high)) > mean_abs(samples(&low)) * 4.0);
    }

    #[derive(Default)]
    struct CountingAnalyzer;

    impl AudioAnalyzer for CountingAnalyzer {
        fn name(&self) -> &str {
            "counting"
        }

        fn process_frame(&mut self, frame: &AudioFrame<'_>) -> Result<Vec<AnalysisEvent>> {
            Ok(vec![
                AnalysisEvent::new(self.name(), "frame").at_timestamp(frame.timestamp)
            ])
        }
    }

    struct VecAudioSource {
        source_info: MediaSourceInfo,
        frames: VecDeque<OwnedAudioFrame>,
    }

    impl VecAudioSource {
        fn new(frames: Vec<OwnedAudioFrame>) -> Self {
            Self {
                source_info: MediaSourceInfo::recorded("memory").with_audio(AudioStreamInfo {
                    sample_rate: 48_000,
                    channels: 1,
                    sample_format: video_analysis_core::AudioSampleFormat::F32,
                }),
                frames: frames.into(),
            }
        }
    }

    impl AudioFrameSource for VecAudioSource {
        fn source_info(&self) -> &MediaSourceInfo {
            &self.source_info
        }

        fn next_audio_frame(&mut self) -> Result<Option<OwnedAudioFrame>> {
            Ok(self.frames.pop_front())
        }
    }

    #[test]
    fn processed_audio_source_can_feed_audio_pipeline() {
        let source = VecAudioSource::new(vec![frame(vec![0.25], 1), frame(vec![0.5], 1)]);
        let processor = AudioProcessor::new().gain(2.0);
        let mut source = ProcessedAudioSource::new(source, processor);
        let mut pipeline = AudioPipeline::builder()
            .analyzer(CountingAnalyzer)
            .build()
            .unwrap();

        let result = analyze_audio_source(&mut source, &mut pipeline, |_| Ok(())).unwrap();

        assert_eq!(result.frames_processed, 2);
        assert_eq!(result.events.len(), 2);
        assert_eq!(source.source_info().mode, SourceMode::Recorded);
    }

    #[test]
    fn stateful_filters_preserve_state_across_chunks() {
        let mut processor = AudioProcessor::new().dc_block(0.95);
        let first = processor
            .process_frame(frame(vec![1.0; 4], 1))
            .unwrap()
            .unwrap();
        let second = processor
            .process_frame(frame(vec![1.0; 4], 1))
            .unwrap()
            .unwrap();

        assert!(samples(&second)[0].abs() < samples(&first)[0].abs());
    }
}
