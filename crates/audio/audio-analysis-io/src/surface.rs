//! Library-owned runtime surface for `audio-analysis-io`.

use video_analysis_core::runtime::{
    OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation, SurfaceRequest,
    SurfaceResponse,
};

const MAX_SAMPLES: usize = 192_000;

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
                "Audio input planning, waveform batch contracts, and file-oriented audio helpers.",
                serde_json::json!({"includeOperations": true}),
            ),
            operation(
                "audio.io.inputPlan",
                "Input plan",
                "Describes the audio input that would be opened without touching the filesystem.",
                serde_json::json!({"source": "clip.wav", "mode": "recorded", "samplesPerChunk": 4096}),
            ),
            operation(
                "audio.io.waveformBatchSummary",
                "Waveform batch summary",
                "Summarizes an in-memory batch/channel/time waveform tensor.",
                serde_json::json!({"sampleRate": 48000, "waveforms": [[[0.0, 0.5, -0.5]]]}),
            ),
            operation(
                "audio.io.decodePlan",
                "Decode plan",
                "Returns deterministic decode settings and backend requirements without decoding.",
                serde_json::json!({"source": "clip.wav", "target": "mono-f32", "sampleRate": 48000}),
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
        "audio.io.inputPlan" => input_plan_value(request.input)?,
        "audio.io.waveformBatchSummary" => waveform_batch_summary_value(request.input)?,
        "audio.io.decodePlan" => decode_plan_value(request.input)?,
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

fn input_plan_value(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let source = string_field(&input, "source", "stdin")?;
    let mode = string_field(&input, "mode", "recorded")?;
    let samples_per_chunk = positive_u64(&input, "samplesPerChunk", 4096)?;
    Ok(serde_json::json!({
        "source": source,
        "mode": mode,
        "samplesPerChunk": samples_per_chunk,
        "realtime": mode == "live",
        "backend": "ffmpeg",
        "opensExternalProcess": false,
        "notes": ["plan only", "no filesystem or FFmpeg access during surface execution"]
    }))
}

fn waveform_batch_summary_value(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let sample_rate = positive_u64(&input, "sampleRate", 48_000)?;
    let batches = input
        .get("waveforms")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "waveforms must be a batch array".to_string())?;
    let mut total_samples = 0usize;
    let mut batch_shapes = Vec::new();
    for batch in batches {
        let channels = batch
            .as_array()
            .ok_or_else(|| "each waveform batch item must be an array of channels".to_string())?;
        let mut channel_lengths = Vec::new();
        for channel in channels {
            let samples = sample_array(channel)?;
            total_samples += samples.len();
            if total_samples > MAX_SAMPLES {
                return Err(format!(
                    "waveforms must not contain more than {MAX_SAMPLES} samples"
                ));
            }
            channel_lengths.push(samples.len());
        }
        batch_shapes.push(serde_json::json!({
            "channels": channels.len(),
            "timeSteps": channel_lengths.into_iter().max().unwrap_or(0)
        }));
    }
    Ok(serde_json::json!({
        "sampleRate": sample_rate,
        "batchSize": batches.len(),
        "totalSamples": total_samples,
        "durationSeconds": if sample_rate == 0 { 0.0 } else { total_samples as f64 / sample_rate as f64 },
        "batches": batch_shapes
    }))
}

fn decode_plan_value(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let source = string_field(&input, "source", "input")?;
    let target = string_field(&input, "target", "waveform-batch")?;
    Ok(serde_json::json!({
        "source": source,
        "target": target,
        "requestedSampleRate": input.get("sampleRate").and_then(serde_json::Value::as_u64),
        "backend": "ffmpeg",
        "requiresExternalTool": true,
        "executed": false,
        "supportedTargets": ["f32", "mono-f32", "waveform-batch", "wav"]
    }))
}

fn sample_array(value: &serde_json::Value) -> Result<Vec<f32>, String> {
    value
        .as_array()
        .ok_or_else(|| "sample channel must be an array".to_string())?
        .iter()
        .map(|sample| {
            let sample = sample
                .as_f64()
                .ok_or_else(|| "samples must be numbers".to_string())?
                as f32;
            if sample.is_finite() {
                Ok(sample)
            } else {
                Err("samples must be finite".to_string())
            }
        })
        .collect()
}

fn string_field(
    input: &serde_json::Value,
    field: &str,
    default_value: &str,
) -> Result<String, String> {
    Ok(input
        .get(field)
        .and_then(serde_json::Value::as_str)
        .unwrap_or(default_value)
        .to_string())
}

fn positive_u64(input: &serde_json::Value, field: &str, default_value: u64) -> Result<u64, String> {
    input
        .get(field)
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(default_value)
        .checked_add(0)
        .filter(|value| *value > 0)
        .ok_or_else(|| format!("{field} must be positive"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_surface_lists_io_operations() {
        let surface = package_surface();
        let ids = surface
            .operations
            .iter()
            .map(|operation| operation.id.as_str())
            .collect::<Vec<_>>();
        assert!(ids.contains(&"audio.io.inputPlan"));
        assert!(ids.contains(&"audio.io.waveformBatchSummary"));
    }

    #[test]
    fn input_plan_operation_returns_backend() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("audio.io.inputPlan"),
            input: serde_json::json!({"source": "clip.wav"}),
        })
        .expect("input plan");
        assert_eq!(response.value["backend"], "ffmpeg");
        assert_eq!(response.value["opensExternalProcess"], false);
    }

    #[test]
    fn invalid_waveforms_return_error() {
        let error = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("audio.io.waveformBatchSummary"),
            input: serde_json::json!({"waveforms": "bad"}),
        })
        .unwrap_err();
        assert!(error.contains("waveforms"));
    }
}
