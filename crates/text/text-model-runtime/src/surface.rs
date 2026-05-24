//! Library-owned runtime surface for `text-model-runtime`.

use runtime_contracts::{
    OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation, SurfaceRequest,
    SurfaceResponse,
};
use serde::Deserialize;

use crate::{softmax, TokenizedText};

/// Returns the package surface exposed by every transport wrapper.
pub fn package_surface() -> PackageSurface {
    PackageSurface {
        library: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: RuntimeCapabilities::pure_rust(),
        operations: vec![
            operation("describe", "Describe package", "Shared tokenizer and native text model runtime traits for video-analysis.", serde_json::json!({"includeOperations": true})),
            operation("runtime.tokenizeSummary", "Tokenize summary", "Builds a deterministic TokenizedText-like whitespace-token summary without downloads or native runtime execution.", serde_json::json!({"text": "Rust text runtime", "maxTokens": 8})),
            operation("runtime.softmax", "Softmax", "Normalizes logits with a stable softmax.", serde_json::json!({"logits": [0.0, 1.0]})),
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
        "runtime.tokenizeSummary" => tokenize_summary_value(parse_input(request.input)?)?,
        "runtime.softmax" => softmax_value(parse_input(request.input)?)?,
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
struct TokenizeSummaryRequest {
    text: String,
    max_tokens: Option<usize>,
    #[serde(default = "default_truncation")]
    truncation: String,
}

#[derive(Debug, Deserialize)]
struct SoftmaxRequest {
    logits: Vec<f32>,
}

fn tokenize_summary_value(request: TokenizeSummaryRequest) -> Result<serde_json::Value, String> {
    let mut pieces = whitespace_tokens(&request.text);
    if let Some(max_tokens) = request.max_tokens {
        match request.truncation.as_str() {
            "none" if pieces.len() > max_tokens => {}
            "none" => {}
            "head" | "onlyFirst" | "only_first" => pieces.truncate(max_tokens),
            "tail" => {
                if pieces.len() > max_tokens {
                    pieces = pieces[pieces.len() - max_tokens..].to_vec();
                }
            }
            other => return Err(format!("unsupported truncation strategy `{other}`")),
        }
    }
    let tokenized = TokenizedText {
        input_ids: (0..pieces.len()).map(|index| index as i64 + 1).collect(),
        attention_mask: vec![1; pieces.len()],
        token_type_ids: None,
        offsets: pieces
            .iter()
            .map(|(_, start, end)| Some((*start, *end)))
            .collect(),
    };
    Ok(serde_json::json!({
        "backend": "deterministic-whitespace",
        "tokens": pieces.into_iter().map(|(token, start, end)| serde_json::json!({
            "text": token,
            "byteStart": start,
            "byteEnd": end
        })).collect::<Vec<_>>(),
        "tokenized": tokenized
    }))
}

fn softmax_value(request: SoftmaxRequest) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({ "probabilities": softmax(&request.logits) }))
}

fn whitespace_tokens(text: &str) -> Vec<(String, usize, usize)> {
    let mut tokens = Vec::new();
    let mut start = None;
    for (index, ch) in text.char_indices() {
        if ch.is_whitespace() {
            if let Some(byte_start) = start.take() {
                tokens.push((text[byte_start..index].to_string(), byte_start, index));
            }
        } else {
            start.get_or_insert(index);
        }
    }
    if let Some(byte_start) = start {
        tokens.push((text[byte_start..].to_string(), byte_start, text.len()));
    }
    tokens
}

fn parse_input<T: for<'de> Deserialize<'de>>(input: serde_json::Value) -> Result<T, String> {
    serde_json::from_value(input).map_err(|error| format!("invalid request: {error}"))
}

fn default_truncation() -> String {
    "head".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_surface_lists_runtime_operations() {
        let ids = package_surface()
            .operations
            .into_iter()
            .map(|operation| operation.id.0)
            .collect::<Vec<_>>();
        assert!(ids.contains(&"runtime.tokenizeSummary".to_string()));
        assert!(ids.contains(&"runtime.softmax".to_string()));
    }

    #[test]
    fn softmax_operation_normalizes_logits() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("runtime.softmax"),
            input: serde_json::json!({"logits": [0.0, 1.0]}),
        })
        .expect("softmax");
        assert_eq!(response.value["probabilities"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn malformed_input_returns_typed_error_string() {
        let error = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("runtime.softmax"),
            input: serde_json::json!({"logits": "bad"}),
        })
        .expect_err("invalid request");
        assert!(error.contains("invalid request"));
    }
}
