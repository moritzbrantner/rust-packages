#![doc = include_str!("../README.md")]

use std::collections::{BTreeMap, BTreeSet};

use math_sparse_data::SparseVector;
use serde::{Deserialize, Serialize};
use text_core::{segment_document_id, tokenize_words, AnnotationProvenance, TextDocument};
use text_lexical::{term_counts, CorpusOptions, TfIdfCorpus};
use vector_analysis_core::cosine_similarity;
/// Re-exports the dense vector API.
pub use vector_analysis_core::DenseVector;
use vector_analysis_index::{SearchConfig, VectorRecord, VectorSearchIndex};
use video_analysis_core::{DetectError, Result, TextSegment};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Data type for text embedding config.
pub struct TextEmbeddingConfig {
    /// The dimensions value.
    pub dimensions: usize,
    /// The use idf value.
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
/// Data type for hashed text embedder.
pub struct HashedTextEmbedder {
    /// The config value.
    pub config: TextEmbeddingConfig,
    /// The corpus options value.
    pub corpus_options: CorpusOptions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
/// Variants describing text embedding backend kind.
#[serde(rename_all = "snake_case")]
pub enum TextEmbeddingBackendKind {
    /// The hashed variant.
    Hashed,
    /// The ONNX variant.
    Onnx,
    /// The candle variant.
    Candle,
    /// The cuda oxide variant.
    CudaOxide,
    /// The external variant.
    External,
    /// The custom variant.
    Custom,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Data type for text embedding metadata.
pub struct TextEmbeddingMetadata {
    /// The backend value.
    pub backend: TextEmbeddingBackendKind,
    /// The provenance value.
    pub provenance: AnnotationProvenance,
    /// The model name value.
    pub model_name: Option<String>,
    /// The dimensions value.
    pub dimensions: Option<usize>,
}

impl Default for TextEmbeddingMetadata {
    fn default() -> Self {
        Self {
            backend: TextEmbeddingBackendKind::Custom,
            provenance: AnnotationProvenance::Derived,
            model_name: None,
            dimensions: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// Data type for embedding model info.
pub struct EmbeddingModelInfo {
    /// The model name value.
    pub model_name: String,
    /// The backend value.
    pub backend: TextEmbeddingBackendKind,
    /// The dimensions value.
    pub dimensions: usize,
}

impl Default for EmbeddingModelInfo {
    fn default() -> Self {
        Self {
            model_name: "custom".to_string(),
            backend: TextEmbeddingBackendKind::Custom,
            dimensions: 0,
        }
    }
}

/// Trait for embedding cache hooks.
pub trait EmbeddingCacheHooks {
    /// Returns cache key for text.
    fn cache_key(&self, text: &str) -> Option<String>;
}

/// Trait for text embedding backend implementations.
pub trait TextEmbeddingBackend {
    /// Returns embed text.
    fn embed_text(&self, text: &str) -> Result<DenseVector>;

    /// Returns embed batch.
    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<DenseVector>> {
        texts.iter().map(|text| self.embed_text(text)).collect()
    }

    /// Returns metadata.
    fn metadata(&self) -> TextEmbeddingMetadata {
        TextEmbeddingMetadata::default()
    }

    /// Returns embed document.
    fn embed_document(&self, document: &TextDocument<'_>) -> Result<DenseVector> {
        self.embed_text(document.text)
    }

    /// Returns model info.
    fn model_info(&self) -> EmbeddingModelInfo {
        let metadata = self.metadata();
        EmbeddingModelInfo {
            model_name: metadata.model_name.unwrap_or_else(|| "custom".to_string()),
            backend: metadata.backend,
            dimensions: metadata.dimensions.unwrap_or(0),
        }
    }

    /// Returns cache hooks.
    fn cache_hooks(&self) -> Option<&dyn EmbeddingCacheHooks> {
        None
    }
}

/// Trait alias-style marker for text embedders used by retrieval indexes.
pub trait TextEmbedderBackend: TextEmbeddingBackend {}

impl<T: TextEmbeddingBackend> TextEmbedderBackend for T {}

/// Trait for sentence embedding backend implementations.
pub trait SentenceEmbedder: TextEmbedderBackend {
    /// Returns embed sentences.
    fn embed_sentences(&self, sentences: &[String]) -> Result<Vec<DenseVector>> {
        sentences
            .iter()
            .map(|sentence| self.embed_text(sentence))
            .collect()
    }
}

impl HashedTextEmbedder {
    /// Creates a new value.
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

    /// Returns embed text.
    pub fn embed_text(&self, text: &str) -> Result<DenseVector> {
        self.embed_text_with_corpus(text, None)
    }

    /// Returns embed document.
    pub fn embed_document(&self, document: &TextDocument<'_>) -> Result<DenseVector> {
        self.embed_text(document.text)
    }

    /// Returns embed text with corpus.
    pub fn embed_text_with_corpus(
        &self,
        text: &str,
        corpus: Option<&TfIdfCorpus>,
    ) -> Result<DenseVector> {
        let counts = term_counts(text, &self.corpus_options);
        self.embed_counts(&counts, corpus)
    }

    /// Returns embed counts.
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

    /// Returns embed text sparse.
    pub fn embed_text_sparse(
        &self,
        text: &str,
        corpus: Option<&TfIdfCorpus>,
    ) -> Result<SparseVector> {
        let counts = term_counts(text, &self.corpus_options);
        self.embed_counts_sparse(&counts, corpus)
    }

    /// Returns embed counts sparse.
    pub fn embed_counts_sparse(
        &self,
        counts: &BTreeMap<String, usize>,
        corpus: Option<&TfIdfCorpus>,
    ) -> Result<SparseVector> {
        if counts.is_empty() {
            return Err(invalid_argument("text must contain at least one term"));
        }
        let mut buckets = BTreeMap::<usize, f32>::new();
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
            *buckets.entry(index).or_insert(0.0) += sign * (*count as f32).ln_1p() * idf;
        }
        SparseVector::new(
            self.config.dimensions,
            buckets.keys().copied().collect(),
            buckets.values().copied().collect(),
        )?
        .canonicalized()?
        .normalize_l2()
    }
}

impl TextEmbeddingBackend for HashedTextEmbedder {
    fn embed_text(&self, text: &str) -> Result<DenseVector> {
        HashedTextEmbedder::embed_text(self, text)
    }

    fn embed_document(&self, document: &TextDocument<'_>) -> Result<DenseVector> {
        HashedTextEmbedder::embed_document(self, document)
    }

    fn metadata(&self) -> TextEmbeddingMetadata {
        TextEmbeddingMetadata {
            backend: TextEmbeddingBackendKind::Hashed,
            provenance: AnnotationProvenance::Heuristic,
            model_name: Some("hashed-text-embedder".to_string()),
            dimensions: Some(self.config.dimensions),
        }
    }
}

impl SentenceEmbedder for HashedTextEmbedder {}

#[derive(Debug, Clone, PartialEq)]
/// Data type for semantic match.
pub struct SemanticMatch {
    /// Identifier for this value.
    pub id: String,
    /// Score assigned to this value.
    pub score: f32,
    /// The distance value.
    pub distance: f32,
    /// Metadata associated with this value.
    pub metadata: TextEmbeddingMetadata,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for embedding search index.
pub struct EmbeddingSearchIndex<E> {
    embedder: E,
    vectors: VectorSearchIndex,
}

/// Type alias for semantic index.
pub type SemanticIndex<E> = EmbeddingSearchIndex<E>;

impl<E: TextEmbeddingBackend> EmbeddingSearchIndex<E> {
    /// Creates a new value.
    pub fn new(embedder: E) -> Self {
        Self {
            embedder,
            vectors: VectorSearchIndex::new(),
        }
    }

    /// Returns embedder.
    pub fn embedder(&self) -> &E {
        &self.embedder
    }

    /// Returns embedder mut.
    pub fn embedder_mut(&mut self) -> &mut E {
        &mut self.embedder
    }

    /// Returns backend metadata.
    pub fn backend_metadata(&self) -> TextEmbeddingMetadata {
        self.embedder.metadata()
    }

    /// Adds add document to this value.
    pub fn add_document(&mut self, id: impl Into<String>, text: &str) -> Result<()> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(invalid_argument("document id must not be empty"));
        }
        if self.vectors.records().iter().any(|record| record.id == id) {
            return Err(invalid_argument(format!(
                "document id `{id}` already exists"
            )));
        }
        let vector = self.embedder.embed_text(text)?;
        self.vectors.add(VectorRecord::new(id, vector))
    }

    /// Adds add text document to this value.
    pub fn add_text_document(&mut self, document: &TextDocument<'_>) -> Result<()> {
        let id = document.id.to_string();
        if id.trim().is_empty() {
            return Err(invalid_argument("document id must not be empty"));
        }
        if self.vectors.records().iter().any(|record| record.id == id) {
            return Err(invalid_argument(format!(
                "document id `{id}` already exists"
            )));
        }
        let vector = self.embedder.embed_document(document)?;
        self.vectors.add(VectorRecord::new(id, vector))
    }

    /// Adds add text segment to this value.
    pub fn add_text_segment(&mut self, stream_id: &str, segment: &TextSegment<'_>) -> Result<()> {
        validate_stream_id(stream_id)?;
        self.add_document(
            segment_document_id(stream_id, segment.segment_index),
            segment.text,
        )
    }

    /// Returns search.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SemanticMatch>> {
        let metadata = self.embedder.metadata();
        let query = self.embedder.embed_text(query)?;
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
                metadata: metadata.clone(),
            })
            .collect())
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for semantic text index.
pub struct SemanticTextIndex {
    embedder: HashedTextEmbedder,
    corpus: TfIdfCorpus,
    vectors: VectorSearchIndex,
}

impl SemanticTextIndex {
    /// Creates a new value.
    pub fn new(embedder: HashedTextEmbedder) -> Self {
        Self {
            corpus: TfIdfCorpus::new(embedder.corpus_options.clone()),
            embedder,
            vectors: VectorSearchIndex::new(),
        }
    }

