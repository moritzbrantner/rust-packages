//! Browser-safe WASM bindings for text-linguistics payloads.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::{from_value, Serializer};
use text_core::{split_sentence_spans, tokenize, TextProcessingOptions, Token};
use text_lexical::{english_stop_words, extractive_summary, ExtractiveSummaryOptions};
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

#[wasm_bindgen(js_name = postprocessEntities)]
/// Converts imported BERT-NER token predictions into text-linguistics entities.
pub fn postprocess_entities_binding(
    text: &str,
    predictions: JsValue,
) -> std::result::Result<JsValue, JsValue> {
    let predictions = from_value::<Vec<RawPrediction>>(predictions).map_err(into_js_error)?;
    let options = TextProcessingOptions {
        include_punctuation: true,
        ..TextProcessingOptions::default()
    };
    let tokens = tokenize(text, &options);
    to_js_value(&serde_json::json!({
        "accepted": true,
        "operation": "entities",
        "runtime": "imported_predictions",
        "modelId": "bert-base-ner",
        "entities": entities_from_predictions(text, &tokens, &predictions)
    }))
}

#[wasm_bindgen(js_name = postprocessClassification)]
/// Normalizes imported classification predictions.
pub fn postprocess_classification_binding(
    text: &str,
    predictions: JsValue,
) -> std::result::Result<JsValue, JsValue> {
    let predictions = normalize_raw_predictions(from_value(predictions).map_err(into_js_error)?, 3);
    to_js_value(&serde_json::json!({
        "accepted": true,
        "operation": "classify",
        "text": text,
        "modelId": "imported-classifier",
        "runtime": "imported_predictions",
        "predictions": predictions
    }))
}

#[wasm_bindgen(js_name = postprocessSentiment)]
/// Normalizes imported sentiment predictions.
pub fn postprocess_sentiment_binding(
    text: &str,
    predictions: JsValue,
) -> std::result::Result<JsValue, JsValue> {
    let predictions = normalize_raw_predictions(from_value(predictions).map_err(into_js_error)?, 3);
    let label = predictions
        .first()
        .and_then(|prediction| prediction.get("label"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("neutral");
    let positive_score = score_for_label(&predictions, &["positive", "label_2"]);
    let negative_score = score_for_label(&predictions, &["negative", "label_0"]);
    to_js_value(&serde_json::json!({
        "accepted": true,
        "operation": "sentiment",
        "text": text,
        "modelId": "imported-sentiment",
        "runtime": "imported_predictions",
        "label": label,
        "positiveScore": positive_score,
        "negativeScore": negative_score,
        "compound": positive_score - negative_score,
        "predictions": predictions
    }))
}

#[wasm_bindgen(js_name = postprocessEmbeddings)]
/// Wraps imported embedding vectors in the shared response schema.
pub fn postprocess_embeddings_binding(
    embeddings: JsValue,
) -> std::result::Result<JsValue, JsValue> {
    let embeddings = from_value::<Vec<Vec<f32>>>(embeddings).map_err(into_js_error)?;
    let dimensions = embeddings.first().map(Vec::len).unwrap_or_default();
    to_js_value(&serde_json::json!({
        "accepted": true,
        "operation": "embed",
        "modelId": "imported-embeddings",
        "runtime": "imported_predictions",
        "dimensions": dimensions,
        "embeddings": embeddings
    }))
}

#[wasm_bindgen(js_name = postprocessZeroShot)]
/// Normalizes imported zero-shot predictions and hypotheses.
pub fn postprocess_zero_shot_binding(
    text: &str,
    labels: JsValue,
    predictions: JsValue,
) -> std::result::Result<JsValue, JsValue> {
    let labels = from_value::<Vec<String>>(labels).map_err(into_js_error)?;
    let predictions = normalize_raw_predictions(
        from_value(predictions).map_err(into_js_error)?,
        labels.len(),
    );
    let hypotheses = labels
        .iter()
        .map(|label| format!("This example is about {label}."))
        .collect::<Vec<_>>();
    to_js_value(&serde_json::json!({
        "accepted": true,
        "operation": "zero-shot",
        "text": text,
        "modelId": "imported-zero-shot",
        "runtime": "imported_predictions",
        "predictions": predictions,
        "hypotheses": hypotheses
    }))
}

#[wasm_bindgen(js_name = summarizeLexical)]
/// Runs browser-safe lexical extractive summarization.
pub fn summarize_lexical_binding(
    text: &str,
    max_sentences: usize,
) -> std::result::Result<JsValue, JsValue> {
    let response = lexical_summary_value(text, max_sentences, "lexical_extractive", None)?;
    to_js_value(&response)
}

#[wasm_bindgen(js_name = summarizeEmbeddingExtractiveFromImportedEmbeddings)]
/// Scores extractive summary sentences using imported sentence embeddings.
pub fn summarize_embedding_extractive_from_imported_embeddings_binding(
    text: &str,
    max_sentences: usize,
    sentence_embeddings: JsValue,
) -> std::result::Result<JsValue, JsValue> {
    let embeddings = from_value::<Vec<Vec<f32>>>(sentence_embeddings).map_err(into_js_error)?;
    let response = lexical_summary_value(
        text,
        max_sentences,
        "embedding_extractive",
        Some(embeddings),
    )?;
    to_js_value(&response)
}

#[wasm_bindgen(js_name = rerankFromImportedScores)]
/// Reranks documents using imported document scores.
pub fn rerank_from_imported_scores_binding(
    query: &str,
    documents: JsValue,
    scores: JsValue,
) -> std::result::Result<JsValue, JsValue> {
    let documents = from_value::<Vec<String>>(documents).map_err(into_js_error)?;
    let scores = from_value::<Vec<f32>>(scores).map_err(into_js_error)?;
    let mut results = documents
        .into_iter()
        .enumerate()
        .map(|(index, document)| {
            serde_json::json!({
                "index": index,
                "document": document,
                "score": scores.get(index).copied().unwrap_or(0.0)
            })
        })
        .collect::<Vec<_>>();
    results.sort_by(|left, right| {
        let left_score = left
            .get("score")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0);
        let right_score = right
            .get("score")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0);
        right_score.total_cmp(&left_score)
    });
    to_js_value(&serde_json::json!({
        "accepted": true,
        "operation": "rerank",
        "query": query,
        "modelId": "imported-reranker",
        "runtime": "imported_predictions",
        "results": results
    }))
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

fn normalize_raw_predictions(
    mut predictions: Vec<RawPrediction>,
    top_k: usize,
) -> Vec<serde_json::Value> {
    predictions.sort_by(|left, right| {
        right
            .score
            .unwrap_or(0.0)
            .total_cmp(&left.score.unwrap_or(0.0))
    });
    predictions
        .into_iter()
        .take(top_k.max(1))
        .map(|prediction| {
            serde_json::json!({
                "label": prediction.label.unwrap_or_else(|| "unknown".to_string()),
                "score": prediction.score.unwrap_or(0.0)
            })
        })
        .collect()
}

fn score_for_label(predictions: &[serde_json::Value], labels: &[&str]) -> f32 {
    predictions
        .iter()
        .find(|prediction| {
            let Some(label) = prediction.get("label").and_then(serde_json::Value::as_str) else {
                return false;
            };
            labels
                .iter()
                .any(|expected| label.eq_ignore_ascii_case(expected))
        })
        .and_then(|prediction| prediction.get("score"))
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0) as f32
}

