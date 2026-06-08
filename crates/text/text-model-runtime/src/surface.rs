//! Library-owned runtime surface for `text-model-runtime`.

use runtime_core::{
    structured_surface_value, OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation,
    SurfaceRequest, SurfaceResponse,
};
use serde::Deserialize;

use std::path::PathBuf;

use crate::{
    softmax, validate_text_model_bundle, validate_tokenizer_bundle, TextModelBundleCheck,
    TextModelCapability, TokenizedText,
};

/// Returns the package surface exposed by every transport wrapper.
pub fn package_surface() -> PackageSurface {
    PackageSurface {
        library: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: RuntimeCapabilities::pure_rust(),
        operations: vec![
            operation("describe", "Inspect package metadata", "Shared tokenizer and native text model runtime traits for video-analysis.", serde_json::json!({"includeOperations": true})),
            operation("runtime.tokenizeSummary", "Tokenize summary", "Builds a deterministic TokenizedText-like whitespace-token summary without downloads or native runtime execution.", serde_json::json!({"text": "Rust text runtime", "maxTokens": 8})),
            operation("runtime.bundleCheck", "Check model bundle", "Validates required local model bundle files without downloads or native runtime execution.", serde_json::json!({"modelId": "demo-tokenizer", "capability": "tokenizer", "bundleRoot": ".model-runtime/demo-tokenizer", "requiredFiles": ["tokenizer.json"]})),
            operation("runtime.tokenizerProbe", "Probe tokenizer bundle", "Validates a local tokenizer file and performs a sample tokenization only when the tokenizers feature is available.", serde_json::json!({"modelId": "demo-tokenizer", "tokenizerPath": ".model-runtime/demo-tokenizer/tokenizer.json", "sample": "Rust text runtime"})),
            operation("runtime.softmax", "Normalize logits", "Normalizes logits with a stable softmax helper.", serde_json::json!({"logits": [0.0, 1.0]})),
        ],
    }
}

fn operation(
    id: &str,
    name: &str,
    description: &str,
    example_request: serde_json::Value,
) -> SurfaceOperation {
    runtime_core::surface_operation(id, name, description, example_request)
}

