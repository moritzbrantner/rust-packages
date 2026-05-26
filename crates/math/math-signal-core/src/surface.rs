//! Library-owned runtime surface for `math-signal-core`.

use serde::Deserialize;
use video_analysis_core::runtime::{
    OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation, SurfaceRequest,
    SurfaceResponse,
};

use crate::{
    design_parametric_biquad, resample_indices, FrameStride, InterpolationMode,
    ParametricBiquadDesign, ResampleSpec, SampleRate,
};

const DEFAULT_PREVIEW: usize = 16;
const MAX_PREVIEW: usize = 256;
const MAX_VALUES: usize = 100_000;

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
                "Shared signal-domain math for windows, frame strides, resampling, and biquad design.",
                serde_json::json!({"includeOperations": true}),
            ),
            operation(
                "signal.frames",
                "Signal frames",
                "Computes frame count and preview mean/RMS summaries for a finite mono sample buffer.",
                serde_json::json!({"samples": [0.0, 1.0, 0.0, -1.0], "frameSize": 2, "hopSize": 1}),
            ),
            operation(
                "signal.resamplePlan",
                "Resample plan",
                "Returns output length and source-position preview indices for a sample-rate conversion.",
                serde_json::json!({"inputRate": 48_000, "outputRate": 16_000, "inputLen": 480}),
            ),
            operation(
                "signal.filterDesign",
                "Biquad filter design",
                "Designs normalized biquad coefficients for supported filter kinds.",
                serde_json::json!({"kind": "lowPass", "sampleRate": 48_000, "frequencyHz": 1_000.0, "q": 0.707}),
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
        "signal.frames" => frames_value(parse_input(request.input)?)?,
        "signal.resamplePlan" => resample_plan_value(parse_input(request.input)?)?,
        "signal.filterDesign" => filter_design_value(parse_input(request.input)?)?,
        operation => {
            return Err(format!(
                "unsupported operation `{operation}` for {}",
                env!("CARGO_PKG_NAME")
            ));
        }
    };
    Ok(response(operation, value))
}

fn describe_value(input: serde_json::Value) -> serde_json::Value {
    let surface = package_surface();
    serde_json::json!({
        "library": surface.library,
        "version": surface.version,
        "operationCount": surface.operations.len(),
        "operations": surface
            .operations
            .iter()
            .map(|operation| operation.id.as_str())
            .collect::<Vec<_>>(),
        "input": input
    })
}

