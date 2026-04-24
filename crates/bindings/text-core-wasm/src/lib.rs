use serde::Serialize;
use serde_wasm_bindgen::to_value;
use text_analysis_core::{
    split_paragraphs, split_sentence_spans, tokenize, TextProcessingOptions, TokenKind,
};
use wasm_bindgen::prelude::*;

#[derive(Serialize)]
struct RawSpan {
    start: usize,
    end: usize,
    text: String,
}

#[derive(Serialize)]
struct RawToken {
    start: usize,
    end: usize,
    text: String,
    kind: &'static str,
}

#[derive(Serialize)]
struct RawSegmentedDocument {
    paragraphs: Vec<RawSpan>,
    sentences: Vec<RawSpan>,
    tokens: Vec<RawToken>,
}

#[wasm_bindgen(js_name = extractWordTexts)]
pub fn extract_word_texts(text: &str) -> Result<JsValue, JsValue> {
    let words = tokenize(text, &TextProcessingOptions::default())
        .into_iter()
        .filter(|token| {
            matches!(
                token.kind,
                TokenKind::Word
                    | TokenKind::Number
                    | TokenKind::Url
                    | TokenKind::Email
                    | TokenKind::Mention
                    | TokenKind::Hashtag
            )
        })
        .map(|token| token.text)
        .collect::<Vec<_>>();
    to_value(&words).map_err(into_js_error)
}

#[wasm_bindgen(js_name = splitSentences)]
pub fn split_sentences_binding(text: &str) -> Result<JsValue, JsValue> {
    let sentences = split_sentence_spans(text, &TextProcessingOptions::default())
        .into_iter()
        .map(|sentence| sentence.text)
        .collect::<Vec<_>>();
    to_value(&sentences).map_err(into_js_error)
}

#[wasm_bindgen(js_name = segmentTextDocument)]
pub fn segment_text_document_binding(
    text: &str,
    keep_apostrophes: bool,
    include_punctuation: bool,
    include_tokens: bool,
) -> Result<JsValue, JsValue> {
    let options = TextProcessingOptions {
        keep_apostrophes,
        include_punctuation,
        ..TextProcessingOptions::default()
    };
    let paragraphs = split_paragraphs(text)
        .into_iter()
        .map(|paragraph| raw_span(text, paragraph.span.byte_start, paragraph.span.byte_end))
        .collect::<Vec<_>>();
    let sentences = split_sentence_spans(text, &options)
        .into_iter()
        .map(|sentence| raw_span(text, sentence.span.byte_start, sentence.span.byte_end))
        .collect::<Vec<_>>();
    let tokens = if include_tokens {
        tokenize(text, &options)
            .into_iter()
            .map(|token| RawToken {
                start: js_index_for_byte(text, token.span.byte_start),
                end: js_index_for_byte(text, token.span.byte_end),
                text: token.text,
                kind: token_kind_name(token.kind),
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    to_value(&RawSegmentedDocument {
        paragraphs,
        sentences,
        tokens,
    })
    .map_err(into_js_error)
}

fn raw_span(text: &str, byte_start: usize, byte_end: usize) -> RawSpan {
    RawSpan {
        start: js_index_for_byte(text, byte_start),
        end: js_index_for_byte(text, byte_end),
        text: text[byte_start..byte_end].to_string(),
    }
}

fn js_index_for_byte(text: &str, byte_index: usize) -> usize {
    text[..byte_index].encode_utf16().count()
}

fn token_kind_name(kind: TokenKind) -> &'static str {
    match kind {
        TokenKind::Word => "word",
        TokenKind::Number => "number",
        TokenKind::Url => "url",
        TokenKind::Email => "email",
        TokenKind::Mention => "mention",
        TokenKind::Hashtag => "hashtag",
        TokenKind::Punctuation => "punctuation",
        TokenKind::Other => "other",
    }
}

fn into_js_error(error: serde_wasm_bindgen::Error) -> JsValue {
    JsValue::from_str(&error.to_string())
}