/// Runs one library-owned operation.
pub fn run_surface_operation(request: SurfaceRequest) -> Result<SurfaceResponse, String> {
    let operation = request.operation.clone();
    let value = match request.operation.as_str() {
        "describe" => describe_value(request.input),
        "runtime.tokenizeSummary" => tokenize_summary_value(parse_input(request.input)?)?,
        "runtime.bundleCheck" => bundle_check_value(parse_input(request.input)?)?,
        "runtime.tokenizerProbe" => tokenizer_probe_value(parse_input(request.input)?)?,
        "runtime.softmax" => softmax_value(parse_input(request.input)?)?,
        operation => {
            return Err(runtime_core::SurfaceError::unsupported_operation(
                operation,
                env!("CARGO_PKG_NAME"),
            )
            .to_error_string())
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
            "Inspected the text-model-runtime package operations and runtime support.",
            serde_json::json!({
                "status": "ok",
                "operationCount": value["operationCount"]
            }),
        ),
        "runtime.tokenizeSummary" => (
            "Tokenizer summary result",
            "Built a deterministic whitespace-token summary without downloads or native model execution.",
            serde_json::json!({
                "status": "ok",
                "backend": value["backend"],
                "tokenCount": value["tokens"].as_array().map(Vec::len).unwrap_or(0)
            }),
        ),
        "runtime.bundleCheck" => (
            "Model bundle readiness result",
            "Validated required local model bundle files without downloads or native runtime execution.",
            serde_json::json!({
                "status": "ok",
                "modelId": value["modelId"],
                "loadable": value["loadable"],
                "missingFileCount": value["missingFiles"].as_array().map(Vec::len).unwrap_or(0)
            }),
        ),
        "runtime.tokenizerProbe" => (
            "Tokenizer probe result",
            "Validated a local tokenizer bundle and reported whether a sample tokenization could run.",
            serde_json::json!({
                "status": "ok",
                "modelId": value["report"]["modelId"],
                "loadable": value["report"]["loadable"],
                "ran": value["run"].is_object()
            }),
        ),
        "runtime.softmax" => (
            "Softmax support result",
            "Normalized logits with the shared stable softmax helper.",
            serde_json::json!({
                "status": "ok",
                "probabilityCount": value["probabilities"].as_array().map(Vec::len).unwrap_or(0)
            }),
        ),
        _ => (
            "Text model runtime result",
            "Ran a text-model-runtime package operation.",
            serde_json::json!({"status": "ok"}),
        ),
    };
    structured_surface_value(operation, title, message, summary, value)
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BundleCheckRequest {
    model_id: String,
    capability: TextModelCapability,
    bundle_root: PathBuf,
    required_files: Vec<String>,
    required_feature: Option<String>,
    required_setup: Option<String>,
    smoke_operation: Option<String>,
    #[serde(default = "default_supported")]
    supported: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenizerProbeRequest {
    model_id: String,
    tokenizer_path: PathBuf,
    sample: String,
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
    runtime_core::require_non_empty("runtime.softmax", "logits", &request.logits)?;
    Ok(serde_json::json!({ "probabilities": softmax(&request.logits) }))
}

fn bundle_check_value(request: BundleCheckRequest) -> Result<serde_json::Value, String> {
    require_non_empty_string("runtime.bundleCheck", "modelId", &request.model_id)?;
    require_non_empty_path("runtime.bundleCheck", "bundleRoot", &request.bundle_root)?;
    runtime_core::require_non_empty(
        "runtime.bundleCheck",
        "requiredFiles",
        &request.required_files,
    )?;
    for file in &request.required_files {
        require_non_empty_string("runtime.bundleCheck", "requiredFiles[]", file)?;
    }

    let mut check = TextModelBundleCheck::new(
        request.model_id,
        request.capability,
        request.bundle_root,
        request.required_files,
    );
    if let Some(feature) = request.required_feature {
        check = check.required_feature(feature);
    }
    if let Some(setup) = request.required_setup {
        check = check.required_setup(setup);
    }
    if let Some(operation) = request.smoke_operation {
        check = check.smoke_operation(operation);
    }
    if !request.supported {
        check = check.reference_only();
    }

    serde_json::to_value(validate_text_model_bundle(check)).map_err(|error| error.to_string())
}

fn tokenizer_probe_value(request: TokenizerProbeRequest) -> Result<serde_json::Value, String> {
    require_non_empty_string("runtime.tokenizerProbe", "modelId", &request.model_id)?;
    require_non_empty_path(
        "runtime.tokenizerProbe",
        "tokenizerPath",
        &request.tokenizer_path,
    )?;
    require_non_empty_string("runtime.tokenizerProbe", "sample", &request.sample)?;

    let (report, run) =
        validate_tokenizer_bundle(request.model_id, request.tokenizer_path, &request.sample);
    Ok(serde_json::json!({
        "report": report,
        "run": run
    }))
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
    runtime_core::parse_surface_input(None, input)
}

fn default_truncation() -> String {
    "head".to_string()
}

fn default_supported() -> bool {
    true
}

fn require_non_empty_string(operation: &str, field: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!(
            "invalid request for `{operation}`: `{field}` must not be empty"
        ));
    }
    Ok(())
}

fn require_non_empty_path(
    operation: &str,
    field: &str,
    value: &std::path::Path,
) -> Result<(), String> {
    if value.as_os_str().is_empty() {
        return Err(format!(
            "invalid request for `{operation}`: `{field}` must not be empty"
        ));
    }
    Ok(())
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
        assert!(ids.contains(&"runtime.bundleCheck".to_string()));
        assert!(ids.contains(&"runtime.tokenizerProbe".to_string()));
        assert!(ids.contains(&"runtime.softmax".to_string()));
    }

    #[test]
    fn bundle_check_reports_missing_files_without_downloads() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("runtime.bundleCheck"),
            input: serde_json::json!({
                "modelId": "missing-tokenizer",
                "capability": "tokenizer",
                "bundleRoot": ".model-runtime/missing-tokenizer",
                "requiredFiles": ["tokenizer.json"]
            }),
        })
        .expect("bundle check");
        assert_eq!(response.value["loadable"], false);
        assert_eq!(response.value["missingFiles"].as_array().unwrap().len(), 1);
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
