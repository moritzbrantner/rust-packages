//! WASM bindings for text-analysis-core.

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
/// Returns extract word texts.
pub fn extract_word_texts(text: &str) -> Result<JsValue, JsValue> {
    to_js_value(&extract_word_texts_data(text))
}

#[wasm_bindgen(js_name = splitSentences)]
/// Returns split sentences binding.
pub fn split_sentences_binding(text: &str) -> Result<JsValue, JsValue> {
    to_js_value(&split_sentences_data(text))
}

#[wasm_bindgen(js_name = segmentTextDocument)]
/// Returns segment text document binding.
pub fn segment_text_document_binding(
    text: &str,
    keep_apostrophes: bool,
    include_punctuation: bool,
    include_tokens: bool,
) -> Result<JsValue, JsValue> {
    to_js_value(&segment_text_document_data(
        text,
        keep_apostrophes,
        include_punctuation,
        include_tokens,
    ))
}

fn extract_word_texts_data(text: &str) -> Vec<String> {
    tokenize(text, &TextProcessingOptions::default())
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
        .collect()
}

fn split_sentences_data(text: &str) -> Vec<String> {
    split_sentence_spans(text, &TextProcessingOptions::default())
        .into_iter()
        .map(|sentence| sentence.text)
        .collect()
}

fn segment_text_document_data(
    text: &str,
    keep_apostrophes: bool,
    include_punctuation: bool,
    include_tokens: bool,
) -> RawSegmentedDocument {
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

    RawSegmentedDocument {
        paragraphs,
        sentences,
        tokens,
    }
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

fn to_js_value<T: Serialize>(value: &T) -> Result<JsValue, JsValue> {
    to_value(value).map_err(into_js_error)
}

fn into_js_error(error: serde_wasm_bindgen::Error) -> JsValue {
    JsValue::from_str(&error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_json::json;

    #[derive(Debug, PartialEq)]
    struct TestRawSpan {
        start: usize,
        end: usize,
        text: String,
    }

    #[derive(Debug, PartialEq)]
    struct TestRawToken {
        start: usize,
        end: usize,
        text: String,
        kind: &'static str,
    }

    #[derive(Debug, PartialEq)]
    struct TestRawSegmentedDocument {
        paragraphs: Vec<TestRawSpan>,
        sentences: Vec<TestRawSpan>,
        tokens: Vec<TestRawToken>,
    }

    #[test]
    fn extract_word_texts_binding_contract_returns_words_and_common_symbol_tokens() {
        let words = extract_word_texts_data("Hi, @mars! Go to https://x.test 42.");
        assert_eq!(
            words,
            vec!["Hi", "@mars", "Go", "to", "https://x.test", "42"]
        );
    }

    #[test]
    fn split_sentences_binding_contract_preserves_sentence_text() {
        let sentences = split_sentences_data("One. Two? Three!");
        assert_eq!(sentences, vec!["One.", "Two?", "Three!"]);
    }

    #[test]
    fn segment_text_document_binding_contract_uses_utf16_offsets_and_optional_tokens() {
        let text = "A😀B.\n\nCafe\u{301} time.";
        let document = segment_text_document_data(text, false, true, true);

        assert_eq!(
            into_test_document(document).paragraphs,
            vec![
                TestRawSpan {
                    start: 0,
                    end: 5,
                    text: "A😀B.".to_string(),
                },
                TestRawSpan {
                    start: 7,
                    end: 18,
                    text: "Cafe\u{301} time.".to_string(),
                },
            ]
        );
        let document = into_test_document(segment_text_document_data(text, false, true, true));
        assert_eq!(
            document.sentences,
            vec![TestRawSpan {
                start: 0,
                end: 18,
                text: "A😀B.\n\nCafe\u{301} time.".to_string(),
            }]
        );
        assert_eq!(
            document.tokens,
            vec![
                TestRawToken {
                    start: 0,
                    end: 1,
                    text: "A".to_string(),
                    kind: "word",
                },
                TestRawToken {
                    start: 1,
                    end: 3,
                    text: "😀".to_string(),
                    kind: "other",
                },
                TestRawToken {
                    start: 3,
                    end: 4,
                    text: "B".to_string(),
                    kind: "word",
                },
                TestRawToken {
                    start: 4,
                    end: 5,
                    text: ".".to_string(),
                    kind: "punctuation",
                },
                TestRawToken {
                    start: 7,
                    end: 11,
                    text: "Cafe".to_string(),
                    kind: "word",
                },
                TestRawToken {
                    start: 11,
                    end: 12,
                    text: "\u{301}".to_string(),
                    kind: "other",
                },
                TestRawToken {
                    start: 13,
                    end: 17,
                    text: "time".to_string(),
                    kind: "word",
                },
                TestRawToken {
                    start: 17,
                    end: 18,
                    text: ".".to_string(),
                    kind: "punctuation",
                },
            ]
        );

        let without_tokens =
            into_test_document(segment_text_document_data(text, false, true, false));
        assert!(without_tokens.tokens.is_empty());
    }

    #[test]
    fn segment_text_document_data_serializes_with_expected_shape() {
        let document = segment_text_document_data("Hello world.", false, true, true);
        let value = serde_json::to_value(&document).unwrap();
        assert_eq!(value["paragraphs"][0]["text"], json!("Hello world."));
        assert_eq!(value["sentences"][0]["end"], json!(12));
        assert_eq!(value["tokens"][0]["kind"], json!("word"));
    }

    fn into_test_document(document: RawSegmentedDocument) -> TestRawSegmentedDocument {
        TestRawSegmentedDocument {
            paragraphs: document
                .paragraphs
                .into_iter()
                .map(|span| TestRawSpan {
                    start: span.start,
                    end: span.end,
                    text: span.text,
                })
                .collect(),
            sentences: document
                .sentences
                .into_iter()
                .map(|span| TestRawSpan {
                    start: span.start,
                    end: span.end,
                    text: span.text,
                })
                .collect(),
            tokens: document
                .tokens
                .into_iter()
                .map(|token| TestRawToken {
                    start: token.start,
                    end: token.end,
                    text: token.text,
                    kind: token.kind,
                })
                .collect(),
        }
    }
}
