//! Morpheme models and deterministic segmentation utilities.

use linguistics_core::{TextSpan, Token};
use std::cmp::Reverse;

/// The distributional role of a morpheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MorphemeKind {
    /// A free or bound root.
    Root,
    /// A prefix.
    Prefix,
    /// A suffix.
    Suffix,
    /// An infix.
    Infix,
    /// Any other morpheme type.
    Other,
}

/// A lexical morpheme entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Morpheme {
    form: String,
    gloss: String,
    kind: MorphemeKind,
}

impl Morpheme {
    /// Creates a morpheme entry.
    #[must_use]
    pub fn new(form: impl Into<String>, gloss: impl Into<String>, kind: MorphemeKind) -> Self {
        Self {
            form: form.into(),
            gloss: gloss.into(),
            kind,
        }
    }

    /// Surface form.
    #[must_use]
    pub fn form(&self) -> &str {
        &self.form
    }

    /// Gloss or meaning label.
    #[must_use]
    pub fn gloss(&self) -> &str {
        &self.gloss
    }

    /// Morpheme kind.
    #[must_use]
    pub const fn kind(&self) -> MorphemeKind {
        self.kind
    }
}

/// A morpheme matched inside a token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    morpheme: Morpheme,
    token: Token,
}

impl Segment {
    /// Creates a segment.
    #[must_use]
    pub fn new(morpheme: Morpheme, token: Token) -> Self {
        Self { morpheme, token }
    }

    /// Matched morpheme.
    #[must_use]
    pub const fn morpheme(&self) -> &Morpheme {
        &self.morpheme
    }

    /// Token for the matched text and span.
    #[must_use]
    pub const fn token(&self) -> &Token {
        &self.token
    }
}

/// A longest-match segmenter backed by a small morpheme lexicon.
#[derive(Debug, Clone, Default)]
pub struct Segmenter {
    lexicon: Vec<Morpheme>,
}

impl Segmenter {
    /// Creates an empty segmenter.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            lexicon: Vec::new(),
        }
    }

    /// Adds a morpheme entry.
    pub fn add(&mut self, morpheme: Morpheme) {
        self.lexicon.push(morpheme);
        self.lexicon
            .sort_by_key(|morpheme| Reverse(morpheme.form.len()));
    }

    /// Segments a word with greedy longest matching.
    ///
    /// # Panics
    ///
    /// Panics only if internally calculated monotonic byte offsets fail span
    /// validation.
    #[must_use]
    pub fn segment(&self, word: &str) -> Vec<Segment> {
        let mut offset = 0;
        let mut segments = Vec::new();

        while offset < word.len() {
            let remainder = &word[offset..];
            let matched = self
                .lexicon
                .iter()
                .find(|morpheme| remainder.starts_with(&morpheme.form));

            if let Some(morpheme) = matched {
                let end = offset + morpheme.form.len();
                let span = TextSpan::new(offset, end).expect("offsets are monotonic");
                let token = Token::new(&word[offset..end], span);
                segments.push(Segment::new(morpheme.clone(), token));
                offset = end;
            } else {
                let next = next_char_boundary(word, offset);
                let span = TextSpan::new(offset, next).expect("offsets are monotonic");
                let token = Token::new(&word[offset..next], span);
                let unknown = Morpheme::new(&word[offset..next], "?", MorphemeKind::Other);
                segments.push(Segment::new(unknown, token));
                offset = next;
            }
        }

        segments
    }
}

fn next_char_boundary(text: &str, offset: usize) -> usize {
    text[offset..]
        .char_indices()
        .nth(1)
        .map_or(text.len(), |(index, _)| offset + index)
}

#[cfg(test)]
mod tests {
    use super::{Morpheme, MorphemeKind, Segmenter};

    #[test]
    fn segmenter_uses_longest_match() {
        let mut segmenter = Segmenter::new();
        segmenter.add(Morpheme::new("un", "NEG", MorphemeKind::Prefix));
        segmenter.add(Morpheme::new("lock", "OPEN", MorphemeKind::Root));
        segmenter.add(Morpheme::new("able", "ABLE", MorphemeKind::Suffix));
        segmenter.add(Morpheme::new("lockable", "OPEN.ABLE", MorphemeKind::Root));

        let segments = segmenter.segment("unlockable");
        let forms = segments
            .iter()
            .map(|segment| segment.morpheme().form())
            .collect::<Vec<_>>();

        assert_eq!(forms, ["un", "lockable"]);
    }

    #[test]
    fn segmenter_keeps_unknown_unicode_characters() {
        let segmenter = Segmenter::new();
        let segments = segmenter.segment("ø");

        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].token().text(), "ø");
        assert_eq!(segments[0].token().span().len(), "ø".len());
    }
}
