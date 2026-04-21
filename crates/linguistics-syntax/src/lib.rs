//! Syntactic annotation and dependency tree helpers.

use linguistics_core::Token;

/// A compact universal-style part-of-speech tag set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PartOfSpeech {
    /// Adjective.
    Adjective,
    /// Adposition.
    Adposition,
    /// Adverb.
    Adverb,
    /// Auxiliary verb.
    Auxiliary,
    /// Coordinating conjunction.
    CoordinatingConjunction,
    /// Determiner.
    Determiner,
    /// Noun.
    Noun,
    /// Numeral.
    Numeral,
    /// Particle.
    Particle,
    /// Pronoun.
    Pronoun,
    /// Proper noun.
    ProperNoun,
    /// Punctuation.
    Punctuation,
    /// Subordinating conjunction.
    SubordinatingConjunction,
    /// Symbol.
    Symbol,
    /// Verb.
    Verb,
    /// Other or unknown.
    Other,
}

/// A token with a part-of-speech annotation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaggedToken {
    token: Token,
    part_of_speech: PartOfSpeech,
}

impl TaggedToken {
    /// Creates a tagged token.
    #[must_use]
    pub const fn new(token: Token, part_of_speech: PartOfSpeech) -> Self {
        Self {
            token,
            part_of_speech,
        }
    }

    /// Underlying token.
    #[must_use]
    pub const fn token(&self) -> &Token {
        &self.token
    }

    /// Part-of-speech tag.
    #[must_use]
    pub const fn part_of_speech(&self) -> PartOfSpeech {
        self.part_of_speech
    }
}

/// A dependency relation between two token indexes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dependency {
    head: Option<usize>,
    dependent: usize,
    relation: String,
}

impl Dependency {
    /// Creates a dependency relation. `head == None` marks the root.
    #[must_use]
    pub fn new(head: Option<usize>, dependent: usize, relation: impl Into<String>) -> Self {
        Self {
            head,
            dependent,
            relation: relation.into(),
        }
    }

    /// Head token index, or `None` for the root.
    #[must_use]
    pub const fn head(&self) -> Option<usize> {
        self.head
    }

    /// Dependent token index.
    #[must_use]
    pub const fn dependent(&self) -> usize {
        self.dependent
    }

    /// Dependency relation label.
    #[must_use]
    pub fn relation(&self) -> &str {
        &self.relation
    }
}

/// A dependency tree over tagged tokens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyTree {
    tokens: Vec<TaggedToken>,
    dependencies: Vec<Dependency>,
}

impl DependencyTree {
    /// Creates a dependency tree.
    ///
    /// # Errors
    ///
    /// Returns [`SyntaxError::InvalidDependencyIndex`] when any dependency
    /// points outside the token list.
    pub fn new(
        tokens: Vec<TaggedToken>,
        dependencies: Vec<Dependency>,
    ) -> Result<Self, SyntaxError> {
        for dependency in &dependencies {
            if dependency.dependent >= tokens.len()
                || dependency.head.is_some_and(|head| head >= tokens.len())
            {
                return Err(SyntaxError::InvalidDependencyIndex {
                    token_count: tokens.len(),
                    head: dependency.head,
                    dependent: dependency.dependent,
                });
            }
        }

        Ok(Self {
            tokens,
            dependencies,
        })
    }

    /// Tagged tokens.
    #[must_use]
    pub fn tokens(&self) -> &[TaggedToken] {
        &self.tokens
    }

    /// Dependency relations.
    #[must_use]
    pub fn dependencies(&self) -> &[Dependency] {
        &self.dependencies
    }

    /// Returns the root dependency, when present.
    #[must_use]
    pub fn root(&self) -> Option<&Dependency> {
        self.dependencies
            .iter()
            .find(|dependency| dependency.head.is_none())
    }

    /// Returns dependents headed by the provided token index.
    pub fn dependents_of(&self, head: usize) -> impl Iterator<Item = &Dependency> {
        self.dependencies
            .iter()
            .filter(move |dependency| dependency.head == Some(head))
    }
}

/// Errors produced by syntax helpers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyntaxError {
    /// A dependency references an index outside the token list.
    InvalidDependencyIndex {
        /// Number of tokens in the tree.
        token_count: usize,
        /// Head index that was provided.
        head: Option<usize>,
        /// Dependent index that was provided.
        dependent: usize,
    },
}

impl std::fmt::Display for SyntaxError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidDependencyIndex {
                token_count,
                head,
                dependent,
            } => write!(
                formatter,
                "invalid dependency indexes: token_count={token_count}, head={head:?}, dependent={dependent}"
            ),
        }
    }
}

impl std::error::Error for SyntaxError {}

#[cfg(test)]
mod tests {
    use super::{Dependency, DependencyTree, PartOfSpeech, SyntaxError, TaggedToken};
    use linguistics_core::{TextSpan, Token};

    fn tagged(text: &str, part_of_speech: PartOfSpeech) -> TaggedToken {
        let span = TextSpan::new(0, text.len()).expect("valid span");
        TaggedToken::new(Token::new(text, span), part_of_speech)
    }

    #[test]
    fn dependency_tree_exposes_root_and_dependents() {
        let tokens = vec![
            tagged("cats", PartOfSpeech::Noun),
            tagged("sleep", PartOfSpeech::Verb),
        ];
        let tree = DependencyTree::new(
            tokens,
            vec![
                Dependency::new(Some(1), 0, "nsubj"),
                Dependency::new(None, 1, "root"),
            ],
        )
        .expect("valid tree");

        assert_eq!(tree.root().map(super::Dependency::dependent), Some(1));
        assert_eq!(tree.dependents_of(1).count(), 1);
    }

    #[test]
    fn dependency_tree_rejects_out_of_bounds_indexes() {
        let error = DependencyTree::new(
            vec![tagged("sleep", PartOfSpeech::Verb)],
            vec![Dependency::new(Some(2), 0, "root")],
        )
        .expect_err("invalid head index");

        assert_eq!(
            error,
            SyntaxError::InvalidDependencyIndex {
                token_count: 1,
                head: Some(2),
                dependent: 0,
            }
        );
    }
}
