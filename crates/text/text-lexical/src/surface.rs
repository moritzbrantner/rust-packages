//! Library-owned runtime surface for `text-lexical`.

use runtime_core::{
    structured_surface_value, OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation,
    SurfaceRequest, SurfaceResponse,
};
use serde::Deserialize;
use text_core::TextDocument;

use crate::{
    diverse_extractive_summary, english_stop_words, extractive_summary, keywords, phrase_keywords,
    readability_summary, rule_entities, sentiment, stop_words_for_language, Bm25Corpus,
    Bm25Options, EntityRuleSet, ExtractiveSummaryOptions, KeywordOptions, PhraseKeywordOptions,
    SentimentLexicon, TfIdfCorpus,
};

/// Returns the package surface exposed by every transport wrapper.
pub fn package_surface() -> PackageSurface {
    PackageSurface {
        library: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: RuntimeCapabilities::pure_rust(),
        operations: vec![
            operation("describe", "Inspect package metadata", "Lexical text features, corpus statistics, TF-IDF, and BM25 for video-analysis.", serde_json::json!({"includeOperations": true})),
            operation("lexical.analyze", "Analyze lexical features", "Returns deterministic keywords, summaries, readability, sentiment, and rule entities.", serde_json::json!({"text": "Rust crates make text analysis reliable.", "maxTerms": 5})),
            operation("lexical.keywords", "Extract keywords", "Ranks deterministic lexical keywords.", serde_json::json!({"text": "Rust text crates analyze text text.", "limit": 3})),
            operation("lexical.corpusSearch", "Search corpus", "Searches a transient TF-IDF or BM25 corpus.", serde_json::json!({"documents": [{"id": "doc-1", "text": "rust text analysis"}, {"id": "doc-2", "text": "video scene analysis"}], "query": "text", "mode": "bm25"})),
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
        "lexical.analyze" => lexical_analyze(parse_input(request.input)?)?,
        "lexical.keywords" => lexical_keywords(parse_input(request.input)?)?,
        "lexical.corpusSearch" => corpus_search(parse_input(request.input)?)?,
        operation => {
            return Err(format!(
                "unsupported operation `{operation}` for {}",
                env!("CARGO_PKG_NAME")
            ));
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
            "Inspected the text-lexical package operations and runtime support.",
            serde_json::json!({
                "status": "ok",
                "operationCount": value["operationCount"]
            }),
        ),
        "lexical.analyze" => (
            "Lexical analysis result",
            "Computed deterministic keywords, summaries, readability, sentiment, and rule entities.",
            serde_json::json!({
                "status": "ok",
                "keywordCount": value["keywords"].as_array().map(Vec::len).unwrap_or(0),
                "phraseKeywordCount": value["phraseKeywords"].as_array().map(Vec::len).unwrap_or(0),
                "entityCount": value["ruleEntities"].as_array().map(Vec::len).unwrap_or(0)
            }),
        ),
        "lexical.keywords" => (
            "Keyword ranking result",
            "Ranked deterministic lexical keywords for the supplied text.",
            serde_json::json!({
                "status": "ok",
                "keywordCount": value["keywords"].as_array().map(Vec::len).unwrap_or(0)
            }),
        ),
        "lexical.corpusSearch" => (
            "Lexical corpus search result",
            "Built a transient lexical corpus and searched it without writing persistence artifacts.",
            serde_json::json!({
                "status": "ok",
                "mode": value["mode"],
                "resultCount": value["results"].as_array().map(Vec::len).unwrap_or(0)
            }),
        ),
        _ => (
            "Text lexical result",
            "Ran a text-lexical package operation.",
            serde_json::json!({"status": "ok"}),
        ),
    };
    structured_surface_value(operation, title, message, summary, value)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LexicalAnalyzeRequest {
    text: String,
    language: Option<String>,
    #[serde(default = "default_terms")]
    max_terms: usize,
    #[serde(default = "default_summary_sentences")]
    max_summary_sentences: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LexicalKeywordsRequest {
    text: String,
    language: Option<String>,
    #[serde(default = "default_terms")]
    limit: usize,
    #[serde(default)]
    min_score: f32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CorpusSearchRequest {
    documents: Vec<CorpusDocument>,
    query: String,
    #[serde(default = "default_corpus_mode")]
    mode: String,
    #[serde(default = "default_top_k")]
    top_k: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CorpusDocument {
    id: Option<String>,
    text: String,
}

fn lexical_analyze(request: LexicalAnalyzeRequest) -> Result<serde_json::Value, String> {
    let stop_words = request
        .language
        .as_deref()
        .map(stop_words_for_language)
        .unwrap_or_else(english_stop_words);
    let keyword_options = KeywordOptions {
        max_terms: request.max_terms,
        stop_words: stop_words.clone(),
        ..KeywordOptions::default()
    };
    let phrase_options = PhraseKeywordOptions {
        max_phrases: request.max_terms,
        stop_words: stop_words.clone(),
        ..PhraseKeywordOptions::default()
    };
    let summary_options = ExtractiveSummaryOptions {
        max_sentences: request.max_summary_sentences,
        stop_words,
        ..ExtractiveSummaryOptions::default()
    };
    let extractive =
        extractive_summary(&request.text, &summary_options).map_err(|error| error.to_string())?;
    let diverse = diverse_extractive_summary(&request.text, &summary_options, 0.35)
        .map_err(|error| error.to_string())?;
    Ok(serde_json::json!({
        "summary": crate::summarize_text(&request.text, request.max_terms),
        "keywords": keywords(&request.text, &keyword_options),
        "phraseKeywords": phrase_keywords(&request.text, &phrase_options),
        "readability": readability_summary(&request.text, &Default::default()),
        "sentiment": sentiment(&request.text, &SentimentLexicon::default()),
        "extractiveSummary": extractive,
        "diverseExtractiveSummary": diverse,
        "ruleEntities": rule_entities(&request.text, &EntityRuleSet::default())
    }))
}

fn lexical_keywords(request: LexicalKeywordsRequest) -> Result<serde_json::Value, String> {
    let stop_words = request
        .language
        .as_deref()
        .map(stop_words_for_language)
        .unwrap_or_else(english_stop_words);
    let mut ranked = keywords(
        &request.text,
        &KeywordOptions {
            max_terms: request.limit,
            stop_words,
            ..KeywordOptions::default()
        },
    );
    ranked.retain(|keyword| keyword.score >= request.min_score);
    Ok(serde_json::json!({ "keywords": ranked }))
}

fn corpus_search(request: CorpusSearchRequest) -> Result<serde_json::Value, String> {
    let ids = request
        .documents
        .iter()
        .enumerate()
        .map(|(index, document)| {
            document
                .id
                .clone()
                .unwrap_or_else(|| format!("doc-{index}"))
        })
        .collect::<Vec<_>>();
    let docs = request
        .documents
        .iter()
        .enumerate()
        .map(|(index, document)| TextDocument::new(ids[index].as_str(), &document.text))
        .collect::<Vec<_>>();
    match request.mode.as_str() {
        "tfidf" => {
            let corpus = TfIdfCorpus::from_documents(docs, Default::default())
                .map_err(|error| error.to_string())?;
            Ok(
                serde_json::json!({"mode": "tfidf", "results": corpus.search(&request.query, request.top_k).map_err(|error| error.to_string())?}),
            )
        }
        "bm25" => {
            let corpus = Bm25Corpus::from_documents(docs, Bm25Options::default())
                .map_err(|error| error.to_string())?;
            Ok(
                serde_json::json!({"mode": "bm25", "results": corpus.search(&request.query, request.top_k).map_err(|error| error.to_string())?}),
            )
        }
        other => Err(format!("unsupported corpus search mode `{other}`")),
    }
}

fn parse_input<T: for<'de> Deserialize<'de>>(input: serde_json::Value) -> Result<T, String> {
    serde_json::from_value(input).map_err(|error| format!("invalid request: {error}"))
}

fn default_terms() -> usize {
    10
}
fn default_summary_sentences() -> usize {
    3
}
fn default_top_k() -> usize {
    10
}
fn default_corpus_mode() -> String {
    "bm25".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_surface_lists_lexical_operations() {
        let ids = package_surface()
            .operations
            .into_iter()
            .map(|operation| operation.id.0)
            .collect::<Vec<_>>();
        assert!(ids.contains(&"lexical.analyze".to_string()));
        assert!(ids.contains(&"lexical.corpusSearch".to_string()));
    }

    #[test]
    fn keyword_operation_is_deterministic() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("lexical.keywords"),
            input: serde_json::json!({"text": "rust text text analysis", "limit": 2}),
        })
        .expect("keywords");
        assert_eq!(response.value["keywords"][0]["text"], "text");
    }

    #[test]
    fn malformed_input_returns_typed_error_string() {
        let error = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("lexical.keywords"),
            input: serde_json::json!({"text": 42}),
        })
        .expect_err("invalid request");
        assert!(error.contains("invalid request"));
    }
}