    /// Builds this value from documents.
    pub fn from_documents<'a, I>(embedder: HashedTextEmbedder, documents: I) -> Result<Self>
    where
        I: IntoIterator<Item = TextDocument<'a>>,
    {
        let mut index = Self::new(embedder);
        index.add_documents(documents)?;
        Ok(index)
    }

    /// Returns embedder.
    pub fn embedder(&self) -> &HashedTextEmbedder {
        &self.embedder
    }

    /// Returns corpus.
    pub fn corpus(&self) -> &TfIdfCorpus {
        &self.corpus
    }

    /// Returns backend metadata.
    pub fn backend_metadata(&self) -> TextEmbeddingMetadata {
        self.embedder.metadata()
    }

    /// Adds add document to this value.
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

    /// Adds add text document to this value.
    pub fn add_text_document(&mut self, document: &TextDocument<'_>) -> Result<()> {
        self.add_document(document.id, document.text)
    }

    /// Adds add documents to this value.
    pub fn add_documents<'a, I>(&mut self, documents: I) -> Result<()>
    where
        I: IntoIterator<Item = TextDocument<'a>>,
    {
        let documents = documents
            .into_iter()
            .map(|document| (document.id.to_string(), document.text.to_string()))
            .collect::<Vec<_>>();
        self.validate_new_document_ids(documents.iter().map(|(id, _)| id.as_str()))?;

        for (id, text) in &documents {
            self.corpus.add_document(id.clone(), text)?;
        }
        if self.embedder.config.use_idf {
            self.rebuild_vectors()
        } else {
            for (id, text) in documents {
                let vector = self
                    .embedder
                    .embed_text_with_corpus(&text, Some(&self.corpus))?;
                self.vectors.add(VectorRecord::new(id, vector))?;
            }
            Ok(())
        }
    }

