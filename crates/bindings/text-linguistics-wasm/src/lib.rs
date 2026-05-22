//! Browser-safe WASM bindings for text-linguistics payloads.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::{from_value, Serializer};
use text_core::{split_sentence_spans, tokenize, TextProcessingOptions, Token};
use wasm_bindgen::prelude::*;

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct RawLinguisticOptions {
    profile: Option<String>,
    entity_recognition: Option<String>,
    bert_ner_predictions: Option<Vec<RawPrediction>>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct RawPrediction {
    kind: Option<String>,
    label: Option<String>,
    score: Option<f32>,
    #[serde(default)]
    attributes: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntityType {
    Person,
    Organization,
    Location,
    Product,
    Date,
    Amount,
    Law,
    Work,
    Event,
    Misc,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WasmEntity {
    id: String,
    text: String,
    normalized: String,
    kind: String,
    sentence_index: usize,
    token_start: usize,
    token_end: usize,
    confidence: f32,
}

#[derive(Debug, Clone)]
struct ModelEntityPart {
    entity_type: EntityType,
    begins: bool,
    byte_start: usize,
    byte_end: usize,
    score: f32,
}

#[wasm_bindgen(js_name = analyzeTextLinguistics)]
/// Runs client-side linguistic analysis or merges browser BERT-NER predictions.
pub fn analyze_text_linguistics_binding(
    text: &str,
    options: Option<JsValue>,
) -> std::result::Result<JsValue, JsValue> {
    let raw_options = match options {
        Some(value) if !value.is_null() && !value.is_undefined() => {
            from_value(value).map_err(into_js_error)?
        }
        _ => RawLinguisticOptions::default(),
    };

    if matches!(
        raw_options.entity_recognition.as_deref(),
        Some("local-model" | "bert-base-ner" | "bert-ner")
    ) && raw_options.bert_ner_predictions.is_none()
    {
        return Err(JsValue::from_str(
            "client-side BERT-NER wasm requires `bertNerPredictions`; run the native server for bundled Candle inference",
        ));
    }

    to_js_value(&analysis_payload(text, &raw_options))
}

fn analysis_payload(text: &str, raw_options: &RawLinguisticOptions) -> serde_json::Value {
    let options = TextProcessingOptions {
        include_punctuation: true,
        ..TextProcessingOptions::default()
    };
    let tokens = tokenize(text, &options);
    let sentences = split_sentence_spans(text, &options);
    let entities = if let Some(predictions) = &raw_options.bert_ner_predictions {
        entities_from_predictions(text, &tokens, predictions)
    } else {
        heuristic_entities(text, &tokens)
    };
    let (entity_recognition, entity_model) = if raw_options.bert_ner_predictions.is_some() {
        ("client-wasm-predictions", Some("bert-base-ner"))
    } else {
        ("heuristic", None)
    };
    let language = if text.trim().is_empty() {
        None
    } else {
        Some("en")
    };

    serde_json::json!({
        "package": "text-linguistics-wasm",
        "library": "text-linguistics",
        "accepted": true,
        "operation": "analyze",
        "profile": profile_label(raw_options.profile.as_deref()),
        "provenance": if raw_options.bert_ner_predictions.is_some() { "External" } else { "Heuristic" },
        "confidence": if raw_options.bert_ner_predictions.is_some() { 0.85 } else { 0.65 },
        "model": {
            "entityRecognition": entity_recognition,
            "entityModel": entity_model,
            "tokenizerMode": "Word",
            "tokenizerSource": null,
            "alignmentCount": 0
        },
        "summary": {
            "language": language,
            "tokenCount": tokens.len(),
            "sentenceCount": sentences.len(),
            "lemmaCount": tokens.len(),
            "entityCount": entities.len(),
            "eventCount": 0,
            "relationCount": 0,
            "topicCount": 0,
            "chunkCount": 0
        },
        "language": {
            "primary": language.map(|language| serde_json::json!({
                "language": language,
                "confidence": 0.65,
                "script": null,
                "reason": "client wasm fallback"
            })),
            "dominantScript": null,
            "isMixed": false,
            "tokenCount": tokens.len()
        },
        "tokens": tokens.iter().enumerate().map(|(index, token)| serde_json::json!({
            "index": index,
            "text": token.text,
            "normalized": token.normalized,
            "kind": format!("{:?}", token.kind),
            "start": token.span.char_start,
            "end": token.span.char_end
        })).collect::<Vec<_>>(),
        "sentences": sentences.iter().enumerate().map(|(index, sentence)| serde_json::json!({
            "index": index,
            "text": sentence.text,
            "start": sentence.span.char_start,
            "end": sentence.span.char_end,
            "tokenCount": sentence.token_count
        })).collect::<Vec<_>>(),
        "lemmas": tokens.iter().enumerate().map(|(index, token)| serde_json::json!({
            "tokenIndex": index,
            "token": token.text,
            "lemma": token.normalized,
            "language": language,
            "confidence": 0.55
        })).collect::<Vec<_>>(),
        "pos": tokens.iter().enumerate().map(|(index, token)| serde_json::json!({
            "tokenIndex": index,
            "token": token.text,
            "tag": "X",
            "confidence": 0.5,
            "reason": "client wasm fallback"
        })).collect::<Vec<_>>(),
        "entities": entities,
        "events": [],
        "relations": [],
        "topics": [],
        "style": {
            "register": "Neutral",
            "averageSentenceTokens": if sentences.is_empty() { 0.0 } else { tokens.len() as f32 / sentences.len() as f32 },
            "typeTokenRatio": type_token_ratio(&tokens),
            "formalityScore": 0.5,
            "questionCount": text.matches('?').count(),
            "exclamationCount": text.matches('!').count()
        }
    })
}

fn entities_from_predictions(
    text: &str,
    tokens: &[Token],
    predictions: &[RawPrediction],
) -> Vec<WasmEntity> {
    let mut parts = predictions
        .iter()
        .filter_map(model_entity_part)
        .collect::<Vec<_>>();
    parts.sort_by_key(|part| (part.byte_start, part.byte_end));

    let mut entities = Vec::new();
    let mut current = Vec::<ModelEntityPart>::new();
    for part in parts {
        let starts_new = part.begins
            || current
                .last()
                .map(|last| last.entity_type != part.entity_type)
                .unwrap_or(false);
        if starts_new && !current.is_empty() {
            if let Some(entity) = build_model_entity(entities.len(), text, tokens, &current) {
                entities.push(entity);
            }
            current.clear();
        }
        current.push(part);
    }
    if !current.is_empty() {
        if let Some(entity) = build_model_entity(entities.len(), text, tokens, &current) {
            entities.push(entity);
        }
    }
    entities
}

fn model_entity_part(prediction: &RawPrediction) -> Option<ModelEntityPart> {
    if let Some(kind) = prediction.kind.as_deref() {
        if kind != "token" {
            return None;
        }
    }
    let label = prediction.label.as_deref()?;
    let (begins, entity_type) = parse_label(label)?;
    let byte_start = prediction.attributes.get("byte_start")?.parse().ok()?;
    let byte_end = prediction.attributes.get("byte_end")?.parse().ok()?;
    if byte_start >= byte_end {
        return None;
    }
    Some(ModelEntityPart {
        entity_type,
        begins,
        byte_start,
        byte_end,
        score: prediction.score.unwrap_or(0.5),
    })
}

fn parse_label(label: &str) -> Option<(bool, EntityType)> {
    let normalized = label
        .strip_prefix("LABEL_")
        .unwrap_or(label)
        .trim()
        .to_ascii_uppercase()
        .replace('_', "-");
    if normalized == "O" || normalized == "OUTSIDE" {
        return None;
    }
    let (begins, kind) = if let Some(kind) = normalized.strip_prefix("B-") {
        (true, kind)
    } else if let Some(kind) = normalized.strip_prefix("I-") {
        (false, kind)
    } else {
        (true, normalized.as_str())
    };
    let entity_type = match kind {
        "PER" | "PERSON" => EntityType::Person,
        "ORG" | "ORGANIZATION" => EntityType::Organization,
        "LOC" | "LOCATION" | "GPE" => EntityType::Location,
        "PROD" | "PRODUCT" => EntityType::Product,
        "DATE" | "TIME" => EntityType::Date,
        "MONEY" | "PERCENT" | "QUANTITY" | "AMOUNT" => EntityType::Amount,
        "LAW" => EntityType::Law,
        "WORK" | "WORK-OF-ART" => EntityType::Work,
        "EVENT" => EntityType::Event,
        "MISC" | "MISCELLANEOUS" => EntityType::Misc,
        _ => return None,
    };
    Some((begins, entity_type))
}

fn build_model_entity(
    index: usize,
    text: &str,
    tokens: &[Token],
    parts: &[ModelEntityPart],
) -> Option<WasmEntity> {
    let first = parts.first()?;
    let last = parts.last()?;
    if last.byte_end > text.len()
        || !text.is_char_boundary(first.byte_start)
        || !text.is_char_boundary(last.byte_end)
    {
        return None;
    }
    let mention = text[first.byte_start..last.byte_end].to_string();
    let (token_start, token_end) = token_range_for_bytes(tokens, first.byte_start, last.byte_end);
    Some(WasmEntity {
        id: format!("mention-model-{index}"),
        text: mention.clone(),
        normalized: mention.to_lowercase(),
        kind: format_entity_type(first.entity_type).to_string(),
        sentence_index: 0,
        token_start,
        token_end,
        confidence: parts.iter().map(|part| part.score).sum::<f32>() / parts.len() as f32,
    })
}

fn heuristic_entities(text: &str, tokens: &[Token]) -> Vec<WasmEntity> {
    tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| {
            token
                .text
                .chars()
                .next()
                .map(|ch| ch.is_uppercase())
                .unwrap_or(false)
        })
        .enumerate()
        .map(|(entity_index, (token_index, token))| WasmEntity {
            id: format!("mention-{entity_index}"),
            text: token.text.clone(),
            normalized: token.normalized.clone(),
            kind: "Misc".to_string(),
            sentence_index: 0,
            token_start: token_index,
            token_end: token_index + 1,
            confidence: 0.55,
        })
        .filter(|entity| !entity.text.trim().is_empty() && text.contains(&entity.text))
        .collect()
}

fn token_range_for_bytes(tokens: &[Token], byte_start: usize, byte_end: usize) -> (usize, usize) {
    let start = tokens
        .iter()
        .position(|token| token.span.byte_end > byte_start)
        .unwrap_or(0);
    let end = tokens
        .iter()
        .rposition(|token| token.span.byte_start < byte_end)
        .map(|index| index + 1)
        .unwrap_or(start + 1);
    (start, end)
}

fn type_token_ratio(tokens: &[Token]) -> f32 {
    if tokens.is_empty() {
        return 0.0;
    }
    let unique = tokens
        .iter()
        .map(|token| token.normalized.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    unique as f32 / tokens.len() as f32
}

fn profile_label(profile: Option<&str>) -> &'static str {
    match profile {
        Some("fast") => "Fast",
        Some("rich") => "Rich",
        _ => "Balanced",
    }
}

fn format_entity_type(entity_type: EntityType) -> &'static str {
    match entity_type {
        EntityType::Person => "Person",
        EntityType::Organization => "Organization",
        EntityType::Location => "Location",
        EntityType::Product => "Product",
        EntityType::Date => "Date",
        EntityType::Amount => "Amount",
        EntityType::Law => "Law",
        EntityType::Work => "Work",
        EntityType::Event => "Event",
        EntityType::Misc => "Misc",
    }
}

fn to_js_value(value: &serde_json::Value) -> std::result::Result<JsValue, JsValue> {
    let serializer = Serializer::json_compatible();
    value.serialize(&serializer).map_err(into_js_error)
}

fn into_js_error(error: serde_wasm_bindgen::Error) -> JsValue {
    JsValue::from_str(&error.to_string())
}
