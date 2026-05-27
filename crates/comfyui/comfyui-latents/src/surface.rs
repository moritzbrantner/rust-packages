//! Library-owned runtime surface for `comfyui-latents`.

use serde::de::DeserializeOwned;
use serde::Deserialize;
use tensor_data::F32Tensor;
use video_analysis_core::runtime::{
    OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation, SurfaceRequest,
    SurfaceResponse,
};

use crate::{LatentBatch, LatentImageSize, LatentMask};

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
                "ComfyUI-oriented latent-space data contracts built on tensor-data.",
                serde_json::json!({"includeOperations": true}),
            ),
            operation(
                "comfy.latents.size",
                "Latent size",
                "Converts image size to ComfyUI latent dimensions.",
                serde_json::json!({"width": 512, "height": 512}),
            ),
            operation(
                "comfy.latents.batchSummary",
                "Latent batch summary",
                "Validates and summarizes a rank-4 latent sample tensor.",
                serde_json::json!({"samples": {"shape": [1, 4, 64, 64], "values": [0.0]}}),
            ),
            operation(
                "comfy.latents.maskCompatibility",
                "Mask compatibility",
                "Checks latent mask compatibility with a latent batch.",
                serde_json::json!({"batch": {"samples": {"shape": [1, 4, 64, 64], "values": [0.0]}}, "mask": {"shape": [64, 64], "values": [0.0]}}),
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
        "comfy.latents.size" => size_value(parse_input(request.input)?)?,
        "comfy.latents.batchSummary" => batch_summary_value(parse_input(request.input)?)?,
        "comfy.latents.maskCompatibility" => mask_compatibility_value(parse_input(request.input)?)?,
        operation => {
            return Err(format!(
                "unsupported operation `{operation}` for {}",
                env!("CARGO_PKG_NAME")
            ))
        }
    };
    Ok(surface_response(operation, value))
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

fn surface_response(operation: OperationId, value: serde_json::Value) -> SurfaceResponse {
    SurfaceResponse {
        operation,
        value,
        diagnostics: Vec::new(),
        artifacts: Vec::new(),
    }
}

fn parse_input<T: DeserializeOwned>(input: serde_json::Value) -> Result<T, String> {
    serde_json::from_value(input).map_err(|error| format!("invalid request: {error}"))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SizeRequest {
    width: u32,
    height: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BatchSummaryRequest {
    samples: TensorPayload,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MaskCompatibilityRequest {
    batch: BatchPayload,
    mask: TensorPayload,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BatchPayload {
    samples: TensorPayload,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TensorPayload {
    shape: Vec<usize>,
    values: Vec<f32>,
}

fn size_value(request: SizeRequest) -> Result<serde_json::Value, String> {
    let size = LatentImageSize::new(request.width, request.height).map_err(|e| e.to_string())?;
    let (latent_height, latent_width) = size.latent_dimensions().map_err(|e| e.to_string())?;
    Ok(serde_json::json!({
        "width": size.width,
        "height": size.height,
        "scaleFactor": LatentImageSize::SCALE_FACTOR,
        "latentHeight": latent_height,
        "latentWidth": latent_width
    }))
}

fn batch_summary_value(request: BatchSummaryRequest) -> Result<serde_json::Value, String> {
    let batch = LatentBatch::new(request.samples.tensor()?).map_err(|e| e.to_string())?;
    Ok(batch_value(&batch))
}

fn mask_compatibility_value(
    request: MaskCompatibilityRequest,
) -> Result<serde_json::Value, String> {
    let batch = LatentBatch::new(request.batch.samples.tensor()?).map_err(|e| e.to_string())?;
    let mask = LatentMask::new(request.mask.tensor()?).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({
        "compatible": mask.compatible_with(&batch),
        "batch": batch_value(&batch),
        "mask": {
            "shape": mask.tensor().shape().dimensions(),
            "rank": mask.rank(),
            "spatialDimensions": mask.spatial_dimensions()
        }
    }))
}

fn batch_value(batch: &LatentBatch) -> serde_json::Value {
    let image_size = batch.image_size().ok();
    serde_json::json!({
        "batchSize": batch.batch_size(),
        "channelCount": batch.channel_count(),
        "latentHeight": batch.latent_height(),
        "latentWidth": batch.latent_width(),
        "imageSize": image_size.map(|size| serde_json::json!({"width": size.width, "height": size.height})),
        "hasMask": batch.mask().is_some()
    })
}

impl TensorPayload {
    fn tensor(self) -> Result<F32Tensor, String> {
        let expected = self.shape.iter().product::<usize>();
        let values = if self.values.len() == 1 && expected > 1 {
            vec![self.values[0]; expected]
        } else {
            self.values
        };
        F32Tensor::from_dims(self.shape, values).map_err(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_rejects_non_divisible_dimensions() {
        let error = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("comfy.latents.size"),
            input: serde_json::json!({"width": 513, "height": 512}),
        })
        .expect_err("non divisible");
        assert!(error.contains("divisible"));
    }

    #[test]
    fn batch_summary_rejects_non_4d_samples() {
        let error = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("comfy.latents.batchSummary"),
            input: serde_json::json!({"samples": {"shape": [64, 64], "values": [0.0]}}),
        })
        .expect_err("rank");
        assert!(error.contains("rank 4"));
    }

    #[test]
    fn mask_compatibility_accepts_rank_2_mask() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("comfy.latents.maskCompatibility"),
            input: serde_json::json!({
                "batch": {"samples": {"shape": [1, 4, 64, 64], "values": [0.0]}},
                "mask": {"shape": [64, 64], "values": [0.0]}
            }),
        })
        .expect("compatibility");
        assert_eq!(response.value["compatible"], true);
    }
}