    /// Adds add text segment to this value.
    pub fn add_text_segment(&mut self, stream_id: &str, segment: &TextSegment<'_>) -> Result<()> {
        validate_stream_id(stream_id)?;
        self.add_document(
            segment_document_id(stream_id, segment.segment_index),
            segment.text,
        )
    }

    /// Returns search.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SemanticMatch>> {
        let metadata = self.embedder.metadata();
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
                metadata: metadata.clone(),
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

    fn validate_new_document_ids<'a>(&self, ids: impl IntoIterator<Item = &'a str>) -> Result<()> {
        let mut seen = BTreeSet::new();
        for id in ids {
            if id.trim().is_empty() {
                return Err(invalid_argument("document id must not be empty"));
            }
            if self.corpus.document(id).is_some() || !seen.insert(id.to_string()) {
                return Err(invalid_argument(format!(
                    "document id `{id}` already exists"
                )));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Data type for cooccurrence config.
pub struct CooccurrenceConfig {
    /// The window size value.
    pub window_size: usize,
    /// The min term len value.
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
/// Data type for related term.
pub struct RelatedTerm {
    /// The term value.
    pub term: String,
    /// Number of items represented by this value.
    pub count: usize,
    /// Score assigned to this value.
    pub score: f32,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for cooccurrence graph.
pub struct CooccurrenceGraph {
    /// The config value.
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
    /// Creates a new value.
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

    /// Returns term counts.
    pub fn term_counts(&self) -> &BTreeMap<String, usize> {
        &self.term_counts
    }

    /// Returns pair counts.
    pub fn pair_counts(&self) -> &BTreeMap<(String, String), usize> {
        &self.pair_counts
    }

    /// Returns train text.
    pub fn train_text(&mut self, text: &str) {
        let tokens = tokenize_words(text)
            .into_iter()
            .filter(|term| term.chars().count() >= self.config.min_term_len)
            .collect::<Vec<_>>();
        self.train_tokens(&tokens);
    }

    /// Returns train tokens.
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

    /// Returns related terms.
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

/// Returns text similarity.
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

fn validate_stream_id(stream_id: &str) -> Result<()> {
    if stream_id.trim().is_empty() {
        return Err(invalid_argument("stream id must not be empty"));
    }
    Ok(())
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
    fn idf_weighted_batch_ingestion_rebuilds_vectors_once_after_corpus_update() {
        let embedder = HashedTextEmbedder::new(
            TextEmbeddingConfig {
                dimensions: 64,
                use_idf: true,
            },
            CorpusOptions::default(),
        )
        .unwrap();
        let documents = [
            TextDocument::new("rust", "rust cargo crates"),
            TextDocument::new("fruit", "orange banana apple"),
            TextDocument::new("search", "semantic retrieval search"),
        ];

        let index = SemanticTextIndex::from_documents(embedder, documents).unwrap();

        assert_eq!(index.corpus().len(), 3);
        assert_eq!(index.vectors.records().len(), 3);
        assert_eq!(index.search("cargo", 1).unwrap()[0].id, "rust");
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

    #[derive(Debug, Clone)]
    struct TinyEmbedder;

    impl TextEmbeddingBackend for TinyEmbedder {
        fn embed_text(&self, text: &str) -> Result<DenseVector> {
            let has_rust = text.contains("rust") || text.contains("cargo");
            DenseVector::new(if has_rust { [1.0, 0.0] } else { [0.0, 1.0] })
        }

        fn metadata(&self) -> TextEmbeddingMetadata {
            TextEmbeddingMetadata {
                backend: TextEmbeddingBackendKind::Custom,
                provenance: AnnotationProvenance::External,
                model_name: Some("tiny".to_string()),
                dimensions: Some(2),
            }
        }
    }

    #[test]
    fn generic_embedding_index_supports_trait_backends() {
        let mut index = EmbeddingSearchIndex::new(TinyEmbedder);
        index.add_document("rust", "rust cargo").unwrap();
        index.add_document("fruit", "orange banana").unwrap();
        assert!(index.add_document("rust", "duplicate").is_err());

        let results = index.search("cargo", 1).unwrap();
        assert_eq!(results[0].id, "rust");
    }

    #[test]
    fn generic_embedding_index_adds_text_segments_with_generated_ids() {
        let mut index = EmbeddingSearchIndex::new(TinyEmbedder);
        let rust_segment = TextSegment {
            segment_index: 4,
            timestamp: None,
            text: "rust cargo segment",
            language: Some("en"),
            is_final: true,
        };
        let fruit_segment = TextSegment {
            segment_index: 5,
            timestamp: None,
            text: "orange banana segment",
            language: Some("en"),
            is_final: true,
        };
        index.add_text_segment("subs", &rust_segment).unwrap();
        index.add_text_segment("subs", &fruit_segment).unwrap();

        let results = index.search("cargo", 1).unwrap();
        assert_eq!(results[0].id, "subs:4");
        assert!(matches!(
            index.add_text_segment("subs", &rust_segment),
            Err(DetectError::InvalidArgument(message))
                if message.contains("document id `subs:4` already exists")
        ));
        assert!(matches!(
            index.add_text_segment(" ", &rust_segment),
            Err(DetectError::InvalidArgument(message)) if message == "stream id must not be empty"
        ));
    }

    #[test]
    fn semantic_index_adds_text_segments_with_generated_ids() {
        let mut index = SemanticTextIndex::new(HashedTextEmbedder::default());
        let first = TextSegment {
            segment_index: 0,
            timestamp: None,
            text: "rust cargo crates",
            language: Some("en"),
            is_final: true,
        };
        let second = TextSegment {
            segment_index: 1,
            timestamp: None,
            text: "banana citrus apple",
            language: Some("en"),
            is_final: true,
        };
        index.add_text_segment("subs", &first).unwrap();
        index.add_text_segment("subs", &second).unwrap();

        assert_eq!(index.corpus().documents()[0].id, "subs:0");
        assert_eq!(index.corpus().documents()[1].id, "subs:1");
        assert_eq!(index.search("cargo", 1).unwrap()[0].id, "subs:0");
        assert!(matches!(
            index.add_text_segment("", &first),
            Err(DetectError::InvalidArgument(message)) if message == "stream id must not be empty"
        ));
    }

    #[test]
    fn stream_segment_document_constructor_matches_corpus_and_semantic_ids() {
        let segment = TextSegment {
            segment_index: 9,
            timestamp: None,
            text: "rust cargo segment",
            language: Some("en"),
            is_final: true,
        };
        let document = TextDocument::from_stream_segment("subs", &segment);
        let mut corpus = TfIdfCorpus::default();
        corpus.add_text_segment("subs", &segment).unwrap();
        let mut semantic = SemanticTextIndex::new(HashedTextEmbedder::default());
        semantic.add_text_segment("subs", &segment).unwrap();

        assert_eq!(document.id, "subs:9");
        assert_eq!(corpus.documents()[0].id, document.id);
        assert_eq!(semantic.corpus().documents()[0].id, document.id);
    }

    #[test]
    fn hashed_embedder_implements_embedding_backend() {
        fn embed_with_trait(backend: &dyn TextEmbeddingBackend, text: &str) -> Result<DenseVector> {
            backend.embed_text(text)
        }

        let embedder = HashedTextEmbedder::default();
        let direct = embedder.embed_text("rust cargo").unwrap();
        let via_trait = embed_with_trait(&embedder, "rust cargo").unwrap();
        assert_eq!(direct, via_trait);
    }

    #[test]
    fn semantic_matches_include_backend_metadata() {
        let mut index = EmbeddingSearchIndex::new(TinyEmbedder);
        index.add_document("rust", "rust cargo").unwrap();

        let results = index.search("cargo", 1).unwrap();
        assert_eq!(
            results[0].metadata.backend,
            TextEmbeddingBackendKind::Custom
        );
        assert_eq!(
            results[0].metadata.provenance,
            AnnotationProvenance::External
        );
        assert_eq!(results[0].metadata.model_name.as_deref(), Some("tiny"));

        let hashed = HashedTextEmbedder::default().metadata();
        assert_eq!(hashed.backend, TextEmbeddingBackendKind::Hashed);
        assert_eq!(hashed.provenance, AnnotationProvenance::Heuristic);
        assert_eq!(
            hashed.dimensions,
            Some(TextEmbeddingConfig::default().dimensions)
        );
    }

    #[test]
    fn hashed_embedder_can_emit_sparse_vectors() {
        let embedder = HashedTextEmbedder::default();
        let sparse = embedder
            .embed_text_sparse("rust cargo crates", None)
            .unwrap();
        assert_eq!(sparse.dimensions(), embedder.config.dimensions);
        assert!(sparse.nnz() > 0);
    }

    #[test]
    fn hashed_embedder_is_stable_and_normalized() {
        let embedder = HashedTextEmbedder::default();
        let left = embedder.embed_text("rust cargo crates").unwrap();
        let right = embedder.embed_text("rust cargo crates").unwrap();

        assert_eq!(left, right);
        assert!((vector_analysis_core::l2_norm(left.as_slice()).unwrap() - 1.0).abs() < 1.0e-6);
    }
}
