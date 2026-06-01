//! Library-owned runtime surface for `text-question-answering`.

use runtime_core::{
    structured_surface_value, OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation,
    SurfaceRequest, SurfaceResponse,
};

use crate::{answer_question, model_catalog};

/// Returns the package surface exposed by every transport wrapper.
pub fn package_surface() -> PackageSurface {
    PackageSurface {
        library: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: RuntimeCapabilities::pure_rust(),
        operations: vec![
            operation("describe", "Inspect package metadata", "Extractive question-answering contracts, imported span postprocessing, and backend execution hooks.", serde_json::json!({"includeOperations": true})),
            operation("qa.models", "Inspect QA model catalog", "Lists registered extractive question-answering presets.", serde_json::json!({})),
            operation("qa.answer", "Answer question", "Postprocesses imported span predictions for extractive QA.", serde_json::json!({"question": "What is reliable?", "context": "Rust is reliable.", "importedPredictions": [{"text": "Rust", "score": 0.9}]})),
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
        "qa.models" => serde_json::json!({ "models": model_catalog() }),
        "qa.answer" => serde_json::to_value(
            answer_question(
                serde_json::from_value(request.input)
                    .map_err(|error| format!("invalid request: {error}"))?,
            )
            .map_err(|error| error.to_string())?,
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
            "Inspected the text-question-answering package operations and runtime support.",
            serde_json::json!({
                "status": "ok",
                "operationCount": value["operationCount"]
            }),
        ),
        "qa.models" => (
            "QA model catalog",
            "Inspected registered extractive question-answering model presets without executing a model.",
            serde_json::json!({
                "status": "ok",
                "modelCount": value["models"].as_array().map(Vec::len).unwrap_or(0)
            }),
        ),
        "qa.answer" => (
            "Question answering result",
            "Postprocessed imported extractive QA span predictions for the supplied question and context.",
            serde_json::json!({
                "status": "ok",
                "answerCount": value["answers"].as_array().map(Vec::len).unwrap_or(0)
            }),
        ),
        _ => (
            "Question answering result",
            "Ran a text-question-answering package operation.",
            serde_json::json!({"status": "ok"}),
        ),
    };
    structured_surface_value(operation, title, message, summary, value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_surface_lists_qa_operations_only() {
        let ids = package_surface()
            .operations
            .into_iter()
            .map(|operation| operation.id.0)
            .collect::<Vec<_>>();
        assert!(ids.contains(&"qa.models".to_string()));
        assert!(ids.contains(&"qa.answer".to_string()));
        assert!(!ids.contains(&"nlp.sentiment".to_string()));
    }
}
