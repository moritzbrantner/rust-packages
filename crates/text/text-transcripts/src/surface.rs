//! Library-owned runtime surface for `text-transcripts`.

use runtime_contracts::{
    OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation, SurfaceRequest,
    SurfaceResponse,
};
use serde::Deserialize;

use crate::{
    format_srt, parse_plain_lines, parse_srt, parse_webvtt, parse_whisper_json,
    TranscriptionContract, TranscriptionResult,
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
                "Transcript parsing and ASR command adapters for video-analysis.",
                serde_json::json!({"includeOperations": true}),
            ),
            operation(
                "transcripts.parse",
                "Parse transcript",
                "Parses plain text, Whisper JSON, SRT, or WebVTT into the transcript contract.",
                serde_json::json!({"format": "srt", "content": "1\n00:00:01,000 --> 00:00:02,000\nHello.\n"}),
            ),
            operation(
                "transcripts.normalize",
                "Normalize transcript",
                "Normalizes transcript contract text, segments, words, and confidence.",
                serde_json::json!({"segments": [{"index": 0, "text": " hello ", "isFinal": true}]}),
            ),
            operation(
                "transcripts.formatSrt",
                "Format SRT",
                "Formats a transcript contract as SRT text.",
                serde_json::json!({"segments": [{"index": 0, "startSeconds": 1.0, "endSeconds": 2.0, "text": "Hello.", "isFinal": true}]}),
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
        "transcripts.parse" => parse_value(parse_input(request.input)?)?,
        "transcripts.normalize" => normalize_value(parse_input(request.input)?)?,
        "transcripts.formatSrt" => format_srt_value(parse_input(request.input)?)?,
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
struct ParseRequest {
    format: String,
    content: String,
}

fn parse_value(request: ParseRequest) -> Result<serde_json::Value, String> {
    let result = match request.format.as_str() {
        "plain" | "lines" => parse_plain_lines(&request.content),
        "whisperJson" | "whisper_json" | "whisper-json" => {
            parse_whisper_json(request.content.as_bytes()).map_err(|error| error.to_string())?
        }
        "srt" => parse_srt(&request.content).map_err(|error| error.to_string())?,
        "webVtt" | "webvtt" | "web-vtt" => {
            parse_webvtt(&request.content).map_err(|error| error.to_string())?
        }
        other => return Err(format!("unsupported transcript format `{other}`")),
    };
    let mut contract = TranscriptionContract::from(result)
        .normalized()
        .map_err(|error| error.to_string())?;
    let joined = contract.joined_text();
    if !joined.is_empty() {
        contract.text = Some(joined);
    }
    Ok(serde_json::json!(contract))
}

fn normalize_value(contract: TranscriptionContract) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!(contract
        .normalized()
        .map_err(|error| error.to_string())?))
}

fn format_srt_value(contract: TranscriptionContract) -> Result<serde_json::Value, String> {
    let normalized = contract.normalized().map_err(|error| error.to_string())?;
    let result = TranscriptionResult::from(normalized);
    Ok(serde_json::json!({ "srt": format_srt(&result.segments) }))
}

fn parse_input<T: for<'de> Deserialize<'de>>(input: serde_json::Value) -> Result<T, String> {
    serde_json::from_value(input).map_err(|error| format!("invalid request: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_surface_lists_transcript_operations() {
        let ids = package_surface()
            .operations
            .into_iter()
            .map(|operation| operation.id.0)
            .collect::<Vec<_>>();
        assert!(ids.contains(&"transcripts.parse".to_string()));
        assert!(ids.contains(&"transcripts.formatSrt".to_string()));
    }

    #[test]
    fn parse_operation_normalizes_plain_lines() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("transcripts.parse"),
            input: serde_json::json!({"format": "lines", "content": "hello\n\nworld\n"}),
        })
        .expect("parse");
        assert_eq!(response.value["text"], "hello world");
        assert_eq!(response.value["segments"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn malformed_input_returns_typed_error_string() {
        let error = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("transcripts.parse"),
            input: serde_json::json!({"format": "srt"}),
        })
        .expect_err("invalid request");
        assert!(error.contains("invalid request"));
    }
}
