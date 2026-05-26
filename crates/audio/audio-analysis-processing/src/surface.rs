//! Library-owned runtime surface for `audio-analysis-processing`.

use audio_analysis_core::{mean_absolute, peak, rms, ChannelMix};
use video_analysis_core::runtime::{
    OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation, SurfaceRequest,
    SurfaceResponse,
};
use video_analysis_core::{AudioBuffer, OwnedAudioFrame, Timebase, Timestamp};

use crate::{AudioEnergyAnalyzer, AudioProcessor, NoiseGateSpec};

const MAX_SAMPLES: usize = 192_000;
const DEFAULT_PREVIEW_SAMPLES: usize = 1024;

/// Returns the package surface exposed by every transport wrapper.
pub fn package_surface() -> PackageSurface {
    PackageSurface {
        library: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: RuntimeCapabilities::pure_rust(),
        operations: vec![
            operation(
                "describe",
                "Describe package",
                "Realtime-safe audio transforms and processed sources for video-analysis.",
                serde_json::json!({"includeOperations": true}),
            ),
            operation(
                "audio.processing.apply",
                "Apply processing",
                "Applies an in-memory gain/clip/mono/noise-gate chain to normalized samples.",
                serde_json::json!({"samples": [0.0, 0.5, -0.5], "sampleRate": 48000, "channels": 1, "gain": 0.8, "clipMin": -0.9, "clipMax": 0.9}),
            ),
            operation(
                "audio.processing.energy",
                "Energy",
                "Returns RMS, peak, mean absolute value, and silence/loud labels.",
                serde_json::json!({"samples": [0.0, 0.5, -0.5], "sampleRate": 48000, "channels": 1}),
            ),
            operation(
                "audio.processing.chainSummary",
                "Chain summary",
                "Describes the deterministic transform chain that would be applied.",
                serde_json::json!({"gain": 0.8, "mono": true, "noiseGateThreshold": 0.01}),
            ),
        ],
    }
}

fn operation(
    id: &str,
    name: &str,
    description: &str,
    example_request: serde_json::Value,
) -> SurfaceOperation {
    SurfaceOperation {
        id: OperationId::new(id),
        name: name.to_string(),
        description: Some(description.to_string()),
        input_schema: serde_json::json!({"type": "object", "additionalProperties": true}),
        output_schema: serde_json::json!({"type": "object"}),
        example_request,
        wasm_supported: true,
        server_supported: true,
    }
}

/// Runs one library-owned operation.
pub fn run_surface_operation(request: SurfaceRequest) -> Result<SurfaceResponse, String> {
    let operation = request.operation.clone();
    let value = match request.operation.as_str() {
        "describe" => describe_value(request.input),
        "audio.processing.apply" => apply_value(request.input)?,
        "audio.processing.energy" => energy_value(request.input)?,
        "audio.processing.chainSummary" => chain_summary_value(request.input)?,
        operation => {
            return Err(format!(
                "unsupported operation `{operation}` for {}",
                env!("CARGO_PKG_NAME")
            ));
        }
    };
    Ok(response(operation, value))
}

fn response(operation: OperationId, value: serde_json::Value) -> SurfaceResponse {
    SurfaceResponse {
        operation,
        value,
        diagnostics: Vec::new(),
        artifacts: Vec::new(),
    }
}

fn describe_value(input: serde_json::Value) -> serde_json::Value {
    let surface = package_surface();
    serde_json::json!({
        "library": surface.library,
        "version": surface.version,
        "operationCount": surface.operations.len(),
        "operations": surface.operations.iter().map(|operation| operation.id.as_str()).collect::<Vec<_>>(),
        "input": input
    })
}

fn apply_value(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let samples = sample_array(&input, "samples")?;
    let sample_rate = sample_rate(&input)?;
    let channels = channels(&input)?;
    let mut processor = AudioProcessor::new();
    if let Some(gain) = finite_f32(&input, "gain")? {
        processor = processor.gain(gain);
    }
    if input
        .get("mono")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        processor = processor.mono(ChannelMix::Average);
    }
    if input.get("clipMin").is_some() || input.get("clipMax").is_some() {
        processor = processor.hard_clip(
            finite_f32(&input, "clipMin")?.unwrap_or(-1.0),
            finite_f32(&input, "clipMax")?.unwrap_or(1.0),
        );
    }
    if let Some(threshold) = finite_f32(&input, "noiseGateThreshold")? {
        processor = processor.noise_gate(NoiseGateSpec {
            threshold,
            attenuation: finite_f32(&input, "noiseGateAttenuation")?.unwrap_or(0.0),
        });
    }
    let frame = OwnedAudioFrame::new(
        Timestamp::new(0, Timebase::new(1, sample_rate as i32)),
        sample_rate,
        channels,
        AudioBuffer::F32(samples),
    )
    .map_err(|error| error.to_string())?;
    let processed = processor
        .process_frame(frame)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "processing chain produced no frame".to_string())?;
    let processed_samples = match &processed.data {
        AudioBuffer::F32(samples) => samples.as_slice(),
        _ => return Err("processing output was not f32".to_string()),
    };
    Ok(serde_json::json!({
        "sampleRate": processed.sample_rate,
        "channels": processed.channels,
        "sampleCount": processed_samples.len(),
        "samplesPerChannel": processed.samples_per_channel(),
        "rms": rms(processed_samples),
        "peak": peak(processed_samples),
        "samplePreview": preview(processed_samples, preview_limit(&input)?)
    }))
}

