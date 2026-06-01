//! Library-owned runtime surface for `video-analysis-detectors`.

use runtime_core::{
    OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation, SurfaceRequest,
    SurfaceResponse,
};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use video_analysis_core::SceneDetector;

use crate::{
    AdaptiveScoreAlgorithm, ContentScoreAlgorithm, FlashFilter, FlashFilterMode,
    HashScoreAlgorithm, HistogramScoreAlgorithm, WeightedCompositeDetector,
};

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
                "Detector traits, registries, and deterministic detector utilities for video-analysis.",
                serde_json::json!({"includeOperations": true}),
            ),
            operation(
                "video.detectors.registry",
                "Summarize detector registry",
                "Summarizes detector names, supported score algorithms, and deterministic defaults.",
                serde_json::json!({"includeAlgorithms": true}),
            ),
            operation(
                "video.detectors.flashFilter",
                "Run flash filter",
                "Runs the deterministic scene flash filter over threshold decisions.",
                serde_json::json!({
                    "mode": "merge",
                    "length": 15,
                    "frames": [
                        {"frameIndex": 0, "aboveThreshold": true},
                        {"frameIndex": 4, "aboveThreshold": false}
                    ]
                }),
            ),
            operation(
                "video.detectors.compositePlan",
                "Validate composite detector",
                "Builds and validates a weighted composite detector from score algorithms.",
                serde_json::json!({
                    "threshold": 0.5,
                    "minSceneLen": 15,
                    "filterMode": "merge",
                    "components": [
                        {"kind": "content", "weight": 0.7, "threshold": 27.0},
                        {"kind": "histogram", "weight": 0.3, "threshold": 0.15, "bins": 64}
                    ]
                }),
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
        "video.detectors.registry" => registry_value(parse_input(request.input)?),
        "video.detectors.flashFilter" => flash_filter_value(parse_input(request.input)?),
        "video.detectors.compositePlan" => composite_plan_value(parse_input(request.input)?)?,
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
struct RegistryRequest {
    #[serde(default = "default_true")]
    include_algorithms: bool,
}

fn default_true() -> bool {
    true
}

fn registry_value(request: RegistryRequest) -> serde_json::Value {
    serde_json::json!({
        "detectors": [
            {"id": "content", "defaultThreshold": 27.0, "defaultMinSceneLen": 15},
            {"id": "adaptive", "defaultThreshold": 3.0, "defaultMinSceneLen": 15},
            {"id": "threshold", "defaultThreshold": 12.0, "defaultMinSceneLen": 15},
            {"id": "histogram", "defaultThreshold": 0.15, "defaultMinSceneLen": 15},
            {"id": "hash", "defaultThreshold": 0.395, "defaultMinSceneLen": 15}
        ],
        "scoreAlgorithms": if request.include_algorithms {
            serde_json::json!(["content", "adaptive", "histogram", "hash"])
        } else {
            serde_json::json!([])
        },
        "flashFilterModes": ["merge", "suppress"],
        "compositeDefaults": {
            "threshold": 0.5,
            "minSceneLen": 15,
            "filterMode": "merge"
        },
        "externalToolsRequired": false
    })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FlashFilterRequest {
    #[serde(default = "default_flash_mode")]
    mode: String,
    #[serde(default = "default_min_scene_len")]
    length: u64,
    #[serde(default)]
    frames: Vec<ThresholdFrameRequest>,
}

fn default_flash_mode() -> String {
    "merge".to_string()
}

fn default_min_scene_len() -> u64 {
    15
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ThresholdFrameRequest {
    frame_index: u64,
    above_threshold: bool,
}

fn flash_filter_value(request: FlashFilterRequest) -> serde_json::Value {
    let mode = flash_mode(&request.mode);
    let mut filter = FlashFilter::new(mode, request.length);
    let mut total = 0usize;
    let frames = request
        .frames
        .into_iter()
        .map(|frame| {
            let accepted = filter.filter(frame.frame_index, frame.above_threshold);
            total += accepted.len();
            serde_json::json!({
                "frameIndex": frame.frame_index,
                "aboveThreshold": frame.above_threshold,
                "accepted": accepted
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "mode": flash_mode_name(mode),
        "length": request.length,
        "frames": frames,
        "acceptedCount": total,
        "maxBehind": FlashFilter::new(mode, request.length).max_behind()
    })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompositePlanRequest {
    #[serde(default = "default_composite_threshold")]
    threshold: f32,
    #[serde(default = "default_min_scene_len")]
    min_scene_len: u64,
    #[serde(default = "default_flash_mode")]
    filter_mode: String,
    #[serde(default)]
    components: Vec<ComponentRequest>,
}

fn default_composite_threshold() -> f32 {
    0.5
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ComponentRequest {
    kind: String,
    #[serde(default = "default_weight")]
    weight: f32,
    threshold: Option<f32>,
    bins: Option<usize>,
    size: Option<usize>,
    lowpass: Option<usize>,
    window_width: Option<usize>,
    min_content_val: Option<f32>,
}

fn default_weight() -> f32 {
    1.0
}

fn composite_plan_value(request: CompositePlanRequest) -> Result<serde_json::Value, String> {
    if request.components.is_empty() {
        return Err("at least one composite component is required".to_string());
    }

    let filter_mode = flash_mode(&request.filter_mode);
    let mut builder = WeightedCompositeDetector::builder()
        .threshold(request.threshold)
        .min_scene_len(request.min_scene_len)
        .filter_mode(filter_mode);
    let mut component_summaries = Vec::new();
    let mut total_weight = 0.0f32;

    for component in request.components {
        if !component.weight.is_finite() || component.weight <= 0.0 {
            return Err(format!(
                "component `{}` weight must be finite and greater than zero",
                component.kind
            ));
        }
        total_weight += component.weight;
        let threshold = component
            .threshold
            .unwrap_or_else(|| default_threshold(&component.kind));
        let kind = component.kind.as_str();
        builder = match kind {
            "content" => builder.component(ContentScoreAlgorithm::new(threshold), component.weight),
            "adaptive" => builder.component(
                AdaptiveScoreAlgorithm::new(
                    threshold,
                    component.window_width.unwrap_or(2),
                    component.min_content_val.unwrap_or(15.0),
                ),
                component.weight,
            ),
            "histogram" => builder.component(
                HistogramScoreAlgorithm::new(threshold, component.bins.unwrap_or(64)),
                component.weight,
            ),
            "hash" => builder.component(
                HashScoreAlgorithm::new(
                    threshold,
                    component.size.unwrap_or(16),
                    component.lowpass.unwrap_or(2),
                ),
                component.weight,
            ),
            other => return Err(format!("unknown detector component `{other}`")),
        };
        component_summaries.push(serde_json::json!({
            "kind": kind,
            "weight": component.weight,
            "threshold": threshold,
            "normalizedWeight": 0.0
        }));
    }

    let detector = builder.build().map_err(|error| error.to_string())?;
    for summary in &mut component_summaries {
        if let Some(weight) = summary["weight"].as_f64() {
            summary["normalizedWeight"] = serde_json::json!(weight / f64::from(total_weight));
        }
    }

    Ok(serde_json::json!({
        "threshold": request.threshold,
        "minSceneLen": request.min_scene_len,
        "filterMode": flash_mode_name(filter_mode),
        "componentCount": component_summaries.len(),
        "components": component_summaries,
        "totalWeight": total_weight,
        "metricKeys": detector.metric_keys(),
        "eventBufferLen": detector.event_buffer_len(),
        "externalToolsRequired": false
    }))
}

fn default_threshold(kind: &str) -> f32 {
    match kind {
        "content" => 27.0,
        "adaptive" => 3.0,
        "histogram" => 0.15,
        "hash" => 0.395,
        _ => 0.5,
    }
}

fn flash_mode(value: &str) -> FlashFilterMode {
    if value.eq_ignore_ascii_case("suppress") {
        FlashFilterMode::Suppress
    } else {
        FlashFilterMode::Merge
    }
}

fn flash_mode_name(value: FlashFilterMode) -> &'static str {
    match value {
        FlashFilterMode::Merge => "merge",
        FlashFilterMode::Suppress => "suppress",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_surface_lists_renamed_detector_operations() {
        let ids = package_surface()
            .operations
            .into_iter()
            .map(|operation| operation.id.0)
            .collect::<Vec<_>>();
        assert!(ids.contains(&"video.detectors.flashFilter".to_string()));
        assert!(ids.contains(&"video.detectors.compositePlan".to_string()));
        assert!(!ids.contains(&"video.detectors.colorRule".to_string()));
    }

    #[test]
    fn flash_filter_returns_accepted_frames() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("video.detectors.flashFilter"),
            input: serde_json::json!({
                "mode": "suppress",
                "length": 2,
                "frames": [
                    {"frameIndex": 0, "aboveThreshold": true},
                    {"frameIndex": 1, "aboveThreshold": true},
                    {"frameIndex": 2, "aboveThreshold": true}
                ]
            }),
        })
        .expect("flash filter");
        assert_eq!(response.value["acceptedCount"], 1);
        assert!(response.value.get("plan").is_none());
    }

    #[test]
    fn composite_plan_validates_components() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("video.detectors.compositePlan"),
            input: serde_json::json!({
                "components": [{"kind": "content", "weight": 2.0}]
            }),
        })
        .expect("composite plan");
        assert_eq!(response.value["componentCount"], 1);
        assert_eq!(response.value["externalToolsRequired"], false);

        let error = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("video.detectors.compositePlan"),
            input: serde_json::json!({"components": [{"kind": "missing"}]}),
        })
        .expect_err("unknown component");
        assert!(error.contains("unknown detector component"));
    }
}
