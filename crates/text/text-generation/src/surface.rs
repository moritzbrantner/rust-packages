//! Library-owned runtime surface for `text-generation`.

use runtime_contracts::{
    OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation, SurfaceRequest,
    SurfaceResponse,
};
use serde::Deserialize;

use crate::{synthesize_from_terms, MarkovChain, TermPrompt, TextSynthesisOptions};

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
                "Deterministic Markov-chain prediction and text synthesis for video-analysis.",
                serde_json::json!({"includeOperations": true}),
            ),
            operation(
                "generation.markovPredict",
                "Predict next token",
                "Trains a transient Markov chain and predicts next tokens.",
                serde_json::json!({"trainingTexts": ["rust text analysis rust text"], "context": ["rust", "text"], "order": 2}),
            ),
            operation(
                "generation.markovGenerate",
                "Generate Markov text",
                "Trains a transient Markov chain and deterministically generates text.",
                serde_json::json!({"trainingTexts": ["rust text analysis supports crates"], "order": 2, "maxTokens": 6}),
            ),
            operation(
                "generation.synthesizeTerms",
                "Synthesize from terms",
                "Synthesizes deterministic text from weighted term prompts.",
                serde_json::json!({"terms": [{"term": "rust", "weight": 2.0}, {"term": "analysis", "weight": 1.0}]}),
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
        "generation.markovPredict" => predict_value(parse_input(request.input)?)?,
        "generation.markovGenerate" => generate_value(parse_input(request.input)?)?,
        "generation.synthesizeTerms" => synthesize_terms_value(parse_input(request.input)?)?,
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MarkovPredictRequest {
    training_texts: Vec<String>,
    context: Vec<String>,
    #[serde(default = "default_order")]
    order: usize,
    #[serde(default = "default_top_k")]
    top_k: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MarkovGenerateRequest {
    training_texts: Vec<String>,
    #[serde(default)]
    seed: Vec<String>,
    #[serde(default = "default_order")]
    order: usize,
    #[serde(default = "default_max_tokens")]
    max_tokens: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SynthesizeTermsRequest {
    id: Option<String>,
    terms: Vec<TermPrompt>,
    #[serde(default)]
    options: TextSynthesisOptions,
}

fn predict_value(request: MarkovPredictRequest) -> Result<serde_json::Value, String> {
    let chain = trained_chain(request.order, &request.training_texts)?;
    let context = request.context.iter().map(String::as_str);
    Ok(serde_json::json!({
        "order": chain.order(),
        "contexts": chain.contexts(),
        "transitions": chain.total_transitions(),
        "predictions": chain.predict_next(context, request.top_k).map_err(|error| error.to_string())?
    }))
}

fn generate_value(request: MarkovGenerateRequest) -> Result<serde_json::Value, String> {
    let chain = trained_chain(request.order, &request.training_texts)?;
    let seed = request.seed.iter().map(String::as_str).collect::<Vec<_>>();
    Ok(serde_json::json!({
        "order": chain.order(),
        "contexts": chain.contexts(),
        "transitions": chain.total_transitions(),
        "generation": chain.generate(&seed, request.max_tokens).map_err(|error| error.to_string())?
    }))
}

fn synthesize_terms_value(request: SynthesizeTermsRequest) -> Result<serde_json::Value, String> {
    let generated = synthesize_from_terms(
        request.id.unwrap_or_else(|| "generated-text".to_string()),
        &request.terms,
        request.options,
    )
    .map_err(|error| error.to_string())?;
    Ok(serde_json::json!({
        "value": {
            "id": generated.value.id,
            "text": generated.value.text,
            "language": generated.value.language
        },
        "trace": {
            "sourceType": generated.trace.source_type,
            "targetType": generated.trace.target_type,
            "fidelity": format!("{:?}", generated.trace.fidelity),
            "confidence": generated.trace.confidence,
            "assumptions": generated.trace.assumptions,
            "notes": generated.trace.notes.into_iter().map(|note| serde_json::json!({
                "field": note.field,
                "method": format!("{:?}", note.method),
                "message": note.message
            })).collect::<Vec<_>>()
        }
    }))
}

fn trained_chain(order: usize, training_texts: &[String]) -> Result<MarkovChain, String> {
    if training_texts.is_empty() {
        return Err("invalid request: trainingTexts must not be empty".to_string());
    }
    let mut chain = MarkovChain::new(order).map_err(|error| error.to_string())?;
    chain.train_documents(training_texts);
    Ok(chain)
}

fn parse_input<T: for<'de> Deserialize<'de>>(input: serde_json::Value) -> Result<T, String> {
    serde_json::from_value(input).map_err(|error| format!("invalid request: {error}"))
}

fn default_order() -> usize {
    2
}
fn default_top_k() -> usize {
    5
}
fn default_max_tokens() -> usize {
    32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_surface_lists_generation_operations() {
        let ids = package_surface()
            .operations
            .into_iter()
            .map(|operation| operation.id.0)
            .collect::<Vec<_>>();
        assert!(ids.contains(&"generation.markovPredict".to_string()));
        assert!(ids.contains(&"generation.synthesizeTerms".to_string()));
    }

    #[test]
    fn markov_predict_operation_returns_predictions() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("generation.markovPredict"),
            input: serde_json::json!({
                "trainingTexts": ["rust text analysis rust text crates"],
                "context": ["rust", "text"],
                "order": 2
            }),
        })
        .expect("predict");
        assert_eq!(response.value["predictions"][0]["token"], "analysis");
    }

    #[test]
    fn malformed_input_returns_typed_error_string() {
        let error = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("generation.markovPredict"),
            input: serde_json::json!({"trainingTexts": [], "context": ["rust"]}),
        })
        .expect_err("invalid request");
        assert!(error.contains("trainingTexts"));
    }
}
