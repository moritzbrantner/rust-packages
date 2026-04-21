//! Shared building blocks for linguistics packages.

use std::fmt;
use std::str::FromStr;

/// A byte span into a source string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextSpan {
    start: usize,
    end: usize,
}

impl TextSpan {
    /// Creates a new span when `start <= end`.
    ///
    /// # Errors
    ///
    /// Returns [`LinguisticsError::InvalidSpan`] when `start` is greater than
    /// `end`.
    pub fn new(start: usize, end: usize) -> Result<Self, LinguisticsError> {
        if start > end {
            return Err(LinguisticsError::InvalidSpan { start, end });
        }

        Ok(Self { start, end })
    }

    /// Start byte offset.
    #[must_use]
    pub const fn start(self) -> usize {
        self.start
    }

    /// End byte offset.
    #[must_use]
    pub const fn end(self) -> usize {
        self.end
    }

    /// Length in bytes.
    #[must_use]
    pub const fn len(self) -> usize {
        self.end - self.start
    }

    /// Whether the span is empty.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }
}

/// A token and its source span.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Token {
    text: String,
    span: TextSpan,
}

impl Token {
    /// Creates a token from owned or borrowed text and its source span.
    pub fn new(text: impl Into<String>, span: TextSpan) -> Self {
        Self {
            text: text.into(),
            span,
        }
    }

    /// Token surface form.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Source span.
    #[must_use]
    pub const fn span(&self) -> TextSpan {
        self.span
    }
}

/// A lightweight BCP 47 style language tag.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LanguageTag(String);

impl LanguageTag {
    /// Creates a normalized language tag.
    ///
    /// The validation here is intentionally conservative: subtags must be
    /// non-empty ASCII alphanumeric sequences separated by `-`.
    ///
    /// # Errors
    ///
    /// Returns [`LinguisticsError::InvalidLanguageTag`] when the tag is empty
    /// or contains invalid subtags.
    pub fn new(tag: impl Into<String>) -> Result<Self, LinguisticsError> {
        let tag = tag.into();
        let normalized = tag.trim().to_ascii_lowercase();
        let valid = !normalized.is_empty()
            && normalized
                .split('-')
                .all(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_alphanumeric()));

        if valid {
            Ok(Self(normalized))
        } else {
            Err(LinguisticsError::InvalidLanguageTag(tag))
        }
    }

    /// Returns the normalized tag.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for LanguageTag {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl FromStr for LanguageTag {
    type Err = LinguisticsError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::new(value)
    }
}

/// Errors shared by the linguistics crates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinguisticsError {
    /// A span was constructed with `start > end`.
    InvalidSpan { start: usize, end: usize },
    /// A language tag failed validation.
    InvalidLanguageTag(String),
}

impl fmt::Display for LinguisticsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSpan { start, end } => {
                write!(formatter, "invalid span: start {start} is after end {end}")
            }
            Self::InvalidLanguageTag(tag) => write!(formatter, "invalid language tag: {tag}"),
        }
    }
}

impl std::error::Error for LinguisticsError {}

#[cfg(test)]
mod tests {
    use super::{LanguageTag, LinguisticsError, TextSpan, Token};

    #[test]
    fn span_rejects_reversed_offsets() {
        assert_eq!(
            TextSpan::new(10, 2),
            Err(LinguisticsError::InvalidSpan { start: 10, end: 2 })
        );
    }

    #[test]
    fn token_stores_text_and_span() {
        let span = TextSpan::new(0, 5).expect("valid span");
        let token = Token::new("hello", span);

        assert_eq!(token.text(), "hello");
        assert_eq!(token.span(), span);
    }

    #[test]
    fn language_tag_is_normalized() {
        let tag: LanguageTag = " EN-us ".parse().expect("valid language tag");

        assert_eq!(tag.as_str(), "en-us");
        assert_eq!(tag.to_string(), "en-us");
    }
}
