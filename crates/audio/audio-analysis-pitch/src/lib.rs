#![doc = include_str!("../README.md")]

use audio_analysis_core::mono_samples;
use video_analysis_core::{AnalysisEvent, AudioAnalyzer, AudioFrame, DetectError, Result};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PitchDetectorConfig {
    pub min_frequency_hz: f32,
    pub max_frequency_hz: f32,
    pub confidence_threshold: f32,
}

impl PitchDetectorConfig {
    pub fn validate(&self) -> Result<()> {
        if !self.min_frequency_hz.is_finite()
            || !self.max_frequency_hz.is_finite()
            || self.min_frequency_hz <= 0.0
            || self.max_frequency_hz <= self.min_frequency_hz
        {
            return Err(DetectError::InvalidArgument(
                "pitch frequency range must be finite and increasing".to_string(),
            ));
        }
        if !self.confidence_threshold.is_finite()
            || !(0.0..=1.0).contains(&self.confidence_threshold)
        {
            return Err(DetectError::InvalidArgument(
                "pitch confidence_threshold must be between 0 and 1".to_string(),
            ));
        }
        Ok(())
    }
}

impl Default for PitchDetectorConfig {
    fn default() -> Self {
        Self {
            min_frequency_hz: 50.0,
            max_frequency_hz: 2_000.0,
            confidence_threshold: 0.6,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PitchEstimate {
    pub frequency_hz: Option<f32>,
    pub confidence: f32,
}

impl PitchEstimate {
    pub fn unpitched(confidence: f32) -> Self {
        Self {
            frequency_hz: None,
            confidence,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AutocorrelationPitchDetector {
    name: String,
    pub config: PitchDetectorConfig,
}

impl AutocorrelationPitchDetector {
    pub fn new(config: PitchDetectorConfig) -> Result<Self> {
        config.validate()?;
        Ok(Self {
            name: "audio_pitch".to_string(),
            config,
        })
    }

    pub fn estimate_frame(&self, frame: &AudioFrame<'_>) -> Result<PitchEstimate> {
        let mono = mono_samples(frame)?;
        self.estimate_samples(&mono.samples, mono.sample_rate)
    }

    pub fn estimate_samples(&self, samples: &[f32], sample_rate: u32) -> Result<PitchEstimate> {
        if sample_rate == 0 {
            return Err(DetectError::InvalidAudioFormat {
                sample_rate,
                channels: 1,
            });
        }
        if samples.len() < 3 {
            return Ok(PitchEstimate::unpitched(0.0));
        }

        let min_lag = (sample_rate as f32 / self.config.max_frequency_hz).floor() as usize;
        let max_lag = (sample_rate as f32 / self.config.min_frequency_hz).ceil() as usize;
        let max_lag = max_lag.min(samples.len().saturating_sub(2));
        if min_lag == 0 || min_lag >= max_lag {
            return Ok(PitchEstimate::unpitched(0.0));
        }

        let centered = remove_dc(samples);
        let mut best_lag = min_lag;
        let mut best_score = 0.0_f32;
        let mut scores = Vec::with_capacity(max_lag - min_lag + 1);

        for lag in min_lag..=max_lag {
            let score = normalized_autocorrelation(&centered, lag);
            if score > best_score {
                best_score = score;
                best_lag = lag;
            }
            scores.push(score);
        }

        let (selected_lag, selected_score) =
            first_confident_peak(min_lag, &scores, self.config.confidence_threshold)
                .unwrap_or((best_lag, best_score));

        if selected_score < self.config.confidence_threshold {
            return Ok(PitchEstimate::unpitched(best_score.max(0.0)));
        }

        let refined_lag = parabolic_lag(selected_lag, min_lag, &scores);
        Ok(PitchEstimate {
            frequency_hz: Some(sample_rate as f32 / refined_lag),
            confidence: selected_score.clamp(0.0, 1.0),
        })
    }
}

impl Default for AutocorrelationPitchDetector {
    fn default() -> Self {
        Self::new(PitchDetectorConfig::default()).expect("default pitch config is valid")
    }
}

impl AudioAnalyzer for AutocorrelationPitchDetector {
    fn name(&self) -> &str {
        &self.name
    }

    fn process_frame(&mut self, frame: &AudioFrame<'_>) -> Result<Vec<AnalysisEvent>> {
        let estimate = self.estimate_frame(frame)?;
        let Some(frequency_hz) = estimate.frequency_hz else {
            return Ok(Vec::new());
        };
        Ok(vec![AnalysisEvent::new(
            self.name(),
            format!("audio:pitch:{frequency_hz:.2}hz"),
        )
        .at_timestamp(frame.timestamp)
        .score(estimate.confidence)])
    }
}

fn remove_dc(samples: &[f32]) -> Vec<f32> {
    let mean = samples.iter().sum::<f32>() / samples.len() as f32;
    samples.iter().map(|sample| sample - mean).collect()
}

fn normalized_autocorrelation(samples: &[f32], lag: usize) -> f32 {
    let len = samples.len().saturating_sub(lag);
    if len == 0 {
        return 0.0;
    }
    let mut numerator = 0.0;
    let mut energy_a = 0.0;
    let mut energy_b = 0.0;
    for index in 0..len {
        let a = samples[index];
        let b = samples[index + lag];
        numerator += a * b;
        energy_a += a * a;
        energy_b += b * b;
    }
    let denom = (energy_a * energy_b).sqrt();
    if denom <= f32::EPSILON {
        0.0
    } else {
        numerator / denom
    }
}

fn first_confident_peak(min_lag: usize, scores: &[f32], threshold: f32) -> Option<(usize, f32)> {
    scores
        .windows(3)
        .enumerate()
        .find(|(_, scores)| {
            scores[1] >= threshold && scores[1] >= scores[0] && scores[1] >= scores[2]
        })
        .map(|(index, scores)| (min_lag + index + 1, scores[1]))
}

fn parabolic_lag(best_lag: usize, min_lag: usize, scores: &[f32]) -> f32 {
    let index = best_lag - min_lag;
    if index == 0 || index + 1 >= scores.len() {
        return best_lag as f32;
    }
    let left = scores[index - 1];
    let center = scores[index];
    let right = scores[index + 1];
    let denom = left - 2.0 * center + right;
    if denom.abs() <= f32::EPSILON {
        best_lag as f32
    } else {
        best_lag as f32 + 0.5 * (left - right) / denom
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use audio_analysis_test_support::{assert_approx_eq, white_noise};
    use video_analysis_core::{AudioBuffer, AudioFrame, Timebase, Timestamp};

    fn sine(freq_hz: f32, sample_rate: u32, seconds: f32) -> Vec<f32> {
        let samples = (sample_rate as f32 * seconds) as usize;
        (0..samples)
            .map(|index| {
                let t = index as f32 / sample_rate as f32;
                (2.0 * std::f32::consts::PI * freq_hz * t).sin()
            })
            .collect()
    }

    #[test]
    fn config_validation_rejects_invalid_values() {
        assert!(PitchDetectorConfig {
            min_frequency_hz: 0.0,
            max_frequency_hz: 2_000.0,
            confidence_threshold: 0.6,
        }
        .validate()
        .is_err());
        assert!(PitchDetectorConfig {
            min_frequency_hz: 200.0,
            max_frequency_hz: 100.0,
            confidence_threshold: 0.6,
        }
        .validate()
        .is_err());
        assert!(PitchDetectorConfig {
            min_frequency_hz: 50.0,
            max_frequency_hz: f32::INFINITY,
            confidence_threshold: 0.6,
        }
        .validate()
        .is_err());
        assert!(PitchDetectorConfig {
            min_frequency_hz: 50.0,
            max_frequency_hz: 2_000.0,
            confidence_threshold: f32::NAN,
        }
        .validate()
        .is_err());
        assert!(PitchDetectorConfig {
            min_frequency_hz: 50.0,
            max_frequency_hz: 2_000.0,
            confidence_threshold: 1.1,
        }
        .validate()
        .is_err());
    }

    #[test]
    fn estimates_pitch_for_sine_wave() {
        let detector = AutocorrelationPitchDetector::default();
        let estimate = detector
            .estimate_samples(&sine(440.0, 44_100, 0.1), 44_100)
            .unwrap();
        let frequency = estimate.frequency_hz.unwrap();
        assert!((frequency - 440.0).abs() < 3.0);
        assert!(estimate.confidence > 0.8);
    }

    #[test]
    fn estimates_pitch_across_reference_frequencies() {
        let detector = AutocorrelationPitchDetector::default();
        for frequency in [80.0, 220.0, 440.0, 1_000.0] {
            let estimate = detector
                .estimate_samples(&sine(frequency, 48_000, 0.12), 48_000)
                .unwrap();
            let actual = estimate.frequency_hz.unwrap();
            assert!(
                (actual - frequency).abs() <= frequency * 0.02,
                "expected {frequency}, got {actual}"
            );
        }
    }

    #[test]
    fn returns_unpitched_for_silence() {
        let detector = AutocorrelationPitchDetector::default();
        let estimate = detector.estimate_samples(&vec![0.0; 2048], 48_000).unwrap();
        assert_eq!(estimate.frequency_hz, None);
        assert_eq!(estimate.confidence, 0.0);
        assert_eq!(
            detector
                .estimate_samples(&[0.0, 1.0], 48_000)
                .unwrap()
                .frequency_hz,
            None
        );
        assert!(detector.estimate_samples(&[0.0; 2048], 0).is_err());
    }

    #[test]
    fn estimates_pitch_with_dc_offset_and_seeded_noise() {
        let detector = AutocorrelationPitchDetector::default();
        let with_dc = sine(220.0, 48_000, 0.12)
            .into_iter()
            .map(|sample| sample * 0.5 + 0.25)
            .collect::<Vec<_>>();
        let dc_estimate = detector.estimate_samples(&with_dc, 48_000).unwrap();
        assert_approx_eq(dc_estimate.frequency_hz.unwrap(), 220.0, 220.0 * 0.02);

        let mut noisy = sine(440.0, 48_000, 0.12);
        for (sample, noise) in noisy.iter_mut().zip(white_noise(7, 48_000)) {
            *sample = *sample * 0.85 + noise * 0.05;
        }
        let noisy_estimate = detector.estimate_samples(&noisy, 48_000).unwrap();
        assert_approx_eq(noisy_estimate.frequency_hz.unwrap(), 440.0, 440.0 * 0.05);
    }

    #[test]
    fn emits_audio_pipeline_event() {
        let mut detector = AutocorrelationPitchDetector::default();
        let samples = AudioBuffer::F32(sine(220.0, 44_100, 0.1));
        let frame = AudioFrame::new(
            Timestamp::new(0, Timebase::new(1, 44_100)),
            44_100,
            1,
            &samples,
        )
        .unwrap();
        let events = detector.process_frame(&frame).unwrap();
        assert_eq!(events.len(), 1);
        assert!(events[0].label.starts_with("audio:pitch:"));
        assert_eq!(events[0].timestamp, Some(frame.timestamp));
        assert!(events[0].score.unwrap() > 0.6);
    }
}
