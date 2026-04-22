use std::collections::{BTreeMap, BTreeSet};

use text_analysis_core::{tokenize, TextDocument, TextProcessingOptions, TokenKind};
use video_analysis_core::{DetectError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorpusOptions {
    pub processing: TextProcessingOptions,
    pub min_term_len: usize,
    pub stop_words: BTreeSet<String>,
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
pub struct IndexedDocument {
    pub id: String,
    pub term_counts: BTreeMap<String, usize>,
    pub total_terms: usize,
}

impl IndexedDocument {
    pub fn unique_terms(&self) -> usize {
        self.term_counts.len()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CorpusStats {
    pub documents: usize,
    pub total_terms: usize,
    pub unique_terms: usize,
    pub average_terms_per_document: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CorpusTermStats {
    pub term: String,
    pub collection_count: usize,
    pub document_count: usize,
    pub collection_frequency: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TfIdfTerm {
    pub term: String,
    pub count: usize,
    pub term_frequency: f32,
    pub inverse_document_frequency: f32,
    pub score: f32,
    pub document_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DocumentSearchResult {
    pub id: String,
    pub score: f32,
    pub matched_terms: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TfIdfCorpus {
    options: CorpusOptions,
    documents: Vec<IndexedDocument>,
    document_frequency: BTreeMap<String, usize>,
    collection_counts: BTreeMap<String, usize>,
    total_terms: usize,
}

impl Default for TfIdfCorpus {
    fn default() -> Self {
        Self::new(CorpusOptions::default())
    }
}

impl TfIdfCorpus {
    pub fn new(options: CorpusOptions) -> Self {
        Self {
            options,
            documents: Vec::new(),
            document_frequency: BTreeMap::new(),
            collection_counts: BTreeMap::new(),
            total_terms: 0,
        }
    }

    pub fn options(&self) -> &CorpusOptions {
        &self.options
    }

    pub fn len(&self) -> usize {
        self.documents.len()
    }

    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }

    pub fn documents(&self) -> &[IndexedDocument] {
        &self.documents
    }

    pub fn document(&self, id: &str) -> Option<&IndexedDocument> {
        self.documents.iter().find(|document| document.id == id)
    }

    pub fn add_text_document(&mut self, document: &TextDocument<'_>) -> Result<usize> {
        self.add_document(document.id, document.text)
    }

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

    pub fn document_tfidf(&self, id: &str, limit: usize) -> Result<Vec<TfIdfTerm>> {
        let document = self
            .document(id)
            .ok_or_else(|| invalid_argument(format!("document `{id}` was not indexed")))?;
        Ok(self.tfidf_for_counts(&document.term_counts, document.total_terms, limit))
    }

    pub fn text_tfidf(&self, text: &str, limit: usize) -> Vec<TfIdfTerm> {
        let counts = term_counts(text, &self.options);
        let total_terms = counts.values().sum();
        self.tfidf_for_counts(&counts, total_terms, limit)
    }

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

    pub fn document_count(&self, term: &str) -> usize {
        self.document_frequency.get(term).copied().unwrap_or(0)
    }

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

pub fn term_counts(text: &str, options: &CorpusOptions) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for term in terms(text, options) {
        *counts.entry(term).or_insert(0) += 1;
    }
    counts
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
}
