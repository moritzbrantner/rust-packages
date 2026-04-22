use std::collections::BTreeMap;

use unicode_normalization::UnicodeNormalization;
use video_analysis_core::{OwnedTextSegment, TextSegment, Timestamp};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextDocument<'a> {
    pub id: &'a str,
    pub text: &'a str,
    pub language: Option<&'a str>,
    pub timestamp: Option<Timestamp>,
}

impl<'a> TextDocument<'a> {
    pub fn new(id: &'a str, text: &'a str) -> Self {
        Self {
            id,
            text,
            language: None,
            timestamp: None,
        }
    }

    pub fn from_segment(stream_id: &'a str, segment: &TextSegment<'a>) -> Self {
        Self {
            id: stream_id,
            text: segment.text,
            language: segment.language,
            timestamp: segment.timestamp,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedTextDocument {
    pub id: String,
    pub text: String,
    pub language: Option<String>,
    pub timestamp: Option<Timestamp>,
}

impl OwnedTextDocument {
    pub fn new(id: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            language: None,
            timestamp: None,
        }
    }

    pub fn language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }

    pub fn timestamp(mut self, timestamp: Timestamp) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    pub fn from_segment(stream_id: impl Into<String>, segment: &OwnedTextSegment) -> Self {
        let segment = segment.as_segment();
        Self {
            id: stream_id.into(),
            text: segment.text.to_string(),
            language: segment.language.map(ToString::to_string),
            timestamp: segment.timestamp,
        }
    }

    pub fn as_document(&self) -> TextDocument<'_> {
        TextDocument {
            id: &self.id,
            text: &self.text,
            language: self.language.as_deref(),
            timestamp: self.timestamp,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextStats {
    pub bytes: usize,
    pub chars: usize,
    pub words: usize,
    pub lines: usize,
    pub sentences: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextSpan {
    pub byte_start: usize,
    pub byte_end: usize,
    pub char_start: usize,
    pub char_end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub text: String,
    pub normalized: String,
    pub span: TextSpan,
    pub kind: TokenKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Word,
    Number,
    Url,
    Email,
    Mention,
    Hashtag,
    Punctuation,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sentence {
    pub text: String,
    pub span: TextSpan,
    pub token_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paragraph {
    pub text: String,
    pub span: TextSpan,
    pub sentence_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextProcessingOptions {
    pub language: Option<String>,
    pub lowercase: bool,
    pub normalize_unicode: bool,
    pub keep_apostrophes: bool,
    pub include_punctuation: bool,
}

impl Default for TextProcessingOptions {
    fn default() -> Self {
        Self {
            language: None,
            lowercase: true,
            normalize_unicode: true,
            keep_apostrophes: true,
            include_punctuation: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DetailedTextStats {
    pub basic: TextStats,
    pub paragraphs: usize,
    pub tokens: usize,
    pub unique_tokens: usize,
    pub average_words_per_sentence: f32,
    pub average_chars_per_word: f32,
}

pub fn text_stats(text: &str) -> TextStats {
    TextStats {
        bytes: text.len(),
        chars: text.chars().count(),
        words: tokenize_words(text).len(),
        lines: text.lines().count(),
        sentences: split_sentences(text).len(),
    }
}

pub fn normalize_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn normalize_text(text: &str, options: &TextProcessingOptions) -> String {
    let normalized = if options.normalize_unicode {
        text.nfkc().collect::<String>()
    } else {
        text.to_string()
    };
    if options.lowercase {
        normalized.to_lowercase()
    } else {
        normalized
    }
}

pub fn tokenize_words(text: &str) -> Vec<String> {
    let options = TextProcessingOptions::default();
    tokenize(text, &options)
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
        .map(|token| token.normalized)
        .collect()
}

pub fn word_counts(text: &str) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for token in tokenize_words(text) {
        *counts.entry(token).or_insert(0) += 1;
    }
    counts
}

pub fn split_sentences(text: &str) -> Vec<String> {
    split_sentence_spans(text, &TextProcessingOptions::default())
        .into_iter()
        .map(|sentence| normalize_whitespace(&sentence.text))
        .collect()
}

pub fn tokenize(text: &str, options: &TextProcessingOptions) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut byte_index = 0;

    while byte_index < text.len() {
        let ch = next_char(text, byte_index);
        if ch.is_whitespace() {
            byte_index += ch.len_utf8();
            continue;
        }

        let (byte_end, kind) = if starts_url(text, byte_index) {
            (consume_until_whitespace(text, byte_index), TokenKind::Url)
        } else if ch == '@' {
            let end = consume_prefixed_word(text, byte_index);
            if end > byte_index + ch.len_utf8() {
                (end, TokenKind::Mention)
            } else {
                (byte_index + ch.len_utf8(), TokenKind::Other)
            }
        } else if ch == '#' {
            let end = consume_prefixed_word(text, byte_index);
            if end > byte_index + ch.len_utf8() {
                (end, TokenKind::Hashtag)
            } else {
                (byte_index + ch.len_utf8(), TokenKind::Other)
            }
        } else if ch.is_ascii_digit() {
            (consume_number(text, byte_index), TokenKind::Number)
        } else if is_word_char(ch, options.keep_apostrophes) {
            let mut end = consume_word_like(text, byte_index, options.keep_apostrophes);
            let candidate_end =
                trim_trailing_token_punctuation(text, byte_index, end, TokenKind::Email);
            let candidate = &text[byte_index..candidate_end];
            let kind = if is_email(candidate) {
                end = candidate_end;
                TokenKind::Email
            } else {
                end = consume_plain_word(text, byte_index, options.keep_apostrophes);
                TokenKind::Word
            };
            (end, kind)
        } else if is_sentence_or_symbol_punctuation(ch) {
            (byte_index + ch.len_utf8(), TokenKind::Punctuation)
        } else {
            (byte_index + ch.len_utf8(), TokenKind::Other)
        };

        let byte_end = trim_trailing_token_punctuation(text, byte_index, byte_end, kind);
        if byte_end == byte_index {
            byte_index += ch.len_utf8();
            continue;
        }
        if kind != TokenKind::Punctuation || options.include_punctuation {
            let raw = &text[byte_index..byte_end];
            tokens.push(Token {
                text: raw.to_string(),
                normalized: normalize_text(raw, options),
                span: span_for(text, byte_index, byte_end),
                kind,
            });
        }
        byte_index = byte_end;
    }

    tokens
}

pub fn split_sentence_spans(text: &str, options: &TextProcessingOptions) -> Vec<Sentence> {
    let mut sentences = Vec::new();
    let mut start = 0;
    let chars = text.char_indices().collect::<Vec<_>>();

    for (position, (byte_index, ch)) in chars.iter().copied().enumerate() {
        if !is_sentence_terminator(ch) {
            continue;
        }
        if ch == '.'
            && previous_char(&chars, position).is_some_and(|value| value.is_ascii_digit())
            && next_char_from_indices(&chars, position).is_some_and(|value| value.is_ascii_digit())
        {
            continue;
        }
        if next_char_from_indices(&chars, position).is_some_and(is_sentence_terminator) {
            continue;
        }

        let end = byte_index + ch.len_utf8();
        push_sentence(text, start, end, options, &mut sentences);
        start = end;
    }

    push_sentence(text, start, text.len(), options, &mut sentences);
    sentences
}

pub fn split_paragraphs(text: &str) -> Vec<Paragraph> {
    let mut paragraphs = Vec::new();
    let mut paragraph_start = None;
    let mut last_non_blank_end = 0;
    let mut line_start = 0;

    for line in text.split_inclusive('\n') {
        let line_end = line_start + line.len();
        let line_without_newline = line.trim_end_matches(['\r', '\n']);
        if line_without_newline.trim().is_empty() {
            if let Some(start) = paragraph_start.take() {
                push_paragraph(text, start, last_non_blank_end, &mut paragraphs);
            }
        } else {
            let content_start =
                line_start + (line_without_newline.len() - line_without_newline.trim_start().len());
            paragraph_start.get_or_insert(content_start);
            last_non_blank_end = line_start + line_without_newline.trim_end().len();
        }
        line_start = line_end;
    }

    if let Some(start) = paragraph_start {
        push_paragraph(text, start, last_non_blank_end, &mut paragraphs);
    }

    paragraphs
}

pub fn detailed_text_stats(text: &str, options: &TextProcessingOptions) -> DetailedTextStats {
    let basic = text_stats(text);
    let paragraphs = split_paragraphs(text).len();
    let tokens = tokenize(text, options);
    let unique_tokens = tokens
        .iter()
        .filter(|token| token.kind != TokenKind::Punctuation)
        .map(|token| token.normalized.clone())
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let chars_in_words = tokens
        .iter()
        .filter(|token| matches!(token.kind, TokenKind::Word | TokenKind::Number))
        .map(|token| token.text.chars().count())
        .sum::<usize>();
    DetailedTextStats {
        basic,
        paragraphs,
        tokens: tokens.len(),
        unique_tokens,
        average_words_per_sentence: if basic.sentences == 0 {
            0.0
        } else {
            basic.words as f32 / basic.sentences as f32
        },
        average_chars_per_word: if basic.words == 0 {
            0.0
        } else {
            chars_in_words as f32 / basic.words as f32
        },
    }
}

fn next_char(text: &str, byte_index: usize) -> char {
    text[byte_index..]
        .chars()
        .next()
        .expect("byte_index must be inside text")
}

fn span_for(text: &str, byte_start: usize, byte_end: usize) -> TextSpan {
    TextSpan {
        byte_start,
        byte_end,
        char_start: text[..byte_start].chars().count(),
        char_end: text[..byte_end].chars().count(),
    }
}

fn starts_url(text: &str, byte_index: usize) -> bool {
    let tail = &text[byte_index..];
    tail.starts_with("http://") || tail.starts_with("https://") || tail.starts_with("www.")
}

fn consume_until_whitespace(text: &str, byte_start: usize) -> usize {
    let mut end = byte_start;
    for (offset, ch) in text[byte_start..].char_indices() {
        if ch.is_whitespace() {
            break;
        }
        end = byte_start + offset + ch.len_utf8();
    }
    end
}

fn consume_prefixed_word(text: &str, byte_start: usize) -> usize {
    let base = byte_start + next_char(text, byte_start).len_utf8();
    let mut end = base;
    for (offset, ch) in text[base..].char_indices() {
        if ch.is_alphanumeric() || ch == '_' || ch == '-' {
            end = base + offset + ch.len_utf8();
        } else {
            break;
        }
    }
    end
}

fn consume_number(text: &str, byte_start: usize) -> usize {
    let mut end = byte_start;
    for (offset, ch) in text[byte_start..].char_indices() {
        if ch.is_ascii_digit() || matches!(ch, '.' | ',' | ':' | '/' | '-') {
            end = byte_start + offset + ch.len_utf8();
        } else {
            break;
        }
    }
    end
}

fn consume_word_like(text: &str, byte_start: usize, keep_apostrophes: bool) -> usize {
    let mut end = byte_start;
    for (offset, ch) in text[byte_start..].char_indices() {
        if is_word_char(ch, keep_apostrophes) || matches!(ch, '@' | '.' | '_' | '-' | '+') {
            end = byte_start + offset + ch.len_utf8();
        } else {
            break;
        }
    }
    end
}

fn consume_plain_word(text: &str, byte_start: usize, keep_apostrophes: bool) -> usize {
    let mut end = byte_start;
    for (offset, ch) in text[byte_start..].char_indices() {
        if is_word_char(ch, keep_apostrophes) {
            end = byte_start + offset + ch.len_utf8();
        } else {
            break;
        }
    }
    end
}

fn is_word_char(ch: char, keep_apostrophes: bool) -> bool {
    ch.is_alphanumeric() || (keep_apostrophes && is_apostrophe(ch))
}

fn is_apostrophe(ch: char) -> bool {
    matches!(ch, '\'' | '’')
}

fn is_email(candidate: &str) -> bool {
    let Some((local, domain)) = candidate.split_once('@') else {
        return false;
    };
    !local.is_empty() && domain.contains('.') && !domain.starts_with('.') && !domain.ends_with('.')
}

fn is_sentence_or_symbol_punctuation(ch: char) -> bool {
    ch.is_ascii_punctuation()
        || matches!(
            ch,
            '…' | '。' | '！' | '？' | '،' | '؛' | '¿' | '¡' | '«' | '»'
        )
}

fn trim_trailing_token_punctuation(
    text: &str,
    byte_start: usize,
    mut byte_end: usize,
    kind: TokenKind,
) -> usize {
    if !matches!(kind, TokenKind::Url | TokenKind::Email | TokenKind::Number) {
        return byte_end;
    }
    while byte_end > byte_start {
        let Some(ch) = text[..byte_end].chars().next_back() else {
            break;
        };
        if matches!(ch, '.' | ',' | ';' | ':' | '!' | '?' | ')' | ']' | '}') {
            byte_end -= ch.len_utf8();
        } else {
            break;
        }
    }
    byte_end
}

fn is_sentence_terminator(ch: char) -> bool {
    matches!(ch, '.' | '?' | '!' | '…' | '。' | '！' | '？')
}

fn previous_char(chars: &[(usize, char)], position: usize) -> Option<char> {
    position
        .checked_sub(1)
        .and_then(|index| chars.get(index).map(|(_, ch)| *ch))
}

fn next_char_from_indices(chars: &[(usize, char)], position: usize) -> Option<char> {
    chars.get(position + 1).map(|(_, ch)| *ch)
}

fn push_sentence(
    text: &str,
    byte_start: usize,
    byte_end: usize,
    options: &TextProcessingOptions,
    sentences: &mut Vec<Sentence>,
) {
    if byte_start >= byte_end {
        return;
    }
    let raw = &text[byte_start..byte_end];
    let leading = raw.len() - raw.trim_start().len();
    let trailing = raw.trim_end().len();
    let start = byte_start + leading;
    let end = byte_start + trailing;
    if start >= end {
        return;
    }
    let sentence_text = text[start..end].to_string();
    let token_count = tokenize(&sentence_text, options).len();
    sentences.push(Sentence {
        text: sentence_text,
        span: span_for(text, start, end),
        token_count,
    });
}

fn push_paragraph(text: &str, byte_start: usize, byte_end: usize, paragraphs: &mut Vec<Paragraph>) {
    if byte_start >= byte_end {
        return;
    }
    let paragraph_text = text[byte_start..byte_end].to_string();
    paragraphs.push(Paragraph {
        sentence_count: split_sentence_spans(&paragraph_text, &TextProcessingOptions::default())
            .len(),
        text: paragraph_text,
        span: span_for(text, byte_start, byte_end),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_and_counts_words() {
        let counts = word_counts("Hello, hello world.");
        assert_eq!(counts.get("hello"), Some(&2));
        assert_eq!(counts.get("world"), Some(&1));
    }

    #[test]
    fn computes_text_stats() {
        let stats = text_stats("One sentence. Two words!");
        assert_eq!(stats.sentences, 2);
        assert_eq!(stats.words, 4);
    }

    #[test]
    fn tokenizes_unicode_words_with_offsets() {
        let tokens = tokenize("Hi café 東京", &TextProcessingOptions::default());
        assert_eq!(tokens[1].text, "café");
        assert_eq!(tokens[1].span.byte_start, 3);
        assert_eq!(tokens[1].span.char_start, 3);
        assert_eq!(tokens[2].text, "東京");
    }

    #[test]
    fn classifies_common_token_patterns() {
        let tokens = tokenize(
            "Mail a@b.com. #rust @team https://example.com 3.14",
            &TextProcessingOptions::default(),
        );
        assert!(tokens.iter().any(|token| token.kind == TokenKind::Email));
        assert!(tokens.iter().any(|token| token.kind == TokenKind::Hashtag));
        assert!(tokens.iter().any(|token| token.kind == TokenKind::Mention));
        assert!(tokens.iter().any(|token| token.kind == TokenKind::Url));
        assert!(tokens.iter().any(|token| token.kind == TokenKind::Number));
    }

    #[test]
    fn apostrophe_behavior_is_configurable() {
        let keep = TextProcessingOptions::default();
        assert_eq!(tokenize_words("Don't stop"), vec!["don't", "stop"]);

        let split = TextProcessingOptions {
            keep_apostrophes: false,
            ..TextProcessingOptions::default()
        };
        let tokens = tokenize("Don't", &split)
            .into_iter()
            .map(|token| token.normalized)
            .collect::<Vec<_>>();
        assert_eq!(tokens, vec!["don", "t"]);
        assert_eq!(tokenize("Don't", &keep)[0].normalized, "don't");
    }

    #[test]
    fn splits_sentences_with_decimals_ellipses_and_multilingual_marks() {
        let sentences = split_sentences("Pi is 3.14. Wait... Really？ Yes!");
        assert_eq!(
            sentences,
            vec!["Pi is 3.14.", "Wait...", "Really？", "Yes!"]
        );
    }

    #[test]
    fn splits_paragraphs_on_blank_lines() {
        let paragraphs = split_paragraphs("First paragraph.\nStill first.\n\nSecond.");
        assert_eq!(paragraphs.len(), 2);
        assert_eq!(paragraphs[0].sentence_count, 2);
        assert_eq!(paragraphs[1].text, "Second.");
    }

    #[test]
    fn detailed_stats_include_derived_counts() {
        let stats = detailed_text_stats("One sentence.\n\nTwo words here.", &Default::default());
        assert_eq!(stats.paragraphs, 2);
        assert_eq!(stats.basic.sentences, 2);
        assert!(stats.average_words_per_sentence > 0.0);
    }
}