fn lexical_summary_value(
    text: &str,
    max_sentences: usize,
    strategy: &str,
    embeddings: Option<Vec<Vec<f32>>>,
) -> std::result::Result<serde_json::Value, JsValue> {
    let options = ExtractiveSummaryOptions {
        max_sentences: max_sentences.max(1),
        min_sentence_words: 3,
        stop_words: english_stop_words(),
    };
    let mut sentences = extractive_summary(text, &options)
        .map_err(|error| JsValue::from_str(&error.to_string()))?
        .into_iter()
        .map(|sentence| {
            serde_json::json!({
                "index": sentence.index,
                "text": sentence.text,
                "span": {
                    "byteStart": sentence.span.byte_start,
                    "byteEnd": sentence.span.byte_end,
                    "charStart": sentence.span.char_start,
                    "charEnd": sentence.span.char_end
                },
                "score": sentence.score
            })
        })
        .collect::<Vec<_>>();

    if let Some(embeddings) = embeddings {
        let centroid = centroid(&embeddings);
        for (sentence, embedding) in sentences.iter_mut().zip(embeddings.iter()) {
            if let Some(score) = sentence.get_mut("score") {
                let current = score.as_f64().unwrap_or(0.0) as f32;
                *score = serde_json::json!(current + cosine(embedding, &centroid));
            }
        }
        sentences.sort_by(|left, right| {
            let left_score = left
                .get("score")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0);
            let right_score = right
                .get("score")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0);
            right_score.total_cmp(&left_score)
        });
        sentences.truncate(max_sentences.max(1));
        sentences.sort_by_key(|sentence| {
            sentence
                .get("index")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0)
        });
    }

    let summary = sentences
        .iter()
        .filter_map(|sentence| sentence.get("text").and_then(serde_json::Value::as_str))
        .collect::<Vec<_>>()
        .join(" ");

    Ok(serde_json::json!({
        "accepted": true,
        "operation": "summarize",
        "modelId": "embedding-extractive-summary",
        "runtime": "lexical",
        "strategy": strategy,
        "summary": summary,
        "sentences": sentences
    }))
}

fn centroid(vectors: &[Vec<f32>]) -> Vec<f32> {
    let dimensions = vectors.first().map(Vec::len).unwrap_or_default();
    let mut centroid = vec![0.0; dimensions];
    if dimensions == 0 {
        return centroid;
    }
    for vector in vectors {
        for (index, value) in vector.iter().take(dimensions).enumerate() {
            centroid[index] += *value;
        }
    }
    for value in &mut centroid {
        *value /= vectors.len().max(1) as f32;
    }
    centroid
}

fn cosine(left: &[f32], right: &[f32]) -> f32 {
    let mut dot = 0.0;
    let mut left_norm = 0.0;
    let mut right_norm = 0.0;
    for (left, right) in left.iter().zip(right.iter()) {
        dot += left * right;
        left_norm += left * left;
        right_norm += right * right;
    }
    if left_norm <= f32::EPSILON || right_norm <= f32::EPSILON {
        0.0
    } else {
        dot / (left_norm.sqrt() * right_norm.sqrt())
    }
}

fn to_js_value(value: &serde_json::Value) -> std::result::Result<JsValue, JsValue> {
    let serializer = Serializer::json_compatible();
    value.serialize(&serializer).map_err(into_js_error)
}

fn into_js_error(error: serde_wasm_bindgen::Error) -> JsValue {
    JsValue::from_str(&error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_predictions_are_ranked() {
        let predictions = normalize_raw_predictions(
            vec![
                RawPrediction {
                    label: Some("low".to_string()),
                    score: Some(0.1),
                    ..RawPrediction::default()
                },
                RawPrediction {
                    label: Some("high".to_string()),
                    score: Some(0.9),
                    ..RawPrediction::default()
                },
            ],
            1,
        );
        assert_eq!(predictions[0]["label"], "high");
    }
}
