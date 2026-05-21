#![doc = include_str!("../README.md")]

use std::collections::{BTreeMap, BTreeSet};

use math_sparse_data::{CooMatrix, CsrMatrix, SparseVector};
use serde::{Deserialize, Serialize};
use text_core::{segment_document_id, tokenize, TextDocument, TextProcessingOptions, TokenKind};
use video_analysis_core::{DetectError, Result, TextSegment};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// Data type for corpus options.
pub struct CorpusOptions {
    /// The processing value.
    pub processing: TextProcessingOptions,
    /// The min term len value.
    pub min_term_len: usize,
    /// The stop words value.
    pub stop_words: BTreeSet<String>,
    /// The max terms per document value.
    pub max_terms_per_document: Option<usize>,
}

impl Default for CorpusOptions {
    fn default() -> Self {
        Self {
            processing: TextProcessingOptions::default(),
            min_term_len: 1,
            stop_words: BTreeSet::new(),
            max_terms_per_document: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Data type for indexed document.
pub struct IndexedDocument {
    /// Identifier for this value.
    pub id: String,
    /// The term counts value.
    pub term_counts: BTreeMap<String, usize>,
    /// The total terms value.
    pub total_terms: usize,
}

impl IndexedDocument {
    /// Returns unique terms.
    pub fn unique_terms(&self) -> usize {
        self.term_counts.len()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Data type for corpus stats.
pub struct CorpusStats {
    /// The documents value.
    pub documents: usize,
    /// The total terms value.
    pub total_terms: usize,
    /// The unique terms value.
    pub unique_terms: usize,
    /// The average terms per document value.
    pub average_terms_per_document: f32,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for corpus term stats.
pub struct CorpusTermStats {
    /// The term value.
    pub term: String,
    /// The collection count value.
    pub collection_count: usize,
    /// The document count value.
    pub document_count: usize,
    /// The collection frequency value.
    pub collection_frequency: f32,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for tf idf term.
pub struct TfIdfTerm {
    /// The term value.
    pub term: String,
    /// Number of items represented by this value.
    pub count: usize,
    /// The term frequency value.
    pub term_frequency: f32,
    /// The inverse document frequency value.
    pub inverse_document_frequency: f32,
    /// Score assigned to this value.
    pub score: f32,
    /// The document count value.
    pub document_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for document search result.
pub struct DocumentSearchResult {
    /// Identifier for this value.
    pub id: String,
    /// Score assigned to this value.
    pub score: f32,
    /// The matched terms value.
    pub matched_terms: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Data type for bm25 options.
pub struct Bm25Options {
    /// The k1 value.
    pub k1: f32,
    /// The b value.
    pub b: f32,
    /// The min term len value.
    pub min_term_len: usize,
    /// The stop words value.
    pub stop_words: BTreeSet<String>,
}

impl Default for Bm25Options {
    fn default() -> Self {
        Self {
            k1: 1.2,
            b: 0.75,
            min_term_len: 1,
            stop_words: BTreeSet::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for bm25 search result.
pub struct Bm25SearchResult {
    /// Identifier for this value.
    pub id: String,
    /// Score assigned to this value.
    pub score: f32,
    /// The matched terms value.
    pub matched_terms: usize,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for sparse term matrix.
pub struct SparseTermMatrix {
    /// The vocabulary value.
    pub vocabulary: Vec<String>,
    /// The matrix value.
    pub matrix: CsrMatrix,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for tf idf corpus.
pub struct TfIdfCorpus {
    options: CorpusOptions,
    documents: Vec<IndexedDocument>,
    document_frequency: BTreeMap<String, usize>,
    collection_counts: BTreeMap<String, usize>,
    total_terms: usize,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for bm25 corpus.
pub struct Bm25Corpus {
    options: Bm25Options,
    documents: Vec<IndexedDocument>,
    document_frequency: BTreeMap<String, usize>,
    total_terms: usize,
}

impl Default for Bm25Corpus {
    fn default() -> Self {
        Self::new(Bm25Options::default())
    }
}

impl Bm25Corpus {
    /// Creates a new value.
    pub fn new(options: Bm25Options) -> Self {
        Self {
            options,
            documents: Vec::new(),
            document_frequency: BTreeMap::new(),
            total_terms: 0,
        }
    }

    /// Builds this value from texts.
    pub fn from_texts<I, S>(texts: I, options: Bm25Options) -> Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut corpus = Self::new(options);
        for (index, text) in texts.into_iter().enumerate() {
            corpus.add_document(format!("doc-{index}"), text.as_ref())?;
        }
        Ok(corpus)
    }

    /// Builds this value from documents.
    pub fn from_documents<'a, I>(documents: I, options: Bm25Options) -> Result<Self>
    where
        I: IntoIterator<Item = TextDocument<'a>>,
    {
        let mut corpus = Self::new(options);
        for document in documents {
            corpus.add_text_document(&document)?;
        }
        Ok(corpus)
    }

    /// Builds this value from text segments.
    pub fn from_text_segments<'a, I>(
        stream_id: &str,
        segments: I,
        options: Bm25Options,
    ) -> Result<Self>
    where
        I: IntoIterator<Item = TextSegment<'a>>,
    {
        validate_stream_id(stream_id)?;
        let mut corpus = Self::new(options);
        for segment in segments {
            corpus.add_text_segment(stream_id, &segment)?;
        }
        Ok(corpus)
    }

    /// Returns options.
    pub fn options(&self) -> &Bm25Options {
        &self.options
    }

    /// Returns len.
    pub fn len(&self) -> usize {
        self.documents.len()
    }

    /// Returns whether is empty.
    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }

    /// Returns documents.
    pub fn documents(&self) -> &[IndexedDocument] {
        &self.documents
    }

    /// Adds add text document to this value.
    pub fn add_text_document(&mut self, document: &TextDocument<'_>) -> Result<usize> {
        self.add_document(document.id, document.text)
    }

    /// Adds add text segment to this value.
    pub fn add_text_segment(
        &mut self,
        stream_id: &str,
        segment: &TextSegment<'_>,
    ) -> Result<usize> {
        validate_stream_id(stream_id)?;
        self.add_document(
            segment_document_id(stream_id, segment.segment_index),
            segment.text,
        )
    }

    /// Adds add document to this value.
    pub fn add_document(&mut self, id: impl Into<String>, text: &str) -> Result<usize> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(invalid_argument("document id must not be empty"));
        }
        if self.documents.iter().any(|document| document.id == id) {
            return Err(invalid_argument(format!(
                "document id `{id}` already exists"
            )));
        }
        if self.options.k1 <= 0.0 || !self.options.k1.is_finite() {
            return Err(invalid_argument(
                "BM25 k1 must be finite and greater than zero",
            ));
        }
        if !(0.0..=1.0).contains(&self.options.b) || !self.options.b.is_finite() {
            return Err(invalid_argument(
                "BM25 b must be finite and between 0 and 1",
            ));
        }

        let term_counts = bm25_term_counts(text, &self.options);
        let indexed = IndexedDocument {
            id,
            total_terms: term_counts.values().sum(),
            term_counts,
        };
        for term in indexed.term_counts.keys() {
            *self.document_frequency.entry(term.clone()).or_insert(0) += 1;
        }
        self.total_terms += indexed.total_terms;
        self.documents.push(indexed);
        Ok(self.documents.len() - 1)
    }

    /// Returns search.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<Bm25SearchResult>> {
        if limit == 0 {
            return Err(invalid_argument("search limit must be greater than zero"));
        }
        if self.documents.is_empty() {
            return Ok(Vec::new());
        }
        let query_counts = bm25_term_counts(query, &self.options);
        if query_counts.is_empty() {
            return Ok(Vec::new());
        }

        let average_document_len = self.total_terms as f32 / self.documents.len() as f32;
        let mut results = Vec::new();
        for document in &self.documents {
            if document.total_terms == 0 {
                continue;
            }
            let mut score = 0.0_f32;
            let mut matched_terms = 0;
            for (term, query_count) in &query_counts {
                let Some(term_count) = document.term_counts.get(term).copied() else {
                    continue;
                };
                matched_terms += 1;
                let idf = self.inverse_document_frequency(term);
                let tf = term_count as f32;
                let length_factor = 1.0 - self.options.b
                    + self.options.b * document.total_terms as f32
                        / average_document_len.max(f32::EPSILON);
                let numerator = tf * (self.options.k1 + 1.0);
                let denominator = tf + self.options.k1 * length_factor;
                score += *query_count as f32 * idf * numerator / denominator;
            }
            if matched_terms > 0 {
                results.push(Bm25SearchResult {
                    id: document.id.clone(),
                    score,
                    matched_terms,
                });
            }
        }
        results.sort_by(|left, right| {
            right
                .score
                .total_cmp(&left.score)
                .then_with(|| right.matched_terms.cmp(&left.matched_terms))
                .then_with(|| left.id.cmp(&right.id))
        });
        results.truncate(limit);
        Ok(results)
    }

    /// Returns inverse document frequency.
    pub fn inverse_document_frequency(&self, term: &str) -> f32 {
        let documents = self.documents.len() as f32;
        let document_count = self.document_frequency.get(term).copied().unwrap_or(0) as f32;
        (1.0 + (documents - document_count + 0.5) / (document_count + 0.5)).ln()
    }

    /// Returns sparse term matrix.
    pub fn sparse_term_matrix(&self) -> Result<SparseTermMatrix> {
        build_sparse_term_matrix(&self.documents)
    }
}

impl Default for TfIdfCorpus {
    fn default() -> Self {
        Self::new(CorpusOptions::default())
    }
}

impl TfIdfCorpus {
    /// Creates a new value.
    pub fn new(options: CorpusOptions) -> Self {
        Self {
            options,
            documents: Vec::new(),
            document_frequency: BTreeMap::new(),
            collection_counts: BTreeMap::new(),
            total_terms: 0,
        }
    }

    /// Builds this value from texts.
    pub fn from_texts<I, S>(texts: I, options: CorpusOptions) -> Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut corpus = Self::new(options);
        for (index, text) in texts.into_iter().enumerate() {
            corpus.add_document(format!("doc-{index}"), text.as_ref())?;
        }
        Ok(corpus)
    }

    /// Builds this value from documents.
    pub fn from_documents<'a, I>(documents: I, options: CorpusOptions) -> Result<Self>
    where
        I: IntoIterator<Item = TextDocument<'a>>,
    {
        let mut corpus = Self::new(options);
        for document in documents {
            corpus.add_text_document(&document)?;
        }
        Ok(corpus)
    }

    /// Builds this value from text segments.
    pub fn from_text_segments<'a, I>(
        stream_id: &str,
        segments: I,
        options: CorpusOptions,
    ) -> Result<Self>
    where
        I: IntoIterator<Item = TextSegment<'a>>,
    {
        validate_stream_id(stream_id)?;
        let mut corpus = Self::new(options);
        for segment in segments {
            corpus.add_text_segment(stream_id, &segment)?;
        }
        Ok(corpus)
    }

    /// Returns options.
    pub fn options(&self) -> &CorpusOptions {
        &self.options
    }

    /// Returns len.
    pub fn len(&self) -> usize {
        self.documents.len()
    }

    /// Returns whether is empty.
    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }

    /// Returns documents.
    pub fn documents(&self) -> &[IndexedDocument] {
        &self.documents
    }

    /// Returns document.
    pub fn document(&self, id: &str) -> Option<&IndexedDocument> {
        self.documents.iter().find(|document| document.id == id)
    }

    /// Adds add text document to this value.
    pub fn add_text_document(&mut self, document: &TextDocument<'_>) -> Result<usize> {
        self.add_document(document.id, document.text)
    }

    /// Adds add text segment to this value.
    pub fn add_text_segment(
        &mut self,
        stream_id: &str,
        segment: &TextSegment<'_>,
    ) -> Result<usize> {
        validate_stream_id(stream_id)?;
        self.add_document(
            segment_document_id(stream_id, segment.segment_index),
            segment.text,
        )
    }

    /// Adds add document to this value.
    pub fn add_document(&mut self, id: impl Into<String>, text: &str) -> Result<usize> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(invalid_argument("document id must not be empty"));
        }
        if self.documents.iter().any(|document| document.id == id) {
            return Err(invalid_argument(format!(
                "document id `{id}` already exists"
            )));
        }

        let term_counts = term_counts(text, &self.options);
        let mut indexed = IndexedDocument {
            id,
            total_terms: term_counts.values().sum(),
            term_counts,
        };
        if let Some(max_terms) = self.options.max_terms_per_document {
            indexed.term_counts = retain_top_counts(&indexed.term_counts, max_terms);
            indexed.total_terms = indexed.term_counts.values().sum();
        }

        for (term, count) in &indexed.term_counts {
            *self.collection_counts.entry(term.clone()).or_insert(0) += count;
            *self.document_frequency.entry(term.clone()).or_insert(0) += 1;
        }
        self.total_terms += indexed.total_terms;
        self.documents.push(indexed);
        Ok(self.documents.len() - 1)
    }

    /// Returns stats.
    pub fn stats(&self) -> CorpusStats {
        CorpusStats {
            documents: self.documents.len(),
            total_terms: self.total_terms,
            unique_terms: self.collection_counts.len(),
            average_terms_per_document: if self.documents.is_empty() {
                0.0
            } else {
                self.total_terms as f32 / self.documents.len() as f32
            },
        }
    }

    /// Returns sparse term matrix.
    pub fn sparse_term_matrix(&self) -> Result<SparseTermMatrix> {
        build_sparse_term_matrix(&self.documents)
    }

    /// Returns term stats.
    pub fn term_stats(&self, limit: usize) -> Vec<CorpusTermStats> {
        let total = self.total_terms.max(1) as f32;
        let mut terms = self
            .collection_counts
            .iter()
            .map(|(term, count)| CorpusTermStats {
                term: term.clone(),
                collection_count: *count,
                document_count: self.document_count(term),
                collection_frequency: *count as f32 / total,
            })
            .collect::<Vec<_>>();
        terms.sort_by(|left, right| {
            right
                .collection_count
                .cmp(&left.collection_count)
                .then_with(|| right.document_count.cmp(&left.document_count))
                .then_with(|| left.term.cmp(&right.term))
        });
        truncate_limit(&mut terms, limit);
        terms
    }

    /// Returns document tfidf.
    pub fn document_tfidf(&self, id: &str, limit: usize) -> Result<Vec<TfIdfTerm>> {
        let document = self
            .document(id)
            .ok_or_else(|| invalid_argument(format!("document `{id}` was not indexed")))?;
        Ok(self.tfidf_for_counts(&document.term_counts, document.total_terms, limit))
    }

    /// Returns text tfidf.
    pub fn text_tfidf(&self, text: &str, limit: usize) -> Vec<TfIdfTerm> {
        let counts = term_counts(text, &self.options);
        let total_terms = counts.values().sum();
        self.tfidf_for_counts(&counts, total_terms, limit)
    }

    /// Returns search.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<DocumentSearchResult>> {
        if limit == 0 {
            return Err(invalid_argument("search limit must be greater than zero"));
        }
        if self.documents.is_empty() {
            return Ok(Vec::new());
        }
        let query_counts = term_counts(query, &self.options);
        if query_counts.is_empty() {
            return Ok(Vec::new());
        }

        let query_weights = self.weight_map(&query_counts, query_counts.values().sum());
        let query_norm = l2_norm(query_weights.values().copied());
        if query_norm <= f32::EPSILON {
            return Ok(Vec::new());
        }

        let mut results = Vec::new();
        for document in &self.documents {
            let document_weights = self.weight_map(&document.term_counts, document.total_terms);
            let document_norm = l2_norm(document_weights.values().copied());
            if document_norm <= f32::EPSILON {
                continue;
            }

            let mut dot = 0.0;
            let mut matched_terms = 0;
            for (term, query_weight) in &query_weights {
                if let Some(document_weight) = document_weights.get(term) {
                    dot += query_weight * document_weight;
                    matched_terms += 1;
                }
            }
            if matched_terms == 0 {
                continue;
            }
            results.push(DocumentSearchResult {
                id: document.id.clone(),
                score: dot / (query_norm * document_norm),
                matched_terms,
            });
        }

        results.sort_by(|left, right| {
            right
                .score
                .total_cmp(&left.score)
                .then_with(|| right.matched_terms.cmp(&left.matched_terms))
                .then_with(|| left.id.cmp(&right.id))
        });
        results.truncate(limit);
        Ok(results)
    }

    /// Returns document count.
    pub fn document_count(&self, term: &str) -> usize {
        self.document_frequency.get(term).copied().unwrap_or(0)
    }

    /// Returns inverse document frequency.
    pub fn inverse_document_frequency(&self, term: &str) -> f32 {
        let documents = self.documents.len() as f32;
        let document_count = self.document_count(term) as f32;
        ((documents + 1.0) / (document_count + 1.0)).ln() + 1.0
    }

    fn tfidf_for_counts(
        &self,
        counts: &BTreeMap<String, usize>,
        total_terms: usize,
        limit: usize,
    ) -> Vec<TfIdfTerm> {
        let denominator = total_terms.max(1) as f32;
        let mut terms = counts
            .iter()
            .map(|(term, count)| {
                let term_frequency = *count as f32 / denominator;
                let inverse_document_frequency = self.inverse_document_frequency(term);
                TfIdfTerm {
                    term: term.clone(),
                    count: *count,
                    term_frequency,
                    inverse_document_frequency,
                    score: term_frequency * inverse_document_frequency,
                    document_count: self.document_count(term),
                }
            })
            .collect::<Vec<_>>();
        terms.sort_by(|left, right| {
            right
                .score
                .total_cmp(&left.score)
                .then_with(|| right.count.cmp(&left.count))
                .then_with(|| left.term.cmp(&right.term))
        });
        truncate_limit(&mut terms, limit);
        terms
    }

    fn weight_map(
        &self,
        counts: &BTreeMap<String, usize>,
        total_terms: usize,
    ) -> BTreeMap<String, f32> {
        self.tfidf_for_counts(counts, total_terms, 0)
            .into_iter()
            .map(|term| (term.term, term.score))
            .collect()
    }
}

/// Returns terms.
pub fn terms(text: &str, options: &CorpusOptions) -> Vec<String> {
    tokenize(text, &options.processing)
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
        .filter(|term| term.chars().count() >= options.min_term_len)
        .filter(|term| !options.stop_words.contains(term))
        .collect()
}

/// Returns term counts.
pub fn term_counts(text: &str, options: &CorpusOptions) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for term in terms(text, options) {
        *counts.entry(term).or_insert(0) += 1;
    }
    counts
}

/// Returns sparse term vector.
pub fn sparse_term_vector(
    counts: &BTreeMap<String, usize>,
    vocabulary: &[String],
) -> Result<SparseVector> {
    let index_by_term = vocabulary
        .iter()
        .enumerate()
        .map(|(index, term)| (term.as_str(), index))
        .collect::<BTreeMap<_, _>>();
    let mut indices = Vec::new();
    let mut values = Vec::new();
    for (term, count) in counts {
        if let Some(index) = index_by_term.get(term.as_str()) {
            indices.push(*index);
            values.push(*count as f32);
        }
    }
    SparseVector::new(vocabulary.len().max(1), indices, values)
}

fn bm25_term_counts(text: &str, options: &Bm25Options) -> BTreeMap<String, usize> {
    term_counts(
        text,
        &CorpusOptions {
            min_term_len: options.min_term_len,
            stop_words: options.stop_words.clone(),
            ..CorpusOptions::default()
        },
    )
}

fn retain_top_counts(counts: &BTreeMap<String, usize>, limit: usize) -> BTreeMap<String, usize> {
    let mut ranked = counts.iter().collect::<Vec<_>>();
    ranked.sort_by(|(left_term, left_count), (right_term, right_count)| {
        right_count
            .cmp(left_count)
            .then_with(|| left_term.cmp(right_term))
    });
    ranked
        .into_iter()
        .take(limit)
        .map(|(term, count)| (term.clone(), *count))
        .collect()
}

fn truncate_limit<T>(values: &mut Vec<T>, limit: usize) {
    if limit > 0 {
        values.truncate(limit);
    }
}

fn build_sparse_term_matrix(documents: &[IndexedDocument]) -> Result<SparseTermMatrix> {
    let vocabulary = documents
        .iter()
        .flat_map(|document| document.term_counts.keys().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let index_by_term = vocabulary
        .iter()
        .enumerate()
        .map(|(index, term)| (term.as_str(), index))
        .collect::<BTreeMap<_, _>>();
    let mut entries = Vec::new();
    for (row, document) in documents.iter().enumerate() {
        for (term, count) in &document.term_counts {
            if let Some(col) = index_by_term.get(term.as_str()) {
                entries.push((row, *col, *count as f32));
            }
        }
    }
    let matrix =
        CooMatrix::new(documents.len().max(1), vocabulary.len().max(1), entries)?.to_csr()?;
    Ok(SparseTermMatrix { vocabulary, matrix })
}

fn l2_norm(values: impl IntoIterator<Item = f32>) -> f32 {
    values
        .into_iter()
        .map(|value| value * value)
        .sum::<f32>()
        .sqrt()
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
    fn indexes_corpus_statistics() {
        let mut corpus = TfIdfCorpus::default();
        corpus.add_document("a", "rust text text").unwrap();
        corpus.add_document("b", "video text").unwrap();

        let stats = corpus.stats();
        assert_eq!(stats.documents, 2);
        assert_eq!(stats.total_terms, 5);
        assert_eq!(stats.unique_terms, 3);
        assert_eq!(corpus.document_count("text"), 2);
    }

    #[test]
    fn filters_terms_with_processing_options() {
        let mut options = CorpusOptions {
            min_term_len: 4,
            ..CorpusOptions::default()
        };
        options.stop_words.insert("this".to_string());

        let terms = terms("This Rust #AI 2026 contact@example.com", &options);

        assert_eq!(terms, vec!["rust", "2026", "contact@example.com"]);
    }

    #[test]
    fn retains_most_frequent_terms_after_counting() {
        let mut corpus = TfIdfCorpus::new(CorpusOptions {
            max_terms_per_document: Some(2),
            ..CorpusOptions::default()
        });

        corpus
            .add_document("doc", "alpha beta beta gamma gamma gamma")
            .unwrap();
        let document = corpus.document("doc").unwrap();

        assert_eq!(document.total_terms, 5);
        assert_eq!(document.term_counts.get("gamma"), Some(&3));
        assert_eq!(document.term_counts.get("beta"), Some(&2));
        assert_eq!(document.term_counts.get("alpha"), None);
    }

    #[test]
    fn rejects_invalid_document_and_search_inputs() {
        let mut corpus = TfIdfCorpus::default();

        assert!(corpus.add_document("  ", "text").is_err());
        corpus.add_document("doc", "rust cargo").unwrap();
        assert!(corpus.add_document("doc", "duplicate").is_err());
        assert!(corpus.search("rust", 0).is_err());
        assert!(corpus.document_tfidf("missing", 10).is_err());
    }

    #[test]
    fn ranks_tfidf_terms() {
        let mut corpus = TfIdfCorpus::default();
        corpus.add_document("a", "rust rust text").unwrap();
        corpus.add_document("b", "text video").unwrap();

        let terms = corpus.document_tfidf("a", 2).unwrap();
        assert_eq!(terms[0].term, "rust");
        assert!(terms[0].score > terms[1].score);
    }

    #[test]
    fn searches_documents_with_tfidf_cosine() {
        let mut corpus = TfIdfCorpus::default();
        corpus
            .add_document("rust", "rust crates cargo ownership")
            .unwrap();
        corpus
            .add_document("video", "frames ffmpeg scene detection")
            .unwrap();

        let results = corpus.search("cargo crate rust", 1).unwrap();
        assert_eq!(results[0].id, "rust");
        assert!(results[0].score > 0.0);
    }

    #[test]
    fn bm25_ranks_documents_and_rejects_duplicates() {
        let mut corpus = Bm25Corpus::default();
        corpus
            .add_document("rust", "rust cargo crates rust ownership")
            .unwrap();
        corpus
            .add_document("video", "frames ffmpeg scene detection")
            .unwrap();

        assert!(corpus.add_document("rust", "duplicate").is_err());
        let results = corpus.search("rust cargo", 2).unwrap();
        assert_eq!(results[0].id, "rust");
        assert!(results[0].score > 0.0);
        assert!(corpus.search("!!!", 5).unwrap().is_empty());
        assert!(corpus.search("rust", 0).is_err());
    }

    #[test]
    fn bm25_honors_text_document_and_filters() {
        let mut stop_words = BTreeSet::new();
        stop_words.insert("the".to_string());
        let mut corpus = Bm25Corpus::new(Bm25Options {
            min_term_len: 4,
            stop_words,
            ..Bm25Options::default()
        });
        let document = TextDocument::new("doc", "the reliable reliable rust code");
        corpus.add_text_document(&document).unwrap();
        assert_eq!(corpus.documents()[0].term_counts.get("the"), None);
        assert_eq!(corpus.search("reliable", 1).unwrap()[0].id, "doc");
    }

    #[test]
    fn tfidf_from_texts_matches_incremental_indexing_and_assigns_ids() {
        let texts = ["rust text text", "video text", "audio scene transcript"];

        let from_texts = TfIdfCorpus::from_texts(texts, CorpusOptions::default()).unwrap();

        let mut incremental = TfIdfCorpus::default();
        incremental.add_document("doc-0", "rust text text").unwrap();
        incremental.add_document("doc-1", "video text").unwrap();
        incremental
            .add_document("doc-2", "audio scene transcript")
            .unwrap();

        assert_eq!(from_texts.stats(), incremental.stats());
        assert_eq!(from_texts.documents(), incremental.documents());
        assert_eq!(from_texts.documents()[0].id, "doc-0");
        assert_eq!(from_texts.documents()[1].id, "doc-1");
        assert_eq!(from_texts.documents()[2].id, "doc-2");
    }

    #[test]
    fn tfidf_from_documents_preserves_ids_and_supports_search() {
        let documents = vec![
            TextDocument::new("rust", "rust cargo crates ownership"),
            TextDocument::new("video", "video scenes reports frames"),
        ];

        let corpus = TfIdfCorpus::from_documents(documents, CorpusOptions::default()).unwrap();

        assert_eq!(corpus.document("rust").unwrap().id, "rust");
        assert_eq!(corpus.document("video").unwrap().unique_terms(), 4);
        assert_eq!(corpus.search("cargo rust", 1).unwrap()[0].id, "rust");
        assert!(!corpus.document_tfidf("video", 3).unwrap().is_empty());
    }

    #[test]
    fn tfidf_indexes_text_segments_incrementally() {
        let mut corpus = TfIdfCorpus::default();
        let segments = vec![
            TextSegment {
                segment_index: 0,
                timestamp: None,
                text: "rust cargo ownership",
                language: Some("en"),
                is_final: true,
            },
            TextSegment {
                segment_index: 1,
                timestamp: None,
                text: "video scene detection",
                language: Some("en"),
                is_final: true,
            },
            TextSegment {
                segment_index: 2,
                timestamp: None,
                text: "cargo crates and compiler",
                language: Some("en"),
                is_final: true,
            },
        ];
        for segment in &segments {
            corpus.add_text_segment("subs", segment).unwrap();
        }

        assert_eq!(corpus.documents()[0].id, "subs:0");
        assert_eq!(corpus.documents()[1].id, "subs:1");
        assert_eq!(corpus.documents()[2].id, "subs:2");
        assert_eq!(corpus.search("compiler cargo", 1).unwrap()[0].id, "subs:2");
        assert!(matches!(
            corpus.add_text_segment("subs", &segments[0]),
            Err(DetectError::InvalidArgument(message))
                if message.contains("document id `subs:0` already exists")
        ));
    }

    #[test]
    fn tfidf_from_text_segments_validates_stream_id() {
        let segments = vec![TextSegment {
            segment_index: 0,
            timestamp: None,
            text: "rust cargo",
            language: None,
            is_final: true,
        }];

        let error =
            TfIdfCorpus::from_text_segments("   ", segments, CorpusOptions::default()).unwrap_err();
        assert!(
            matches!(error, DetectError::InvalidArgument(message) if message == "stream id must not be empty")
        );
    }

    #[test]
    fn tfidf_batch_construction_honors_options() {
        let mut stop_words = BTreeSet::new();
        stop_words.insert("skip".to_string());
        let corpus = TfIdfCorpus::from_texts(
            ["skip alpha beta beta gamma gamma gamma"],
            CorpusOptions {
                min_term_len: 5,
                stop_words,
                max_terms_per_document: Some(2),
                ..CorpusOptions::default()
            },
        )
        .unwrap();

        let document = &corpus.documents()[0];
        assert_eq!(document.id, "doc-0");
        assert_eq!(document.total_terms, 4);
        assert_eq!(document.term_counts.get("gamma"), Some(&3));
        assert_eq!(document.term_counts.get("alpha"), Some(&1));
        assert_eq!(document.term_counts.get("beta"), None);
        assert_eq!(document.term_counts.get("skip"), None);
    }

    #[test]
    fn tfidf_batch_construction_accepts_empty_inputs_and_text() {
        let empty =
            TfIdfCorpus::from_texts(std::iter::empty::<&str>(), CorpusOptions::default()).unwrap();
        assert!(empty.is_empty());

        let corpus = TfIdfCorpus::from_texts([""], CorpusOptions::default()).unwrap();
        assert_eq!(corpus.len(), 1);
        assert_eq!(corpus.documents()[0].total_terms, 0);
    }

    #[test]
    fn tfidf_from_documents_reuses_duplicate_id_validation() {
        let documents = vec![
            TextDocument::new("dup", "rust cargo"),
            TextDocument::new("dup", "video scene"),
        ];

        let error = TfIdfCorpus::from_documents(documents, CorpusOptions::default()).unwrap_err();
        assert!(
            matches!(error, DetectError::InvalidArgument(message) if message.contains("document id `dup` already exists"))
        );
    }

    #[test]
    fn bm25_from_texts_builds_searchable_corpus_and_assigns_ids() {
        let corpus = Bm25Corpus::from_texts(
            [
                "rust cargo crates rust ownership",
                "frames ffmpeg scene detection",
            ],
            Bm25Options::default(),
        )
        .unwrap();

        assert_eq!(corpus.documents()[0].id, "doc-0");
        assert_eq!(corpus.documents()[1].id, "doc-1");
        assert_eq!(corpus.search("rust cargo", 1).unwrap()[0].id, "doc-0");
    }

    #[test]
    fn bm25_from_documents_preserves_ids_and_matches_incremental_behavior() {
        let documents = vec![
            TextDocument::new("rust", "rust cargo crates rust ownership"),
            TextDocument::new("video", "frames ffmpeg scene detection"),
        ];
        let from_documents =
            Bm25Corpus::from_documents(documents.clone(), Bm25Options::default()).unwrap();

        let mut incremental = Bm25Corpus::default();
        for document in documents {
            incremental.add_text_document(&document).unwrap();
        }

        assert_eq!(from_documents.documents(), incremental.documents());
        assert_eq!(
            from_documents.search("rust cargo", 1).unwrap()[0].id,
            "rust"
        );
    }

    #[test]
    fn bm25_indexes_text_segments_and_searches_generated_ids() {
        let segments = vec![
            TextSegment {
                segment_index: 0,
                timestamp: None,
                text: "guten tag berlin",
                language: Some("de"),
                is_final: true,
            },
            TextSegment {
                segment_index: 1,
                timestamp: None,
                text: "rust cargo crates ownership",
                language: Some("en"),
                is_final: true,
            },
            TextSegment {
                segment_index: 2,
                timestamp: None,
                text: "cargo build pipeline",
                language: Some("en"),
                is_final: true,
            },
        ];
        let corpus =
            Bm25Corpus::from_text_segments("subs", segments.clone(), Bm25Options::default())
                .unwrap();

        assert_eq!(corpus.documents()[0].id, "subs:0");
        assert_eq!(corpus.documents()[1].id, "subs:1");
        assert_eq!(corpus.documents()[2].id, "subs:2");
        assert_eq!(corpus.search("cargo ownership", 1).unwrap()[0].id, "subs:1");

        let mut incremental = Bm25Corpus::default();
        for segment in &segments {
            incremental.add_text_segment("subs", segment).unwrap();
        }
        assert_eq!(corpus.documents(), incremental.documents());
    }

    #[test]
    fn bm25_text_segment_ingestion_rejects_empty_stream_id() {
        let segment = TextSegment {
            segment_index: 0,
            timestamp: None,
            text: "rust cargo",
            language: None,
            is_final: true,
        };
        let error = Bm25Corpus::default()
            .add_text_segment(" ", &segment)
            .unwrap_err();
        assert!(
            matches!(error, DetectError::InvalidArgument(message) if message == "stream id must not be empty")
        );
    }

    #[test]
    fn bm25_batch_construction_honors_options_and_empty_inputs() {
        let mut stop_words = BTreeSet::new();
        stop_words.insert("skip".to_string());
        let corpus = Bm25Corpus::from_texts(
            ["skip beta beta gamma gamma"],
            Bm25Options {
                min_term_len: 5,
                stop_words,
                ..Bm25Options::default()
            },
        )
        .unwrap();

        assert_eq!(corpus.documents()[0].term_counts.get("skip"), None);
        assert_eq!(corpus.documents()[0].term_counts.get("beta"), None);
        assert_eq!(corpus.documents()[0].term_counts.get("gamma"), Some(&2));

        let empty = Bm25Corpus::from_documents(
            std::iter::empty::<TextDocument<'static>>(),
            Bm25Options::default(),
        )
        .unwrap();
        assert!(empty.is_empty());
    }

    #[test]
    fn bm25_from_documents_reuses_duplicate_id_validation() {
        let documents = vec![
            TextDocument::new("dup", "rust cargo"),
            TextDocument::new("dup", "video scene"),
        ];

        let error = Bm25Corpus::from_documents(documents, Bm25Options::default()).unwrap_err();
        assert!(
            matches!(error, DetectError::InvalidArgument(message) if message.contains("document id `dup` already exists"))
        );
    }

    #[test]
    fn sparse_term_outputs_round_trip() {
        let corpus =
            TfIdfCorpus::from_texts(["rust cargo", "cargo build"], CorpusOptions::default())
                .unwrap();
        let sparse = corpus.sparse_term_matrix().unwrap();
        assert_eq!(sparse.matrix.rows(), 2);
        assert!(sparse.vocabulary.iter().any(|term| term == "cargo"));
        let vector =
            sparse_term_vector(&corpus.documents()[0].term_counts, &sparse.vocabulary).unwrap();
        assert_eq!(
            vector
                .to_dense()
                .iter()
                .filter(|value| **value > 0.0)
                .count(),
            2
        );
    }
}
