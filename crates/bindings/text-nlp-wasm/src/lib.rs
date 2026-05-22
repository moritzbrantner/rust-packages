//! Browser-safe WASM bindings for text-nlp-tasks payloads.

use serde::de::DeserializeOwned;
use serde::Serialize;
use wasm_bindgen::prelude::*;

use text_nlp_tasks::{
    analyze_sentiment, answer_question, classify_text, embed_texts, model_catalog, rerank,
    summarize, zero_shot_classify, EmbeddingRequest, QuestionAnsweringRequest, RerankRequest,
    SentimentRequest, SummaryRequest, TextClassificationRequest, ZeroShotClassificationRequest,
};

#[wasm_bindgen(js_name = textNlpPackageMetadata)]
pub fn package_metadata_binding() -> Result<JsValue, JsValue> {
    to_js_value(&package_metadata_value())
}

#[wasm_bindgen(js_name = textNlpModelCatalog)]
pub fn model_catalog_binding() -> Result<JsValue, JsValue> {
    to_js_value(&model_catalog(None))
}

#[wasm_bindgen(js_name = classifyText)]
pub fn classify_text_binding(request: JsValue) -> Result<JsValue, JsValue> {
    run_request::<TextClassificationRequest, _>(request, classify_text)
}

#[wasm_bindgen(js_name = analyzeSentiment)]
pub fn analyze_sentiment_binding(request: JsValue) -> Result<JsValue, JsValue> {
    run_request::<SentimentRequest, _>(request, analyze_sentiment)
}

#[wasm_bindgen(js_name = embedTexts)]
pub fn embed_texts_binding(request: JsValue) -> Result<JsValue, JsValue> {
    run_request::<EmbeddingRequest, _>(request, embed_texts)
}

#[wasm_bindgen(js_name = zeroShotClassify)]
pub fn zero_shot_classify_binding(request: JsValue) -> Result<JsValue, JsValue> {
    run_request::<ZeroShotClassificationRequest, _>(request, zero_shot_classify)
}

#[wasm_bindgen(js_name = summarizeText)]
pub fn summarize_binding(request: JsValue) -> Result<JsValue, JsValue> {
    run_request::<SummaryRequest, _>(request, summarize)
}

#[wasm_bindgen(js_name = rerankDocuments)]
pub fn rerank_binding(request: JsValue) -> Result<JsValue, JsValue> {
    run_request::<RerankRequest, _>(request, rerank)
}

#[wasm_bindgen(js_name = answerQuestion)]
pub fn answer_question_binding(request: JsValue) -> Result<JsValue, JsValue> {
    run_request::<QuestionAnsweringRequest, _>(request, answer_question)
}

pub fn package_metadata_json() -> String {
    package_metadata_value().to_string()
}

fn run_request<T, R>(
    request: JsValue,
    run: impl FnOnce(T) -> video_analysis_core::Result<R>,
) -> Result<JsValue, JsValue>
where
    T: DeserializeOwned,
    R: Serialize,
{
    let request = serde_wasm_bindgen::from_value::<T>(request).map_err(into_js_error)?;
    let response = run(request).map_err(|error| JsValue::from_str(&error.to_string()))?;
    to_js_value(&response)
}

fn package_metadata_value() -> serde_json::Value {
    serde_json::json!({
        "package": "text-nlp-wasm",
        "surface": "wasm",
        "library": "text-nlp-tasks",
        "libraryImport": "use text_nlp_tasks"
    })
}

fn to_js_value<T: Serialize>(value: &T) -> Result<JsValue, JsValue> {
    value
        .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(into_js_error)
}

fn into_js_error(error: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_reports_task_surface() {
        let metadata = package_metadata_json();
        assert!(metadata.contains("text-nlp-wasm"));
        assert!(metadata.contains("text-nlp-tasks"));
    }
}
