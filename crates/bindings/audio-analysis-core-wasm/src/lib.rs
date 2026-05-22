//! WASM bindings for browser-friendly audio analysis helpers.

use audio_analysis_core::{interleaved_to_mono, mean_absolute, peak, rms, ChannelMix, FrameSpec};
use audio_analysis_fourier::{zero_crossing_rate, FourierTransform};
use audio_analysis_pitch::{AutocorrelationPitchDetector, PitchDetectorConfig};
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::{from_value, Serializer};
use video_analysis_core::{AudioBuffer, DetectError};
use wasm_bindgen::prelude::*;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawAudioAnalysisOptions {
    sample_rate: Option<u32>,
    channels: Option<u16>,
    channel_mix: Option<String>,
    frame_size: Option<usize>,
    hop_size: Option<usize>,
    fft_size: Option<usize>,
    min_frequency_hz: Option<f32>,
    max_frequency_hz: Option<f32>,
    confidence_threshold: Option<f32>,
}

impl Default for RawAudioAnalysisOptions {
    fn default() -> Self {
        Self {
            sample_rate: Some(48_000),
            channels: Some(1),
            channel_mix: Some("average".to_string()),
            frame_size: Some(1024),
            hop_size: Some(512),
            fft_size: Some(2048),
            min_frequency_hz: None,
            max_frequency_hz: None,
            confidence_threshold: None,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RawAudioAnalysis {
    sample_rate: u32,
    channels: u16,
    sample_count: usize,
    samples_per_channel: usize,
    duration_seconds: f64,
    rms: f32,
    peak: f32,
    mean_absolute: f32,
    zero_crossing_rate: f32,
    frame_count: usize,
    dominant_frequency_hz: Option<f32>,
    pitch: RawPitchEstimate,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RawPitchEstimate {
    frequency_hz: Option<f32>,
    confidence: f32,
    midi_note: Option<f32>,
    note_name: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RawFramePlan {
    frame_size: usize,
    hop_size: usize,
    sample_count: usize,
    frame_count: usize,
    starts: Vec<usize>,
}

#[wasm_bindgen(js_name = analyzeAudioSamples)]
/// Analyzes interleaved audio samples with core level, frame, FFT, and pitch helpers.
pub fn analyze_audio_samples(
    samples: Vec<f32>,
    options: Option<JsValue>,
) -> Result<JsValue, JsValue> {
    to_js_value(&analyze_audio_samples_data(samples, options)?)
}

#[wasm_bindgen(js_name = mixToMono)]
/// Mixes interleaved samples to mono with either average or first-channel mixing.
pub fn mix_to_mono(
    samples: Vec<f32>,
    channels: u16,
    mix: Option<String>,
) -> Result<JsValue, JsValue> {
    let mix = parse_channel_mix(mix.as_deref())?;
    let mono =
        interleaved_to_mono(&AudioBuffer::F32(samples), channels, mix).map_err(into_js_error)?;
    to_js_value(&mono)
}

#[wasm_bindgen(js_name = planAudioFrames)]
/// Returns frame start offsets for a frame size and hop size.
pub fn plan_audio_frames(
    samples_len: usize,
    frame_size: usize,
    hop_size: usize,
) -> Result<JsValue, JsValue> {
    to_js_value(&plan_audio_frames_data(samples_len, frame_size, hop_size)?)
}

fn analyze_audio_samples_data(
    samples: Vec<f32>,
    options: Option<JsValue>,
) -> Result<RawAudioAnalysis, JsValue> {
    let options = read_options(options)?;
    let sample_rate = options.sample_rate.unwrap_or(48_000);
    let channels = options.channels.unwrap_or(1);
    let channel_mix = parse_channel_mix(options.channel_mix.as_deref())?;
    let mono = interleaved_to_mono(&AudioBuffer::F32(samples), channels, channel_mix)
        .map_err(into_js_error)?;
    let frame_size = options.frame_size.unwrap_or(1024);
    let hop_size = options.hop_size.unwrap_or(512);
    let frame_count = FrameSpec::new(frame_size, hop_size)
        .map_err(into_js_error)?
        .frame_count(mono.len());

    let dominant_frequency_hz = if mono.is_empty() {
        None
    } else {
        let fft_size = options.fft_size.unwrap_or(2048);
        Some(
            FourierTransform::new(fft_size)
                .map_err(into_js_error)?
                .analyze_samples(&mono, sample_rate)
                .map_err(into_js_error)?
                .dominant_frequency_hz(),
        )
        .flatten()
    };

    let pitch_config = pitch_config_from_options(&options)?;
    let pitch_estimate = AutocorrelationPitchDetector::new(pitch_config)
        .map_err(into_js_error)?
        .estimate_samples(&mono, sample_rate)
        .map_err(into_js_error)?;
    let samples_per_channel = mono.len();

    Ok(RawAudioAnalysis {
        sample_rate,
        channels,
        sample_count: samples_per_channel * channels as usize,
        samples_per_channel,
        duration_seconds: if sample_rate == 0 {
            0.0
        } else {
            samples_per_channel as f64 / sample_rate as f64
        },
        rms: rms(&mono),
        peak: peak(&mono),
        mean_absolute: mean_absolute(&mono),
        zero_crossing_rate: zero_crossing_rate(&mono),
        frame_count,
        dominant_frequency_hz,
        pitch: RawPitchEstimate {
            frequency_hz: pitch_estimate.frequency_hz,
            confidence: pitch_estimate.confidence,
            midi_note: pitch_estimate.midi_note(),
            note_name: pitch_estimate.note_name(),
        },
    })
}

fn plan_audio_frames_data(
    samples_len: usize,
    frame_size: usize,
    hop_size: usize,
) -> Result<RawFramePlan, JsValue> {
    let spec = FrameSpec::new(frame_size, hop_size).map_err(into_js_error)?;
    let zeroes = vec![0.0; samples_len];
    Ok(RawFramePlan {
        frame_size,
        hop_size,
        sample_count: samples_len,
        frame_count: spec.frame_count(samples_len),
        starts: spec.frames(&zeroes).map(|(start, _)| start).collect(),
    })
}

fn read_options(options: Option<JsValue>) -> Result<RawAudioAnalysisOptions, JsValue> {
    match options {
        Some(value) if !value.is_null() && !value.is_undefined() => {
            from_value(value).map_err(into_deserialize_js_error)
        }
        _ => Ok(RawAudioAnalysisOptions::default()),
    }
}

fn pitch_config_from_options(
    options: &RawAudioAnalysisOptions,
) -> Result<PitchDetectorConfig, JsValue> {
    let default = PitchDetectorConfig::default();
    let config = PitchDetectorConfig {
        min_frequency_hz: options.min_frequency_hz.unwrap_or(default.min_frequency_hz),
        max_frequency_hz: options.max_frequency_hz.unwrap_or(default.max_frequency_hz),
        confidence_threshold: options
            .confidence_threshold
            .unwrap_or(default.confidence_threshold),
    };
    config.validate().map_err(into_js_error)?;
    Ok(config)
}

fn parse_channel_mix(value: Option<&str>) -> Result<ChannelMix, JsValue> {
    match value.unwrap_or("average") {
        "average" => Ok(ChannelMix::Average),
        "first" => Ok(ChannelMix::First),
        other => Err(JsValue::from_str(&format!(
            "unsupported channel mix `{other}`; expected `average` or `first`"
        ))),
    }
}

fn to_js_value<T: Serialize>(value: &T) -> Result<JsValue, JsValue> {
    value
        .serialize(&Serializer::json_compatible())
        .map_err(into_deserialize_js_error)
}

fn into_js_error(error: DetectError) -> JsValue {
    JsValue::from_str(&error.to_string())
}

fn into_deserialize_js_error(error: serde_wasm_bindgen::Error) -> JsValue {
    JsValue::from_str(&error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyzes_sine_wave_with_level_frequency_and_pitch() {
        let sample_rate = 48_000;
        let samples = (0..4096)
            .map(|index| {
                let phase = index as f32 * 440.0 * std::f32::consts::TAU / sample_rate as f32;
                phase.sin()
            })
            .collect::<Vec<_>>();

        let analysis = analyze_audio_samples_data(samples, None).unwrap();

        assert_eq!(analysis.sample_rate, sample_rate);
        assert!(analysis.peak > 0.99);
        assert!(analysis.rms > 0.6);
        assert!(analysis.dominant_frequency_hz.unwrap() > 420.0);
        assert!(analysis.pitch.frequency_hz.unwrap() > 420.0);
        assert_eq!(analysis.pitch.note_name.as_deref(), Some("A4"));
    }

    #[test]
    fn plans_frames_with_shared_frame_spec() {
        let plan = plan_audio_frames_data(10, 4, 3).unwrap();
        assert_eq!(plan.frame_count, 3);
        assert_eq!(plan.starts, vec![0, 3, 6]);
    }
}
