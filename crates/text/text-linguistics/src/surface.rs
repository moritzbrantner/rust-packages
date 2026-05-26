//! Library-owned runtime surface for `text-linguistics`.

use serde::Deserialize;
use video_analysis_core::runtime::{
    OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation, SurfaceRequest,
    SurfaceResponse,
};

use crate::{LinguisticAnalysis, TextNlpConfig, TextNlpPipeline};

/// Returns the package surface exposed by every transport wrapper.
pub fn package_surface() -> PackageSurface {
    PackageSurface {
        library: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: RuntimeCapabilities::pure_rust(),
        operations: vec![
            operation("describe", "Describe package", "Local model-backed linguistic analysis pipeline for video-analysis.", serde_json::json!({"includeOperations": true})),
            operation("linguistics.analyze", "Analyze text", "Runs the deterministic linguistic pipeline and returns a serializable analysis projection.", serde_json::json!({"text": "Alice presented the tokenizer roadmap in Berlin.", "profile": "fast"})),
            operation("linguistics.entities", "Extract linguistic entities", "Returns entities, canonical entities, relations, and events from the pipeline.", serde_json::json!({"text": "Alice presented the tokenizer roadmap in Berlin."})),
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
        "linguistics.analyze" => analyze_value(parse_input(request.input)?)?,
        "linguistics.entities" => entities_value(parse_input(request.input)?)?,
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
struct AnalyzeRequest {
    text: String,
    #[serde(default = "default_profile")]
    profile: String,
    language_hint: Option<String>,
}

fn analyze_value(request: AnalyzeRequest) -> Result<serde_json::Value, String> {
    let analysis = analyze_request(&request)?;
    Ok(analysis_projection(analysis))
}

fn entities_value(request: AnalyzeRequest) -> Result<serde_json::Value, String> {
    let analysis = analyze_request(&request)?;
    Ok(serde_json::json!({
        "language": language_value(&analysis),
        "entities": entity_values(&analysis),
        "canonicalEntities": analysis.canonical_entities.into_iter().map(|entity| serde_json::json!({
            "id": entity.id,
            "type": format!("{:?}", entity.entity_type),
            "canonicalName": entity.canonical_name,
            "aliases": entity.aliases
        })).collect::<Vec<_>>(),
        "relations": analysis.relations.into_iter().map(|relation| serde_json::json!({
            "relation": format!("{:?}", relation.relation),
            "subject": relation.subject,
            "object": relation.object,
            "confidence": relation.confidence
        })).collect::<Vec<_>>(),
        "events": analysis.events.into_iter().map(|event| serde_json::json!({
            "predicate": event.predicate,
            "lemma": event.lemma,
            "relationType": format!("{:?}", event.relation_type),
            "confidence": event.confidence,
            "arguments": event.arguments.into_iter().map(|argument| serde_json::json!({
                "role": argument.role,
                "text": argument.text,
                "confidence": argument.confidence
            })).collect::<Vec<_>>()
        })).collect::<Vec<_>>()
    }))
}

fn analyze_request(request: &AnalyzeRequest) -> Result<LinguisticAnalysis, String> {
    let mut config = match request.profile.as_str() {
        "fast" => TextNlpConfig::fast(),
        "standard" | "balanced" => TextNlpConfig::balanced(),
        "full" | "rich" => TextNlpConfig::rich(),
        other => return Err(format!("unsupported linguistic profile `{other}`")),
    };
    if let Some(language) = &request.language_hint {
        config.options.processing.language = Some(language.clone());
        config.options.language_detection.sentence_level = false;
    }
    config.options.entity_recognition = crate::EntityRecognitionOptions::heuristic();
    TextNlpPipeline::new(config)
        .analyze_text(&request.text)
        .map_err(|error| error.to_string())
}

fn analysis_projection(analysis: LinguisticAnalysis) -> serde_json::Value {
    serde_json::json!({
        "profile": format!("{:?}", analysis.profile),
        "provenance": format!("{:?}", analysis.provenance),
        "confidence": analysis.confidence.get(),
        "language": language_value(&analysis),
        "tokenizer": {
            "mode": format!("{:?}", analysis.tokenizer.mode),
            "source": analysis.tokenizer.source.as_ref().map(|source| format!("{source:?}")),
            "reason": analysis.tokenizer.reason
        },
        "sentences": analysis.sentences,
        "tokens": analysis.tokens,
        "lemmas": analysis.lemmas.iter().map(|lemma| serde_json::json!({
            "tokenIndex": lemma.token_index,
            "value": lemma.value,
            "language": lemma.language,
            "confidence": lemma.confidence
        })).collect::<Vec<_>>(),
        "pos": analysis.pos.iter().map(|pos| serde_json::json!({
            "tokenIndex": pos.token_index,
            "tag": format!("{:?}", pos.tag),
            "confidence": pos.confidence,
            "reason": pos.reason
        })).collect::<Vec<_>>(),
        "chunks": analysis.chunks.iter().map(|chunk| serde_json::json!({
            "kind": format!("{:?}", chunk.kind),
            "text": chunk.text,
            "sentenceIndex": chunk.sentence_index,
            "tokenStart": chunk.token_start,
            "tokenEnd": chunk.token_end,
            "span": chunk.span
        })).collect::<Vec<_>>(),
        "entities": entity_values(&analysis),
        "topics": {
            "descriptors": analysis.topics.descriptors.iter().map(|descriptor| serde_json::json!({
                "label": descriptor.label,
                "terms": descriptor.terms,
                "score": descriptor.score
            })).collect::<Vec<_>>()
        },
        "style": {
            "register": format!("{:?}", analysis.style.register),
            "questionCount": analysis.style.question_count,
            "exclamationCount": analysis.style.exclamation_count,
            "formalityScore": analysis.style.formality_score
        }
    })
}

fn language_value(analysis: &LinguisticAnalysis) -> serde_json::Value {
    serde_json::json!({
        "primary": analysis.language.primary.as_ref().map(|prediction| serde_json::json!({
            "language": prediction.language,
            "confidence": prediction.confidence,
            "script": prediction.script,
            "reason": prediction.reason
        })),
        "dominantScript": analysis.language.dominant_script,
        "isMixed": analysis.language.is_mixed,
        "tokenCount": analysis.language.token_count
    })
}

fn entity_values(analysis: &LinguisticAnalysis) -> Vec<serde_json::Value> {
    analysis
        .entities
        .iter()
        .map(|entity| {
            serde_json::json!({
                "id": entity.id,
                "type": format!("{:?}", entity.entity_type),
                "text": entity.mention.text,
                "span": entity.mention.span,
                "normalized": entity.normalized,
                "sentenceIndex": entity.sentence_index,
                "tokenStart": entity.token_start,
                "tokenEnd": entity.token_end,
                "confidence": entity.confidence
            })
        })
        .collect()
}

fn parse_input<T: for<'de> Deserialize<'de>>(input: serde_json::Value) -> Result<T, String> {
    serde_json::from_value(input).map_err(|error| format!("invalid request: {error}"))
}

fn default_profile() -> String {
    "standard".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_surface_lists_linguistic_operations() {
        let ids = package_surface()
            .operations
            .into_iter()
            .map(|operation| operation.id.0)
            .collect::<Vec<_>>();
        assert!(ids.contains(&"linguistics.analyze".to_string()));
        assert!(ids.contains(&"linguistics.entities".to_string()));
    }

    #[test]
    fn analyze_operation_returns_language_and_entities() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("linguistics.analyze"),
            input: serde_json::json!({"text": "Alice presented the tokenizer roadmap in Berlin.", "profile": "fast"}),
        }).expect("analyze");
        assert_eq!(response.value["language"]["primary"]["language"], "en");
        assert!(!response.value["tokens"].as_array().unwrap().is_empty());
    }

    #[test]
    fn malformed_input_returns_typed_error_string() {
        let error = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("linguistics.analyze"),
            input: serde_json::json!({"text": "hello", "profile": "missing"}),
        })
        .expect_err("invalid profile");
        assert!(error.contains("unsupported linguistic profile"));
    }
}