fn response(operation: OperationId, value: serde_json::Value) -> SurfaceResponse {
    SurfaceResponse {
        operation,
        value,
        diagnostics: Vec::new(),
        artifacts: Vec::new(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FramesRequest {
    samples: Vec<f32>,
    frame_size: usize,
    hop_size: usize,
    #[serde(default = "default_preview")]
    preview_frames: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResamplePlanRequest {
    input_rate: u32,
    output_rate: u32,
    input_len: usize,
    #[serde(default = "default_resample_mode")]
    mode: String,
    #[serde(default = "default_preview")]
    preview_indices: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FilterDesignRequest {
    kind: String,
    sample_rate: u32,
    frequency_hz: f32,
    q: f32,
    #[serde(default)]
    gain_db: Option<f32>,
}

fn frames_value(request: FramesRequest) -> Result<serde_json::Value, String> {
    validate_values(&request.samples)?;
    let stride = FrameStride::new(request.frame_size, request.hop_size)
        .map_err(|error| error.to_string())?;
    let frame_count = stride.frame_count(request.samples.len());
    let preview_count = request.preview_frames.min(MAX_PREVIEW).min(frame_count);
    let frames = (0..preview_count)
        .map(|index| {
            let start = index * request.hop_size;
            let frame = &request.samples[start..start + request.frame_size];
            let mean = frame.iter().sum::<f32>() / frame.len() as f32;
            let rms = (frame.iter().map(|sample| sample * sample).sum::<f32>()
                / frame.len() as f32)
                .sqrt();
            serde_json::json!({
                "start": start,
                "len": frame.len(),
                "mean": mean,
                "rms": rms
            })
        })
        .collect::<Vec<_>>();
    Ok(serde_json::json!({
        "sampleCount": request.samples.len(),
        "frameSize": request.frame_size,
        "hopSize": request.hop_size,
        "frameCount": frame_count,
        "frames": frames
    }))
}

fn resample_plan_value(request: ResamplePlanRequest) -> Result<serde_json::Value, String> {
    if request.input_len > MAX_VALUES {
        return Err(format!("inputLen must not exceed {MAX_VALUES}"));
    }
    let input = SampleRate::new(request.input_rate).map_err(|error| error.to_string())?;
    let output = SampleRate::new(request.output_rate).map_err(|error| error.to_string())?;
    let mode = parse_interpolation_mode(&request.mode)?;
    let spec = ResampleSpec::new(input, output, mode).map_err(|error| error.to_string())?;
    let ratio = spec.ratio().as_f64();
    let output_len = if request.input_len == 0 {
        0
    } else {
        ((request.input_len as f64) * ratio).round().max(1.0) as usize
    };
    let preview_len = request.preview_indices.min(MAX_PREVIEW).min(output_len);
    let indices = resample_indices(spec, preview_len).map_err(|error| error.to_string())?;
    Ok(serde_json::json!({
        "inputRate": request.input_rate,
        "outputRate": request.output_rate,
        "ratio": ratio,
        "inputLen": request.input_len,
        "outputLen": output_len,
        "indicesPreview": indices
    }))
}

fn filter_design_value(request: FilterDesignRequest) -> Result<serde_json::Value, String> {
    let sample_rate = SampleRate::new(request.sample_rate).map_err(|error| error.to_string())?;
    let design = parse_filter_kind(&request.kind, request.gain_db)?;
    let coefficients =
        design_parametric_biquad(design, sample_rate, request.frequency_hz, request.q)
            .map_err(|error| error.to_string())?;
    Ok(serde_json::json!({
        "kind": request.kind,
        "sampleRate": request.sample_rate,
        "frequencyHz": request.frequency_hz,
        "q": request.q,
        "coefficients": {
            "b0": coefficients.b0,
            "b1": coefficients.b1,
            "b2": coefficients.b2,
            "a1": coefficients.a1,
            "a2": coefficients.a2
        }
    }))
}

fn parse_interpolation_mode(mode: &str) -> Result<InterpolationMode, String> {
    match mode {
        "nearest" => Ok(InterpolationMode::Nearest),
        "linear" => Ok(InterpolationMode::Linear),
        _ => Err(format!("unsupported interpolation mode `{mode}`")),
    }
}

fn parse_filter_kind(kind: &str, gain_db: Option<f32>) -> Result<ParametricBiquadDesign, String> {
    match kind {
        "lowPass" => Ok(ParametricBiquadDesign::LowPass),
        "highPass" => Ok(ParametricBiquadDesign::HighPass),
        "bandPass" => Ok(ParametricBiquadDesign::BandPass),
        "notch" => Ok(ParametricBiquadDesign::Notch),
        "peakingEq" => Ok(ParametricBiquadDesign::PeakingEq {
            gain_db: finite_gain(gain_db)?,
        }),
        "lowShelf" => Ok(ParametricBiquadDesign::LowShelf {
            gain_db: finite_gain(gain_db)?,
        }),
        "highShelf" => Ok(ParametricBiquadDesign::HighShelf {
            gain_db: finite_gain(gain_db)?,
        }),
        "allPass" => Ok(ParametricBiquadDesign::AllPass),
        _ => Err(format!("unsupported filter kind `{kind}`")),
    }
}

fn finite_gain(gain_db: Option<f32>) -> Result<f32, String> {
    let gain_db = gain_db.unwrap_or(0.0);
    if !gain_db.is_finite() {
        return Err("gainDb must be finite".to_string());
    }
    Ok(gain_db)
}

fn validate_values(values: &[f32]) -> Result<(), String> {
    if values.len() > MAX_VALUES {
        return Err(format!("samples must not exceed {MAX_VALUES}"));
    }
    if values.iter().any(|value| !value.is_finite()) {
        return Err("samples must be finite".to_string());
    }
    Ok(())
}

fn parse_input<T: for<'de> Deserialize<'de>>(input: serde_json::Value) -> Result<T, String> {
    serde_json::from_value(input).map_err(|error| format!("invalid request: {error}"))
}

fn default_preview() -> usize {
    DEFAULT_PREVIEW
}

fn default_resample_mode() -> String {
    "linear".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_return_preview_stats() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("signal.frames"),
            input: serde_json::json!({"samples": [0.0, 1.0, 0.0], "frameSize": 2, "hopSize": 1}),
        })
        .expect("frames operation");

        assert_eq!(response.value["sampleCount"], 3);
        assert_eq!(response.value["frameCount"], 2);
        assert_eq!(response.value["frames"][0]["start"], 0);
    }

    #[test]
    fn resample_plan_returns_indices() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("signal.resamplePlan"),
            input: serde_json::json!({"inputRate": 4, "outputRate": 8, "inputLen": 3, "previewIndices": 3}),
        })
        .expect("resample plan operation");

        assert_eq!(response.value["outputLen"], 6);
        assert_eq!(
            response.value["indicesPreview"],
            serde_json::json!([0.0, 0.5, 1.0])
        );
    }

    #[test]
    fn filter_design_returns_coefficients() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("signal.filterDesign"),
            input: serde_json::json!({"kind": "lowPass", "sampleRate": 48_000, "frequencyHz": 1_000.0, "q": 0.707}),
        })
        .expect("filter design operation");

        assert_eq!(response.value["kind"], "lowPass");
        assert!(response.value["coefficients"]["b0"].is_number());
    }
}
