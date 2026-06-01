//! Library-owned runtime surface for `text-classification`.

use runtime_core::{
    structured_surface_value, OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation,
    SurfaceRequest, SurfaceResponse,
};
use serde::Deserialize;

use crate::{analyze_sentiment, classify_text, model_catalog, parse_task, zero_shot_classify};

/// Returns the package surface exposed by every transport wrapper.
pub fn package_surface() -> PackageSurface {
    PackageSurface {
        library: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: RuntimeCapabilities::pure_rust(),
        operations: vec![
            operation("describe", "Inspect package metadata", "Text classification, sentiment, and zero-shot classification contracts with imported prediction handling and deterministic fallbacks.", serde_json::json!({"includeOperations": true})),
            operation("classification.models", "Inspect classification model catalog", "Lists registered text classification presets.", serde_json::json!({})),
            operation("classification.classify", "Classify text", "Runs imported-prediction or explicit fallback text classification.", serde_json::json!({"text": "rust is reliable", "labels": ["positive", "negative"], "model": {"fallbackPolicy": "lexical_fallback"}})),
            operation("classification.sentiment", "Analyze sentiment", "Runs imported-prediction or explicit fallback sentiment analysis.", serde_json::json!({"text": "rust is reliable", "model": {"fallbackPolicy": "lexical_fallback"}})),
            operation("classification.zeroShot", "Zero-shot classify", "Runs imported-prediction or explicit fallback zero-shot classification.", serde_json::json!({"text": "rust text", "labels": ["code", "music"], "model": {"fallbackPolicy": "lexical_fallback"}})),
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
        "classification.models" => models_value(parse_input(request.input)?)?,
        "classification.classify" => serde_json::to_value(
            classify_text(parse_input(request.input)?).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?,
        "classification.sentiment" => serde_json::to_value(
            analyze_sentiment(parse_input(request.input)?).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?,
        "classification.zeroShot" => serde_json::to_value(
            zero_shot_classify(parse_input(request.input)?).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?,
        operation => {
            return Err(format!(
                "unsupported operation `{operation}` for {}",
                env!("CARGO_PKG_NAME")
            ))
        }
    };
    let value = annotated_value(&operation, value);
    Ok(SurfaceResponse {
        operation,
        value,
        diagnostics: Vec::new(),
        artifacts: Vec::new(),
    })
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

fn annotated_value(operation: &OperationId, value: serde_json::Value) -> serde_json::Value {
    let (title, message, summary) = match operation.as_str() {
        "describe" => (
            "Package surface metadata",
            "Inspected the text-classification package operations and runtime support.",
            serde_json::json!({
                "status": "ok",
                "operationCount": value["operationCount"]
            }),
        ),
        "classification.models" => (
            "Classification model catalog",
            "Inspected registered classification presets without executing a model.",
            serde_json::json!({
                "status": "ok",
                "modelCount": value["models"].as_array().map(Vec::len).unwrap_or(0)
            }),
        ),
        "classification.classify" => (
            "Text classification result",
            "Ran imported-prediction or explicit lexical fallback text classification.",
            serde_json::json!({
                "status": "ok",
                "predictionCount": value["predictions"].as_array().map(Vec::len).unwrap_or(0),
                "runtime": value["runtime"]
            }),
        ),
        "classification.sentiment" => (
            "Sentiment analysis result",
            "Ran imported-prediction or explicit lexical fallback sentiment analysis.",
            serde_json::json!({
                "status": "ok",
                "label": value["label"],
                "runtime": value["runtime"]
            }),
        ),
        "classification.zeroShot" => (
            "Zero-shot classification result",
            "Ran imported-prediction or explicit lexical fallback zero-shot classification.",
            serde_json::json!({
                "status": "ok",
                "predictionCount": value["predictions"].as_array().map(Vec::len).unwrap_or(0),
                "runtime": value["runtime"]
            }),
        ),
        _ => (
            "Text classification result",
            "Ran a text-classification package operation.",
            serde_json::json!({"status": "ok"}),
        ),
    };
    structured_surface_value(operation, title, message, summary, value)
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ModelsRequest {
    task: Option<String>,
}

fn models_value(request: ModelsRequest) -> Result<serde_json::Value, String> {
    let task = request
        .task
        .as_deref()
        .map(|task| {
            parse_task(task).ok_or_else(|| format!("unknown text classification task `{task}`"))
        })
        .transpose()?;
    Ok(serde_json::json!({ "models": model_catalog(task) }))
}

fn parse_input<T: for<'de> Deserialize<'de>>(input: serde_json::Value) -> Result<T, String> {
    serde_json::from_value(input).map_err(|error| format!("invalid request: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_surface_lists_classification_operations() {
        let ids = package_surface()
            .operations
            .into_iter()
            .map(|operation| operation.id.0)
            .collect::<Vec<_>>();
        assert!(ids.contains(&"classification.models".to_string()));
        assert!(ids.contains(&"classification.sentiment".to_string()));
        assert!(!ids.contains(&"embeddings.embed".to_string()));
    }

    #[test]
    fn models_operation_returns_catalog() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("classification.models"),
            input: serde_json::json!({"task": "sentiment"}),
        })
        .expect("models");
        assert!(!response.value["models"].as_array().unwrap().is_empty());
    }

    #[test]
    fn malformed_input_returns_typed_error_string() {
        let error = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("classification.models"),
            input: serde_json::json!({"task": "missing"}),
        })
        .expect_err("invalid request");
        assert!(error.contains("unknown text classification task"));
    }
}
