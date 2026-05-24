//! Library-owned runtime surface for `text-nlp-models`.

use runtime_contracts::{
    OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation, SurfaceRequest,
    SurfaceResponse,
};
use serde::Deserialize;

use crate::{
    analyze_sentiment, answer_question, classify_text, embed_texts, model_catalog, parse_task,
    rerank, summarize, zero_shot_classify,
};

/// Returns the package surface exposed by every transport wrapper.
pub fn package_surface() -> PackageSurface {
    PackageSurface {
        library: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: RuntimeCapabilities::pure_rust(),
        operations: vec![
            operation("describe", "Describe package", "Shared text NLP task schemas, catalogs, imported prediction handling, and deterministic fallbacks.", serde_json::json!({"includeOperations": true})),
            operation("nlp.models", "List NLP models", "Lists registered NLP task presets.", serde_json::json!({})),
            operation("nlp.classify", "Classify text", "Runs imported-prediction or explicit fallback text classification.", serde_json::json!({"text": "rust is reliable", "labels": ["positive", "negative"], "model": {"fallbackPolicy": "lexical_fallback"}})),
            operation("nlp.sentiment", "Analyze sentiment", "Runs imported-prediction or explicit fallback sentiment analysis.", serde_json::json!({"text": "rust is reliable", "model": {"fallbackPolicy": "lexical_fallback"}})),
            operation("nlp.embed", "Embed text", "Runs imported embedding or explicit fallback text embedding.", serde_json::json!({"texts": ["rust text"], "dimensions": 16, "model": {"fallbackPolicy": "fast_fallback"}})),
            operation("nlp.zeroShot", "Zero-shot classify", "Runs imported-prediction or explicit fallback zero-shot classification.", serde_json::json!({"text": "rust text", "labels": ["code", "music"], "model": {"fallbackPolicy": "lexical_fallback"}})),
            operation("nlp.summarize", "Summarize text", "Runs deterministic extractive summarization.", serde_json::json!({"text": "Rust is reliable. Text crates expose APIs.", "maxSentences": 1})),
            operation("nlp.rerank", "Rerank documents", "Runs imported-score or explicit fallback query/document reranking.", serde_json::json!({"query": "rust", "documents": ["rust text", "video scenes"], "model": {"fallbackPolicy": "lexical_fallback"}})),
            operation("nlp.questionAnswer", "Answer question", "Postprocesses imported span predictions for extractive QA.", serde_json::json!({"question": "What is reliable?", "context": "Rust is reliable.", "importedPredictions": [{"label": "answer", "text": "Rust", "score": 0.9}]})),
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
        "nlp.models" => models_value(parse_input(request.input)?)?,
        "nlp.classify" => serde_json::to_value(
            classify_text(parse_input(request.input)?).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?,
        "nlp.sentiment" => serde_json::to_value(
            analyze_sentiment(parse_input(request.input)?).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?,
        "nlp.embed" => serde_json::to_value(
            embed_texts(parse_input(request.input)?).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?,
        "nlp.zeroShot" => serde_json::to_value(
            zero_shot_classify(parse_input(request.input)?).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?,
        "nlp.summarize" => serde_json::to_value(
            summarize(parse_input(request.input)?).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?,
        "nlp.rerank" => serde_json::to_value(
            rerank(parse_input(request.input)?).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?,
        "nlp.questionAnswer" => serde_json::to_value(
            answer_question(parse_input(request.input)?).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?,
        operation => {
            return Err(format!(
                "unsupported operation `{operation}` for {}",
                env!("CARGO_PKG_NAME")
            ))
        }
    };
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

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ModelsRequest {
    task: Option<String>,
}

fn models_value(request: ModelsRequest) -> Result<serde_json::Value, String> {
    let task = request
        .task
        .as_deref()
        .map(|task| parse_task(task).ok_or_else(|| format!("unknown NLP task `{task}`")))
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
    fn package_surface_lists_nlp_operations() {
        let ids = package_surface()
            .operations
            .into_iter()
            .map(|operation| operation.id.0)
            .collect::<Vec<_>>();
        assert!(ids.contains(&"nlp.models".to_string()));
        assert!(ids.contains(&"nlp.sentiment".to_string()));
    }

    #[test]
    fn models_operation_returns_catalog() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("nlp.models"),
            input: serde_json::json!({"task": "sentiment"}),
        })
        .expect("models");
        assert!(!response.value["models"].as_array().unwrap().is_empty());
    }

    #[test]
    fn malformed_input_returns_typed_error_string() {
        let error = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("nlp.models"),
            input: serde_json::json!({"task": "missing"}),
        })
        .expect_err("invalid request");
        assert!(error.contains("unknown NLP task"));
    }
}
