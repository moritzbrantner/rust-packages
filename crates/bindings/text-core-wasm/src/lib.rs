//! WASM bindings for text-core.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::{from_value, Serializer};
use text_core::{
    detailed_text_stats, detect_script_profile, split_paragraphs, split_sentence_spans, tokenize,
    TextProcessingOptions, TokenKind,
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

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct RawTextProcessingOptions {
    lowercase: Option<bool>,
    normalize_unicode: Option<bool>,
    keep_apostrophes: Option<bool>,
    include_punctuation: Option<bool>,
    include_tokens: Option<bool>,
}

#[derive(Serialize)]
struct RawAnalyzedToken {
    start: usize,
    end: usize,
    text: String,
    normalized: String,
    kind: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RawTextStats {
    bytes: usize,
    chars: usize,
    words: usize,
    lines: usize,
    sentences: usize,
    paragraphs: usize,
    tokens: usize,
    unique_tokens: usize,
    average_words_per_sentence: f32,
    average_chars_per_word: f32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RawScriptProfile {
    scripts: BTreeMap<String, usize>,
    digits: usize,
    whitespace: usize,
    punctuation: usize,
    other: usize,
    dominant_script: Option<String>,
    is_mixed: bool,
}

#[derive(Serialize)]
struct RawSegmentedDocument {
    paragraphs: Vec<RawSpan>,
    sentences: Vec<RawSpan>,
    tokens: Vec<RawToken>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RawAnalyzedDocument {
    stats: RawTextStats,
    script_profile: RawScriptProfile,
    paragraphs: Vec<RawSpan>,
    sentences: Vec<RawSpan>,
    tokens: Vec<RawAnalyzedToken>,
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

#[wasm_bindgen(js_name = analyzeTextDocument)]
/// Returns analyze text document binding.
pub fn analyze_text_document_binding(
    text: &str,
    options: Option<JsValue>,
) -> Result<JsValue, JsValue> {
    let raw_options = match options {
        Some(value) if !value.is_null() && !value.is_undefined() => {
            from_value(value).map_err(into_js_error)?
        }
        _ => RawTextProcessingOptions::default(),
    };
    to_js_value(&analyze_text_document_data(text, &raw_options))
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

fn analyze_text_document_data(
    text: &str,
    raw_options: &RawTextProcessingOptions,
) -> RawAnalyzedDocument {
    let default_options = TextProcessingOptions::default();
    let options = TextProcessingOptions {
        lowercase: raw_options.lowercase.unwrap_or(default_options.lowercase),
        normalize_unicode: raw_options
            .normalize_unicode
            .unwrap_or(default_options.normalize_unicode),
        keep_apostrophes: raw_options
            .keep_apostrophes
            .unwrap_or(default_options.keep_apostrophes),
        include_punctuation: raw_options
            .include_punctuation
            .unwrap_or(default_options.include_punctuation),
        ..default_options
    };
    let include_tokens = raw_options.include_tokens.unwrap_or(true);

    let detailed = detailed_text_stats(text, &options);
    let script_profile = detect_script_profile(text);
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
            .map(|token| RawAnalyzedToken {
                start: js_index_for_byte(text, token.span.byte_start),
                end: js_index_for_byte(text, token.span.byte_end),
                text: token.text,
                normalized: token.normalized,
                kind: token_kind_name(token.kind),
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    RawAnalyzedDocument {
        stats: RawTextStats {
            bytes: detailed.basic.bytes,
            chars: detailed.basic.chars,
            words: detailed.basic.words,
            lines: detailed.basic.lines,
            sentences: detailed.basic.sentences,
            paragraphs: detailed.paragraphs,
            tokens: detailed.tokens,
            unique_tokens: detailed.unique_tokens,
            average_words_per_sentence: detailed.average_words_per_sentence,
            average_chars_per_word: detailed.average_chars_per_word,
        },
        script_profile: RawScriptProfile {
            scripts: script_profile.scripts,
            digits: script_profile.digits,
            whitespace: script_profile.whitespace,
            punctuation: script_profile.punctuation,
            other: script_profile.other,
            dominant_script: script_profile.dominant_script,
            is_mixed: script_profile.is_mixed,
        },
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
    value
        .serialize(&Serializer::json_compatible())
        .map_err(into_js_error)
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

    #[test]
    fn analyze_text_document_data_reports_stats_scripts_and_normalized_tokens() {
        let document = analyze_text_document_data(
            "Hello 東京!\n\nCafe\u{301} time.",
            &RawTextProcessingOptions {
                include_punctuation: Some(true),
                ..RawTextProcessingOptions::default()
            },
        );
        let value = serde_json::to_value(&document).unwrap();

        assert_eq!(value["stats"]["paragraphs"], json!(2));
        assert_eq!(value["stats"]["sentences"], json!(2));
        assert_eq!(value["scriptProfile"]["scripts"]["Latin"], json!(13));
        assert_eq!(value["scriptProfile"]["scripts"]["Han"], json!(2));
        assert_eq!(value["scriptProfile"]["isMixed"], json!(true));
        assert_eq!(value["tokens"][0]["text"], json!("Hello"));
        assert_eq!(value["tokens"][0]["normalized"], json!("hello"));
        assert_eq!(value["tokens"][1]["text"], json!("東京"));
        assert_eq!(value["tokens"][2]["kind"], json!("punctuation"));
    }

    #[test]
    fn analyze_text_document_data_can_omit_tokens_without_changing_stats() {
        let document = analyze_text_document_data(
            "Hello, world.",
            &RawTextProcessingOptions {
                include_punctuation: Some(true),
                include_tokens: Some(false),
                ..RawTextProcessingOptions::default()
            },
        );
        assert!(document.tokens.is_empty());
        assert_eq!(document.stats.tokens, 4);
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
