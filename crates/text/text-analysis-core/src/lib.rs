use std::collections::BTreeMap;

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

pub fn tokenize_words(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        if ch.is_alphanumeric() || ch == '\'' {
            current.extend(ch.to_lowercase());
        } else if !current.is_empty() {
            tokens.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

pub fn word_counts(text: &str) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for token in tokenize_words(text) {
        *counts.entry(token).or_insert(0) += 1;
    }
    counts
}

pub fn split_sentences(text: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        current.push(ch);
        if matches!(ch, '.' | '?' | '!') {
            let sentence = normalize_whitespace(&current);
            if !sentence.is_empty() {
                sentences.push(sentence);
            }
            current.clear();
        }
    }
    let tail = normalize_whitespace(&current);
    if !tail.is_empty() {
        sentences.push(tail);
    }
    sentences
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
}