fn energy_value(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let samples = sample_array(&input, "samples")?;
    let sample_rate = sample_rate(&input)?;
    let channels = channels(&input)?;
    let silence_threshold = finite_f32(&input, "silenceThreshold")?.unwrap_or(0.01);
    let loud_threshold = finite_f32(&input, "loudThreshold")?.unwrap_or(0.5);
    AudioEnergyAnalyzer::new(silence_threshold, loud_threshold)
        .map_err(|error| error.to_string())?;
    let level = rms(&samples);
    Ok(serde_json::json!({
        "sampleRate": sample_rate,
        "channels": channels,
        "sampleCount": samples.len(),
        "rms": level,
        "peak": peak(&samples),
        "meanAbsolute": mean_absolute(&samples),
        "isSilent": level < silence_threshold,
        "isLoud": level >= loud_threshold
    }))
}

fn chain_summary_value(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let mut transforms = Vec::new();
    if input.get("gain").is_some() {
        transforms.push(serde_json::json!({"name": "gain", "linear": finite_f32(&input, "gain")?}));
    }
    if input
        .get("mono")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        transforms.push(serde_json::json!({"name": "mono", "mix": "average"}));
    }
    if input.get("clipMin").is_some() || input.get("clipMax").is_some() {
        transforms.push(serde_json::json!({
            "name": "hard_clip",
            "min": finite_f32(&input, "clipMin")?.unwrap_or(-1.0),
            "max": finite_f32(&input, "clipMax")?.unwrap_or(1.0)
        }));
    }
    if input.get("noiseGateThreshold").is_some() {
        transforms.push(serde_json::json!({
            "name": "noise_gate",
            "threshold": finite_f32(&input, "noiseGateThreshold")?,
            "attenuation": finite_f32(&input, "noiseGateAttenuation")?.unwrap_or(0.0)
        }));
    }
    Ok(serde_json::json!({
        "transformCount": transforms.len(),
        "transforms": transforms,
        "outputSampleFormat": "f32"
    }))
}

fn sample_array(input: &serde_json::Value, field: &str) -> Result<Vec<f32>, String> {
    let values = input
        .get(field)
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| format!("{field} must be an array"))?;
    if values.len() > MAX_SAMPLES {
        return Err(format!(
            "{field} must not contain more than {MAX_SAMPLES} samples"
        ));
    }
    values
        .iter()
        .map(|value| {
            let sample = value
                .as_f64()
                .ok_or_else(|| format!("{field} must contain only numbers"))?
                as f32;
            if sample.is_finite() {
                Ok(sample)
            } else {
                Err(format!("{field} must contain only finite numbers"))
            }
        })
        .collect()
}

fn sample_rate(input: &serde_json::Value) -> Result<u32, String> {
    let value = input
        .get("sampleRate")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(48_000);
    u32::try_from(value)
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| "sampleRate must be a positive u32".to_string())
}

fn channels(input: &serde_json::Value) -> Result<u16, String> {
    let value = input
        .get("channels")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(1);
    u16::try_from(value)
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| "channels must be a positive u16".to_string())
}

fn finite_f32(input: &serde_json::Value, field: &str) -> Result<Option<f32>, String> {
    let Some(value) = input.get(field) else {
        return Ok(None);
    };
    let value = value
        .as_f64()
        .ok_or_else(|| format!("{field} must be a number"))? as f32;
    if value.is_finite() {
        Ok(Some(value))
    } else {
        Err(format!("{field} must be finite"))
    }
}

fn preview_limit(input: &serde_json::Value) -> Result<usize, String> {
    let value = input
        .get("previewSamples")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(DEFAULT_PREVIEW_SAMPLES as u64);
    usize::try_from(value)
        .ok()
        .filter(|value| *value > 0)
        .map(|value| value.min(DEFAULT_PREVIEW_SAMPLES))
        .ok_or_else(|| "previewSamples must be positive".to_string())
}

fn preview(samples: &[f32], limit: usize) -> Vec<f32> {
    samples.iter().copied().take(limit).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_surface_lists_processing_operations() {
        let surface = package_surface();
        let ids = surface
            .operations
            .iter()
            .map(|operation| operation.id.as_str())
            .collect::<Vec<_>>();
        assert!(ids.contains(&"audio.processing.apply"));
        assert!(ids.contains(&"audio.processing.energy"));
    }

    #[test]
    fn energy_operation_returns_summary() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("audio.processing.energy"),
            input: serde_json::json!({"samples": [0.0, 1.0, -1.0], "sampleRate": 3, "channels": 1}),
        })
        .expect("energy");
        assert!(response.value["rms"].as_f64().unwrap() > 0.0);
    }

    #[test]
    fn invalid_samples_return_error() {
        let error = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("audio.processing.energy"),
            input: serde_json::json!({"samples": "bad"}),
        })
        .unwrap_err();
        assert!(error.contains("samples"));
    }
}
