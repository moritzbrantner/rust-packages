use std::collections::BTreeMap;

use text_analysis_core::{tokenize_words, TextDocument};
use text_analysis_corpus::{term_counts, CorpusOptions, TfIdfCorpus};
use vector_analysis_core::{cosine_similarity, DenseVector};
use vector_analysis_index::{SearchConfig, VectorRecord, VectorSearchIndex};
use video_analysis_core::{DetectError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextEmbeddingConfig {
    pub dimensions: usize,
    pub use_idf: bool,
}

impl Default for TextEmbeddingConfig {
    fn default() -> Self {
        Self {
            dimensions: 128,
            use_idf: false,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct HashedTextEmbedder {
    pub config: TextEmbeddingConfig,
    pub corpus_options: CorpusOptions,
}

impl HashedTextEmbedder {
    pub fn new(config: TextEmbeddingConfig, corpus_options: CorpusOptions) -> Result<Self> {
        if config.dimensions == 0 {
            return Err(invalid_argument(
                "text embedding dimensions must be greater than zero",
            ));
        }
        Ok(Self {
            config,
            corpus_options,
        })
    }

    pub fn embed_text(&self, text: &str) -> Result<DenseVector> {
        self.embed_text_with_corpus(text, None)
    }

    pub fn embed_document(&self, document: &TextDocument<'_>) -> Result<DenseVector> {
        self.embed_text(document.text)
    }

    pub fn embed_text_with_corpus(
        &self,
        text: &str,
        corpus: Option<&TfIdfCorpus>,
    ) -> Result<DenseVector> {
        let counts = term_counts(text, &self.corpus_options);
        self.embed_counts(&counts, corpus)
    }

    pub fn embed_counts(
        &self,
        counts: &BTreeMap<String, usize>,
        corpus: Option<&TfIdfCorpus>,
    ) -> Result<DenseVector> {
        if counts.is_empty() {
            return Err(invalid_argument("text must contain at least one term"));
        }
        let mut values = vec![0.0; self.config.dimensions];
        for (term, count) in counts {
            let hash = stable_hash(term.as_bytes());
            let index = hash as usize % self.config.dimensions;
            let sign = if hash & 1 == 0 { 1.0 } else { -1.0 };
            let idf = if self.config.use_idf {
                corpus
                    .map(|corpus| corpus.inverse_document_frequency(term))
                    .unwrap_or(1.0)
            } else {
                1.0
            };
            values[index] += sign * (*count as f32).ln_1p() * idf;
        }
        DenseVector::new(values)?.l2_normalized()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SemanticMatch {
    pub id: String,
    pub score: f32,
    pub distance: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SemanticTextIndex {
    embedder: HashedTextEmbedder,
    corpus: TfIdfCorpus,
    vectors: VectorSearchIndex,
}

impl SemanticTextIndex {
    pub fn new(embedder: HashedTextEmbedder) -> Self {
        Self {
            corpus: TfIdfCorpus::new(embedder.corpus_options.clone()),
            embedder,
            vectors: VectorSearchIndex::new(),
        }
    }

    pub fn embedder(&self) -> &HashedTextEmbedder {
        &self.embedder
    }

    pub fn corpus(&self) -> &TfIdfCorpus {
        &self.corpus
    }

    pub fn add_document(&mut self, id: impl Into<String>, text: &str) -> Result<()> {
        let id = id.into();
        self.corpus.add_document(id.clone(), text)?;
        if self.embedder.config.use_idf {
            self.rebuild_vectors()
        } else {
            let vector = self
                .embedder
                .embed_text_with_corpus(text, Some(&self.corpus))?;
            self.vectors.add(VectorRecord::new(id, vector))
        }
    }

    pub fn add_text_document(&mut self, document: &TextDocument<'_>) -> Result<()> {
        self.add_document(document.id, document.text)
    }

    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SemanticMatch>> {
        let query = self
            .embedder
            .embed_text_with_corpus(query, Some(&self.corpus))?;
        let results = self.vectors.search(
            &query,
            SearchConfig {
                limit,
                ..SearchConfig::default()
            },
        )?;
        Ok(results
            .into_iter()
            .map(|result| SemanticMatch {
                id: result.id,
                score: result.score,
                distance: result.distance,
            })
            .collect())
    }

    fn rebuild_vectors(&mut self) -> Result<()> {
        let mut vectors = VectorSearchIndex::new();
        for document in self.corpus.documents() {
            let vector = self
                .embedder
                .embed_counts(&document.term_counts, Some(&self.corpus))?;
            vectors.add(VectorRecord::new(document.id.clone(), vector))?;
        }
        self.vectors = vectors;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CooccurrenceConfig {
    pub window_size: usize,
    pub min_term_len: usize,
}

impl Default for CooccurrenceConfig {
    fn default() -> Self {
        Self {
            window_size: 4,
            min_term_len: 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RelatedTerm {
    pub term: String,
    pub count: usize,
    pub score: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CooccurrenceGraph {
    pub config: CooccurrenceConfig,
    term_counts: BTreeMap<String, usize>,
    pair_counts: BTreeMap<(String, String), usize>,
}

impl Default for CooccurrenceGraph {
    fn default() -> Self {
        Self::new(CooccurrenceConfig::default()).expect("default cooccurrence config is valid")
    }
}

impl CooccurrenceGraph {
    pub fn new(config: CooccurrenceConfig) -> Result<Self> {
        if config.window_size == 0 {
            return Err(invalid_argument(
                "cooccurrence window size must be greater than zero",
            ));
        }
        Ok(Self {
            config,
            term_counts: BTreeMap::new(),
            pair_counts: BTreeMap::new(),
        })
    }

    pub fn term_counts(&self) -> &BTreeMap<String, usize> {
        &self.term_counts
    }

    pub fn pair_counts(&self) -> &BTreeMap<(String, String), usize> {
        &self.pair_counts
    }

    pub fn train_text(&mut self, text: &str) {
        let tokens = tokenize_words(text)
            .into_iter()
            .filter(|term| term.chars().count() >= self.config.min_term_len)
            .collect::<Vec<_>>();
        self.train_tokens(&tokens);
    }

    pub fn train_tokens(&mut self, tokens: &[String]) {
        for token in tokens {
            *self.term_counts.entry(token.clone()).or_insert(0) += 1;
        }
        for start in 0..tokens.len() {
            let end = (start + self.config.window_size + 1).min(tokens.len());
            for right in start + 1..end {
                if tokens[start] == tokens[right] {
                    continue;
                }
                let pair = ordered_pair(&tokens[start], &tokens[right]);
                *self.pair_counts.entry(pair).or_insert(0) += 1;
            }
        }
    }

    pub fn related_terms(&self, term: &str, limit: usize) -> Vec<RelatedTerm> {
        let normalized = term.to_lowercase();
        let base_count = self
            .term_counts
            .get(&normalized)
            .copied()
            .unwrap_or(0)
            .max(1) as f32;
        let mut related = Vec::new();
        for ((left, right), count) in &self.pair_counts {
            let other = if left == &normalized {
                Some(right)
            } else if right == &normalized {
                Some(left)
            } else {
                None
            };
            if let Some(other) = other {
                let other_count = self.term_counts.get(other).copied().unwrap_or(0).max(1) as f32;
                related.push(RelatedTerm {
                    term: other.clone(),
                    count: *count,
                    score: *count as f32 / (base_count * other_count).sqrt(),
                });
            }
        }
        related.sort_by(|left, right| {
            right
                .score
                .total_cmp(&left.score)
                .then_with(|| right.count.cmp(&left.count))
                .then_with(|| left.term.cmp(&right.term))
        });
        if limit > 0 {
            related.truncate(limit);
        }
        related
    }
}

pub fn text_similarity(left: &str, right: &str, embedder: &HashedTextEmbedder) -> Result<f32> {
    let left = embedder.embed_text(left)?;
    let right = embedder.embed_text(right)?;
    cosine_similarity(left.as_slice(), right.as_slice())
}

fn ordered_pair(left: &str, right: &str) -> (String, String) {
    if left <= right {
        (left.to_string(), right.to_string())
    } else {
        (right.to_string(), left.to_string())
    }
}

fn stable_hash(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn invalid_argument(message: impl Into<String>) -> DetectError {
    DetectError::InvalidArgument(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embeds_and_compares_text() {
        let embedder = HashedTextEmbedder::default();
        let same_topic =
            text_similarity("rust cargo crates", "cargo rust package", &embedder).unwrap();
        let different_topic =
            text_similarity("rust cargo crates", "oranges bananas", &embedder).unwrap();
        assert!(same_topic > different_topic);
    }

    #[test]
    fn validates_embedding_configuration_and_input() {
        assert!(HashedTextEmbedder::new(
            TextEmbeddingConfig {
                dimensions: 0,
                use_idf: false,
            },
            CorpusOptions::default(),
        )
        .is_err());

        let embedder = HashedTextEmbedder::new(
            TextEmbeddingConfig {
                dimensions: 8,
                use_idf: false,
            },
            CorpusOptions::default(),
        )
        .unwrap();

        assert_eq!(embedder.embed_text("rust rust").unwrap().dimensions(), 8);
        assert!(embedder.embed_text("... !!!").is_err());
    }

    #[test]
    fn searches_semantic_index() {
        let mut index = SemanticTextIndex::new(HashedTextEmbedder::default());
        index
            .add_document("rust", "rust cargo crates ownership")
            .unwrap();
        index
            .add_document("fruit", "oranges bananas apples")
            .unwrap();

        let results = index.search("cargo package", 1).unwrap();
        assert_eq!(results[0].id, "rust");
    }

    #[test]
    fn rejects_duplicate_documents_before_vector_insert() {
        let mut index = SemanticTextIndex::new(HashedTextEmbedder::default());
        index.add_document("rust", "rust cargo crates").unwrap();

        assert!(index.add_document("rust", "new text").is_err());
        assert_eq!(index.corpus().len(), 1);
        assert_eq!(index.search("cargo", 1).unwrap()[0].id, "rust");
    }

    #[test]
    fn idf_weighted_index_rebuilds_vectors_after_insert() {
        let embedder = HashedTextEmbedder::new(
            TextEmbeddingConfig {
                dimensions: 64,
                use_idf: true,
            },
            CorpusOptions::default(),
        )
        .unwrap();
        let mut index = SemanticTextIndex::new(embedder);
        index.add_document("rust", "rust cargo crates").unwrap();
        index.add_document("fruit", "orange banana apple").unwrap();

        let results = index.search("cargo", 2).unwrap();
        assert_eq!(results[0].id, "rust");
    }

    #[test]
    fn finds_related_terms_from_context_windows() {
        let mut graph = CooccurrenceGraph::default();
        graph.train_text("rust cargo build rust cargo test rust ownership");

        let related = graph.related_terms("rust", 2);
        assert_eq!(related[0].term, "cargo");
    }

    #[test]
    fn cooccurrence_graph_filters_short_terms_and_validates_window() {
        assert!(CooccurrenceGraph::new(CooccurrenceConfig {
            window_size: 0,
            min_term_len: 1,
        })
        .is_err());

        let mut graph = CooccurrenceGraph::new(CooccurrenceConfig {
            window_size: 1,
            min_term_len: 4,
        })
        .unwrap();
        graph.train_text("AI rust ML cargo rust");

        assert_eq!(graph.term_counts().get("ai"), None);
        assert_eq!(graph.term_counts().get("ml"), None);
        assert_eq!(
            graph
                .pair_counts()
                .get(&("cargo".to_string(), "rust".to_string())),
            Some(&2)
        );
    }
}
