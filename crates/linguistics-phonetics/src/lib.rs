//! IPA-oriented phonetics and phonology helpers.

use linguistics_core::LanguageTag;
use std::collections::BTreeSet;

/// Broad phoneme classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PhonemeKind {
    /// Vowel-like segment.
    Vowel,
    /// Consonant-like segment.
    Consonant,
    /// Tone, stress, length, or another suprasegmental mark.
    Suprasegmental,
    /// A segment whose class is not known by this crate.
    Unknown,
}

/// A phonological feature represented by name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Feature(String);

impl Feature {
    /// Creates a normalized feature name.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into().trim().to_ascii_lowercase().replace(' ', "-");
        Self(name)
    }

    /// Returns the normalized feature name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// An IPA symbol with a broad class and feature set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Phoneme {
    symbol: String,
    kind: PhonemeKind,
    features: BTreeSet<Feature>,
}

impl Phoneme {
    /// Creates a phoneme.
    #[must_use]
    pub fn new(symbol: impl Into<String>, kind: PhonemeKind) -> Self {
        Self {
            symbol: symbol.into(),
            kind,
            features: BTreeSet::new(),
        }
    }

    /// Adds a feature and returns the updated phoneme.
    #[must_use]
    pub fn with_feature(mut self, feature: impl Into<String>) -> Self {
        self.features.insert(Feature::new(feature));
        self
    }

    /// IPA symbol.
    #[must_use]
    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    /// Broad phoneme class.
    #[must_use]
    pub const fn kind(&self) -> PhonemeKind {
        self.kind
    }

    /// Whether the phoneme has the requested feature.
    #[must_use]
    pub fn has_feature(&self, feature: &str) -> bool {
        self.features.contains(&Feature::new(feature))
    }
}

/// A language-specific phoneme inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inventory {
    language: LanguageTag,
    phonemes: Vec<Phoneme>,
}

impl Inventory {
    /// Creates an empty inventory for a language.
    #[must_use]
    pub fn new(language: LanguageTag) -> Self {
        Self {
            language,
            phonemes: Vec::new(),
        }
    }

    /// Adds a phoneme to the inventory.
    pub fn add(&mut self, phoneme: Phoneme) {
        if !self
            .phonemes
            .iter()
            .any(|item| item.symbol == phoneme.symbol)
        {
            self.phonemes.push(phoneme);
        }
    }

    /// Language tag for this inventory.
    #[must_use]
    pub const fn language(&self) -> &LanguageTag {
        &self.language
    }

    /// Finds a phoneme by IPA symbol.
    #[must_use]
    pub fn find(&self, symbol: &str) -> Option<&Phoneme> {
        self.phonemes
            .iter()
            .find(|phoneme| phoneme.symbol == symbol)
    }

    /// Iterates over phonemes in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = &Phoneme> {
        self.phonemes.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::{Inventory, Phoneme, PhonemeKind};
    use linguistics_core::LanguageTag;

    #[test]
    fn features_are_normalized() {
        let phoneme = Phoneme::new("p", PhonemeKind::Consonant).with_feature("Voiceless Stop");

        assert!(phoneme.has_feature("voiceless-stop"));
        assert!(phoneme.has_feature("Voiceless Stop"));
    }

    #[test]
    fn inventory_ignores_duplicate_symbols() {
        let language = LanguageTag::new("en").expect("valid language tag");
        let mut inventory = Inventory::new(language);

        inventory.add(Phoneme::new("p", PhonemeKind::Consonant));
        inventory.add(Phoneme::new("p", PhonemeKind::Consonant));

        assert_eq!(inventory.iter().count(), 1);
        assert_eq!(
            inventory.find("p").map(super::Phoneme::kind),
            Some(PhonemeKind::Consonant)
        );
    }
}
