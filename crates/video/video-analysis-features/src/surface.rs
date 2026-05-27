//! Library-owned runtime surface for `video-analysis-features`.

use serde::de::DeserializeOwned;
use serde::Deserialize;
use video_analysis_core::runtime::{
    OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation, SurfaceRequest,
    SurfaceResponse,
};
use video_analysis_dataset::AnalysisDataset;

use crate::{
    AudioEventStatsExtractor, FeaturePipeline, FeatureVectorMeanExtractor,
    ObservationLabelHistogramExtractor, SceneStatsExtractor, TrackSummaryExtractor,
    TranscriptStatsExtractor,
};

const DEFAULT_EXTRACTORS: &[&str] = &[
    "scene_stats",
    "observation_label_histogram",
    "transcript_stats",
    "audio_event_stats",
    "track_summary",
    "feature_vector_mean",
];

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
                "Feature extraction over retained video-analysis datasets.",
                serde_json::json!({"includeOperations": true}),
            ),
            operation(
                "video.features.extract",
                "Extract features",
                "Runs selected deterministic feature extractors over a retained dataset.",
                serde_json::json!({
                    "dataset": {"metadata": {}, "records": []},
                    "extractors": DEFAULT_EXTRACTORS
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
        "video.features.extract" => extract_value(parse_input(request.input)?)?,
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
struct ExtractRequest {
    dataset: AnalysisDataset,
    #[serde(default)]
    extractors: Vec<String>,
}

fn extract_value(request: ExtractRequest) -> Result<serde_json::Value, String> {
    let extractor_names = if request.extractors.is_empty() {
        DEFAULT_EXTRACTORS
            .iter()
            .map(|value| (*value).to_string())
            .collect::<Vec<_>>()
    } else {
        request.extractors
    };

    let mut builder = FeaturePipeline::builder();
    for name in &extractor_names {
        builder = match name.as_str() {
            "scene_stats" => builder.extractor(SceneStatsExtractor),
            "observation_label_histogram" => {
                builder.extractor(ObservationLabelHistogramExtractor::default())
            }
            "transcript_stats" => builder.extractor(TranscriptStatsExtractor),
            "audio_event_stats" => builder.extractor(AudioEventStatsExtractor),
            "track_summary" => builder.extractor(TrackSummaryExtractor),
            "feature_vector_mean" => builder.extractor(FeatureVectorMeanExtractor),
            other => return Err(format!("unknown feature extractor `{other}`")),
        };
    }

    let features = builder
        .build()
        .extract(&request.dataset)
        .map_err(|error| error.to_string())?;
    Ok(serde_json::json!({
        "features": features,
        "featureCount": features.len(),
        "extractors": extractor_names
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use video_analysis_dataset::AnalysisDataset;

    #[test]
    fn default_extractors_return_stable_names() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("video.features.extract"),
            input: serde_json::json!({"dataset": AnalysisDataset::empty()}),
        })
        .expect("features");

        assert_eq!(
            response.value["extractors"],
            serde_json::json!(DEFAULT_EXTRACTORS)
        );
        assert!(response.value["featureCount"].as_u64().unwrap() > 0);
    }

    #[test]
    fn unknown_extractor_returns_typed_error() {
        let error = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("video.features.extract"),
            input: serde_json::json!({
                "dataset": AnalysisDataset::empty(),
                "extractors": ["missing"]
            }),
        })
        .expect_err("unknown extractor");
        assert!(error.contains("unknown feature extractor"));
    }

    #[test]
    fn package_surface_lists_feature_operation() {
        let ids = package_surface()
            .operations
            .into_iter()
            .map(|operation| operation.id.0)
            .collect::<Vec<_>>();
        assert!(ids.contains(&"video.features.extract".to_string()));
    }
}
