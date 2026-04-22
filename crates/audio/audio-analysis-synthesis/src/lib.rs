use data_inversion_core::{Generated, InformationFidelity, InversionMethod, InversionTrace};
use video_analysis_core::{
    AnalysisEvent, AudioBuffer, DetectError, OwnedAudioFrame, Result, Timebase, Timestamp,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Waveform {
    Sine,
    Square,
    Saw,
    Triangle,
    Pulse,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AmplitudeEnvelope {
    pub gain: f32,
    pub attack_seconds: f32,
    pub release_seconds: f32,
}

impl Default for AmplitudeEnvelope {
    fn default() -> Self {
        Self {
            gain: 0.8,
            attack_seconds: 0.005,
            release_seconds: 0.02,
        }
    }
}

impl AmplitudeEnvelope {
    pub fn validate(self) -> Result<()> {
        if !self.gain.is_finite() || self.gain < 0.0 {
            return Err(invalid_argument(
                "envelope gain must be finite and non-negative",
            ));
        }
        if !self.attack_seconds.is_finite()
            || !self.release_seconds.is_finite()
            || self.attack_seconds < 0.0
            || self.release_seconds < 0.0
        {
            return Err(invalid_argument(
                "envelope attack and release must be finite and non-negative",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToneSpec {
    pub frequency_hz: f32,
    pub duration_seconds: f32,
    pub amplitude: f32,
    pub waveform: Waveform,
}

impl ToneSpec {
    pub fn sine(frequency_hz: f32, duration_seconds: f32) -> Self {
        Self {
            frequency_hz,
            duration_seconds,
            amplitude: 0.5,
            waveform: Waveform::Sine,
        }
    }

    pub fn validate(self) -> Result<()> {
        if !self.frequency_hz.is_finite() || self.frequency_hz <= 0.0 {
            return Err(invalid_argument(
                "tone frequency_hz must be finite and greater than zero",
            ));
        }
        if !self.duration_seconds.is_finite() || self.duration_seconds <= 0.0 {
            return Err(invalid_argument(
                "tone duration_seconds must be finite and greater than zero",
            ));
        }
        if !self.amplitude.is_finite() || self.amplitude < 0.0 {
            return Err(invalid_argument(
                "tone amplitude must be finite and non-negative",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToneSegment {
    pub start_seconds: f32,
    pub tone: ToneSpec,
}

impl ToneSegment {
    pub fn validate(self) -> Result<()> {
        if !self.start_seconds.is_finite() || self.start_seconds < 0.0 {
            return Err(invalid_argument(
                "tone segment start_seconds must be finite and non-negative",
            ));
        }
        self.tone.validate()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AudioSynthesisConfig {
    pub sample_rate: u32,
    pub channels: u16,
    pub start_timestamp: Timestamp,
    pub envelope: AmplitudeEnvelope,
}

impl AudioSynthesisConfig {
    pub fn new(sample_rate: u32, channels: u16) -> Result<Self> {
        if sample_rate == 0 || channels == 0 {
            return Err(DetectError::InvalidAudioFormat {
                sample_rate,
                channels,
            });
        }
        if sample_rate > i32::MAX as u32 {
            return Err(invalid_argument("sample_rate is too large for a timebase"));
        }
        Ok(Self {
            sample_rate,
            channels,
            start_timestamp: Timestamp::new(0, Timebase::new(1, sample_rate as i32)),
            envelope: AmplitudeEnvelope::default(),
        })
    }

    pub fn envelope(mut self, envelope: AmplitudeEnvelope) -> Result<Self> {
        envelope.validate()?;
        self.envelope = envelope;
        Ok(self)
    }
}

impl Default for AudioSynthesisConfig {
    fn default() -> Self {
        Self::new(48_000, 1).expect("default audio synthesis config is valid")
    }
}

pub fn synthesize_tone(
    tone: ToneSpec,
    config: AudioSynthesisConfig,
) -> Result<Generated<OwnedAudioFrame>> {
    synthesize_timeline(
        &[ToneSegment {
            start_seconds: 0.0,
            tone,
        }],
        config,
    )
}

pub fn synthesize_timeline(
    segments: &[ToneSegment],
    config: AudioSynthesisConfig,
) -> Result<Generated<OwnedAudioFrame>> {
    if segments.is_empty() {
        return Err(invalid_argument("tone timeline must not be empty"));
    }
    config.envelope.validate()?;
    let channels = config.channels as usize;
    let mut end_sample = 0_usize;
    for segment in segments {
        segment.validate()?;
        let start = seconds_to_samples(segment.start_seconds, config.sample_rate)?;
        let len = seconds_to_samples(segment.tone.duration_seconds, config.sample_rate)?.max(1);
        end_sample = end_sample.max(start + len);
    }

    let mut mono = vec![0.0_f32; end_sample];
    for segment in segments {
        let start = seconds_to_samples(segment.start_seconds, config.sample_rate)?;
        let len = seconds_to_samples(segment.tone.duration_seconds, config.sample_rate)?.max(1);
        for offset in 0..len {
            let t = offset as f32 / config.sample_rate as f32;
            let env = envelope_gain(offset, len, config.sample_rate, config.envelope);
            let sample = wave_sample(segment.tone.waveform, segment.tone.frequency_hz, t)
                * segment.tone.amplitude
                * env;
            mono[start + offset] = (mono[start + offset] + sample).clamp(-1.0, 1.0);
        }
    }

    let mut interleaved = Vec::with_capacity(mono.len() * channels);
    for sample in mono {
        for _ in 0..channels {
            interleaved.push(sample);
        }
    }

    let frame = OwnedAudioFrame::new(
        config.start_timestamp,
        config.sample_rate,
        config.channels,
        AudioBuffer::F32(interleaved),
    )?;
    let trace = InversionTrace::new(
        "tone_timeline",
        "owned_audio_frame",
        InformationFidelity::Interpolated,
    )
    .assumption("phase starts at zero for every synthesized segment")
    .note(
        "waveform",
        InversionMethod::Template,
        "samples are generated from analytic waveform templates",
    )
    .note(
        "samples",
        InversionMethod::Interpolated,
        "continuous tones are sampled at the configured sample rate",
    );
    Ok(Generated::new(frame, trace))
}

pub fn event_to_tone_segment(
    event: &AnalysisEvent,
    default_duration_seconds: f32,
) -> Option<ToneSegment> {
    if !default_duration_seconds.is_finite() || default_duration_seconds <= 0.0 {
        return None;
    }
    let start_seconds = event
        .timestamp
        .map_or(0.0, |timestamp| timestamp.seconds() as f32);
    if let Some(frequency_hz) = parse_pitch_label(&event.label) {
        return Some(ToneSegment {
            start_seconds,
            tone: ToneSpec {
                frequency_hz,
                duration_seconds: default_duration_seconds,
                amplitude: event.score.unwrap_or(0.5).clamp(0.05, 1.0),
                waveform: Waveform::Sine,
            },
        });
    }
    if event.label == "audio:onset" {
        return Some(ToneSegment {
            start_seconds,
            tone: ToneSpec {
                frequency_hz: 1_200.0,
                duration_seconds: default_duration_seconds.min(0.04),
                amplitude: event.score.unwrap_or(0.8).clamp(0.1, 1.0),
                waveform: Waveform::Pulse,
            },
        });
    }
    None
}

pub fn events_to_tone_segments(
    events: &[AnalysisEvent],
    default_duration_seconds: f32,
) -> Vec<ToneSegment> {
    events
        .iter()
        .filter_map(|event| event_to_tone_segment(event, default_duration_seconds))
        .collect()
}

pub fn synthesize_events(
    events: &[AnalysisEvent],
    default_duration_seconds: f32,
    config: AudioSynthesisConfig,
) -> Result<Generated<OwnedAudioFrame>> {
    let segments = events_to_tone_segments(events, default_duration_seconds);
    if segments.is_empty() {
        return Err(invalid_argument(
            "events did not contain pitch or onset labels that can be synthesized",
        ));
    }
    let mut generated = synthesize_timeline(&segments, config)?;
    generated.trace.source_type = "analysis_events".to_string();
    generated.trace = generated.trace.note(
        "events",
        InversionMethod::Inferred,
        "pitch and onset labels are interpreted as tones",
    );
    Ok(generated)
}

fn parse_pitch_label(label: &str) -> Option<f32> {
    label
        .strip_prefix("audio:pitch:")
        .and_then(|value| value.strip_suffix("hz"))
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|value| value.is_finite() && *value > 0.0)
}

fn seconds_to_samples(seconds: f32, sample_rate: u32) -> Result<usize> {
    if !seconds.is_finite() || seconds < 0.0 {
        return Err(invalid_argument("seconds must be finite and non-negative"));
    }
    Ok((seconds * sample_rate as f32).round() as usize)
}

fn envelope_gain(offset: usize, len: usize, sample_rate: u32, envelope: AmplitudeEnvelope) -> f32 {
    let attack_samples = seconds_to_samples(envelope.attack_seconds, sample_rate).unwrap_or(0);
    let release_samples = seconds_to_samples(envelope.release_seconds, sample_rate).unwrap_or(0);
    let attack = if attack_samples > 0 && offset < attack_samples {
        offset as f32 / attack_samples as f32
    } else {
        1.0
    };
    let release_start = len.saturating_sub(release_samples);
    let release = if release_samples > 0 && offset >= release_start {
        (len.saturating_sub(offset) as f32 / release_samples as f32).clamp(0.0, 1.0)
    } else {
        1.0
    };
    envelope.gain * attack.min(release)
}

fn wave_sample(waveform: Waveform, frequency_hz: f32, t: f32) -> f32 {
    let phase = (frequency_hz * t).fract();
    match waveform {
        Waveform::Sine => (2.0 * std::f32::consts::PI * frequency_hz * t).sin(),
        Waveform::Square => {
            if phase < 0.5 {
                1.0
            } else {
                -1.0
            }
        }
        Waveform::Saw => (2.0 * phase) - 1.0,
        Waveform::Triangle => 1.0 - (4.0 * (phase - 0.5).abs()),
        Waveform::Pulse => {
            if phase < 0.08 {
                1.0
            } else {
                0.0
            }
        }
    }
}

fn invalid_argument(message: impl Into<String>) -> DetectError {
    DetectError::InvalidArgument(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthesizes_tone_as_audio_frame() {
        let generated = synthesize_tone(
            ToneSpec::sine(440.0, 0.01),
            AudioSynthesisConfig::new(1_000, 2).unwrap(),
        )
        .unwrap();
        assert_eq!(generated.value.channels, 2);
        assert_eq!(generated.value.samples_per_channel(), 10);
        assert_eq!(generated.trace.fidelity, InformationFidelity::Interpolated);
    }

    #[test]
    fn maps_pitch_event_to_tone() {
        let event = AnalysisEvent::new("audio_pitch", "audio:pitch:220.00hz").score(0.7);
        let segment = event_to_tone_segment(&event, 0.2).unwrap();
        assert_eq!(segment.tone.frequency_hz, 220.0);
        assert_eq!(segment.tone.amplitude, 0.7);
    }
}
