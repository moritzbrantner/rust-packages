#![doc = include_str!("../README.md")]

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use text_analysis_models::TextEmbedderBackend;
pub use text_analysis_retrieval::{
    DocumentChunk, IngestReport, IngestionOptions, RetrievalIndex, SearchDocument,
};
use text_analysis_retrieval::{
    HybridConfig, SearchFilter as RetrievalSearchFilter, SearchQuery as RetrievalSearchQuery,
    SearchResult as RetrievalSearchResult,
};
use video_analysis_core::{DetectError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum SearchMode {
    FullText,
    Semantic,
    Hybrid {
        semantic_weight: f32,
        full_text_weight: f32,
    },
}

impl Default for SearchMode {
    fn default() -> Self {
        Self::Hybrid {
            semantic_weight: 0.8,
            full_text_weight: 0.2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortOrder {
    Ascending,
    Descending,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchSort {
    Relevance,
    DocumentId(SortOrder),
    ChunkId(SortOrder),
    Metadata { key: String, order: SortOrder },
}

impl Default for SearchSort {
    fn default() -> Self {
        Self::Relevance
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SearchFilter {
    pub metadata_equals: BTreeMap<String, String>,
    pub metadata_contains: BTreeMap<String, String>,
    pub required_tags: Vec<String>,
    pub document_ids: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchOptions {
    pub query: String,
    pub top_k: usize,
    pub mode: SearchMode,
    pub filter: Option<SearchFilter>,
    pub sort: SearchSort,
    pub facet_fields: Vec<String>,
    pub candidate_limit: Option<usize>,
}

impl SearchOptions {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            top_k: 10,
            mode: SearchMode::default(),
            filter: None,
            sort: SearchSort::default(),
            facet_fields: Vec::new(),
            candidate_limit: None,
        }
    }

    pub fn top_k(mut self, top_k: usize) -> Self {
        self.top_k = top_k;
        self
    }

    pub fn mode(mut self, mode: SearchMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn filter(mut self, filter: SearchFilter) -> Self {
        self.filter = Some(filter);
        self
    }

    pub fn sort(mut self, sort: SearchSort) -> Self {
        self.sort = sort;
        self
    }

    pub fn facet_field(mut self, field: impl Into<String>) -> Self {
        self.facet_fields.push(field.into());
        self
    }

    pub fn candidate_limit(mut self, candidate_limit: usize) -> Self {
        self.candidate_limit = Some(candidate_limit);
        self
    }
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self::new("")
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchHit {
    pub chunk_id: String,
    pub document_id: String,
    pub score: f32,
    pub semantic_score: f32,
    pub full_text_score: f32,
    pub snippet: String,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SearchFacets {
    pub metadata: BTreeMap<String, BTreeMap<String, usize>>,
    pub tags: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchResponse {
    pub hits: Vec<SearchHit>,
    pub total_candidates: usize,
    pub facets: SearchFacets,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchEngine<B> {
    index: RetrievalIndex<B>,
}

impl<B: TextEmbedderBackend> SearchEngine<B> {
    pub fn new(embedder: B) -> Self {
        Self {
            index: RetrievalIndex::new(embedder),
        }
    }

    pub fn from_retrieval_index(index: RetrievalIndex<B>) -> Self {
        Self { index }
    }

    pub fn index(&self) -> &RetrievalIndex<B> {
        &self.index
    }

    pub fn index_mut(&mut self) -> &mut RetrievalIndex<B> {
        &mut self.index
    }

    pub fn into_retrieval_index(self) -> RetrievalIndex<B> {
        self.index
    }

    pub fn ingest_documents(
        &mut self,
        docs: &[SearchDocument],
        options: &IngestionOptions,
    ) -> Result<IngestReport> {
        self.index.ingest_documents(docs, options)
    }

    pub fn search(&self, options: &SearchOptions) -> Result<SearchResponse> {
        validate_options(options)?;

        let candidate_limit = candidate_limit(options);
        let retrieval_query = RetrievalSearchQuery {
            text: options.query.clone(),
            top_k: candidate_limit,
            filter: options.filter.as_ref().map(to_retrieval_filter),
            hybrid: to_hybrid_config(options.mode, candidate_limit)?,
        };

        let mut candidates = self
            .index
            .search(&retrieval_query)?
            .into_iter()
            .filter(|hit| matches_search_filter(hit, options.filter.as_ref()))
            .map(|hit| SearchHit::from_retrieval(hit, options.mode))
            .collect::<Vec<_>>();

        let total_candidates = candidates.len();
        let facets = build_facets(&candidates, &options.facet_fields);
        sort_hits(&mut candidates, &options.sort);
        candidates.truncate(options.top_k);

        Ok(SearchResponse {
            hits: candidates,
            total_candidates,
            facets,
        })
    }

    pub fn related(&self, chunk_id: &str, top_k: usize) -> Result<Vec<SearchHit>> {
        self.index
            .related_chunks(chunk_id, top_k)
            .map(|hits| hits.into_iter().map(SearchHit::from).collect())
    }
}

impl SearchHit {
    fn from_retrieval(value: RetrievalSearchResult, mode: SearchMode) -> Self {
        let mut hit = Self {
            chunk_id: value.chunk_id,
            document_id: value.document_id,
            score: value.score,
            semantic_score: value.semantic_score,
            full_text_score: value.lexical_score,
            snippet: value.snippet,
            metadata: value.metadata,
        };
        match mode {
            SearchMode::FullText => {
                hit.semantic_score = 0.0;
                hit.score = hit.full_text_score;
            }
            SearchMode::Semantic => {
                hit.full_text_score = 0.0;
                hit.score = hit.semantic_score;
            }
            SearchMode::Hybrid { .. } => {}
        }
        hit
    }
}

impl From<RetrievalSearchResult> for SearchHit {
    fn from(value: RetrievalSearchResult) -> Self {
        Self::from_retrieval(value, SearchMode::default())
    }
}

fn validate_options(options: &SearchOptions) -> Result<()> {
    if options.top_k == 0 {
        return Err(invalid_argument("search limit must be greater than zero"));
    }
    if matches!(&options.sort, SearchSort::Metadata { key, .. } if key.trim().is_empty()) {
        return Err(invalid_argument("metadata sort key must not be empty"));
    }
    if options
        .facet_fields
        .iter()
        .any(|field| field.trim().is_empty())
    {
        return Err(invalid_argument("facet fields must not be empty"));
    }
    let _ = to_hybrid_config(options.mode, candidate_limit(options))?;
    Ok(())
}

fn candidate_limit(options: &SearchOptions) -> usize {
    options
        .candidate_limit
        .unwrap_or_else(|| options.top_k.saturating_mul(8).max(options.top_k).max(32))
        .max(options.top_k)
}

fn to_hybrid_config(mode: SearchMode, candidate_limit: usize) -> Result<HybridConfig> {
    let (semantic_weight, lexical_weight) = match mode {
        SearchMode::FullText => (0.0, 1.0),
        SearchMode::Semantic => (1.0, 0.0),
        SearchMode::Hybrid {
            semantic_weight,
            full_text_weight,
        } => (semantic_weight, full_text_weight),
    };

    if !semantic_weight.is_finite()
        || !lexical_weight.is_finite()
        || semantic_weight < 0.0
        || lexical_weight < 0.0
    {
        return Err(invalid_argument(
            "search weights must be finite and non-negative",
        ));
    }
    if semantic_weight + lexical_weight <= f32::EPSILON {
        return Err(invalid_argument("search weights must not both be zero"));
    }

    Ok(HybridConfig {
        semantic_weight,
        lexical_weight,
        rerank_window: candidate_limit,
    })
}

fn to_retrieval_filter(filter: &SearchFilter) -> RetrievalSearchFilter {
    RetrievalSearchFilter {
        metadata_equals: filter.metadata_equals.clone(),
        required_tags: filter.required_tags.clone(),
    }
}

fn matches_search_filter(hit: &RetrievalSearchResult, filter: Option<&SearchFilter>) -> bool {
    let Some(filter) = filter else {
        return true;
    };

    if !filter.document_ids.is_empty() && !filter.document_ids.contains(&hit.document_id) {
        return false;
    }
    if !filter
        .metadata_equals
        .iter()
        .all(|(key, value)| hit.metadata.get(key) == Some(value))
    {
        return false;
    }
    if !filter.metadata_contains.iter().all(|(key, needle)| {
        hit.metadata
            .get(key)
            .is_some_and(|value| value.contains(needle))
    }) {
        return false;
    }

    let tags = metadata_tags(&hit.metadata);
    filter
        .required_tags
        .iter()
        .all(|tag| tags.iter().any(|candidate| candidate == tag))
}

fn sort_hits(hits: &mut [SearchHit], sort: &SearchSort) {
    hits.sort_by(|left, right| match sort {
        SearchSort::Relevance => relevance_order(left, right),
        SearchSort::DocumentId(order) => ordered(left.document_id.cmp(&right.document_id), *order)
            .then_with(|| relevance_order(left, right)),
        SearchSort::ChunkId(order) => ordered(left.chunk_id.cmp(&right.chunk_id), *order)
            .then_with(|| relevance_order(left, right)),
        SearchSort::Metadata { key, order } => {
            ordered(left.metadata.get(key).cmp(&right.metadata.get(key)), *order)
                .then_with(|| relevance_order(left, right))
        }
    });
}

fn relevance_order(left: &SearchHit, right: &SearchHit) -> std::cmp::Ordering {
    right
        .score
        .total_cmp(&left.score)
        .then_with(|| right.semantic_score.total_cmp(&left.semantic_score))
        .then_with(|| right.full_text_score.total_cmp(&left.full_text_score))
        .then_with(|| left.chunk_id.cmp(&right.chunk_id))
}

fn ordered(ordering: std::cmp::Ordering, order: SortOrder) -> std::cmp::Ordering {
    match order {
        SortOrder::Ascending => ordering,
        SortOrder::Descending => ordering.reverse(),
    }
}

fn build_facets(hits: &[SearchHit], fields: &[String]) -> SearchFacets {
    let mut facets = SearchFacets::default();
    let requested = fields.iter().cloned().collect::<BTreeSet<_>>();
    for hit in hits {
        for tag in metadata_tags(&hit.metadata) {
            *facets.tags.entry(tag).or_insert(0) += 1;
        }
        for field in &requested {
            if let Some(value) = hit.metadata.get(field) {
                *facets
                    .metadata
                    .entry(field.clone())
                    .or_default()
                    .entry(value.clone())
                    .or_insert(0) += 1;
            }
        }
    }
    facets
}

fn metadata_tags(metadata: &BTreeMap<String, String>) -> Vec<String> {
    metadata
        .get("tags")
        .map(|tags| {
            tags.split([',', ';'])
                .flat_map(|group| group.split_whitespace())
                .map(str::trim)
                .filter(|tag| !tag.is_empty())
                .map(ToString::to_string)
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn invalid_argument(message: impl Into<String>) -> DetectError {
    DetectError::InvalidArgument(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use text_analysis_corpus::CorpusOptions;
    use text_analysis_semantics::{HashedTextEmbedder, TextEmbeddingConfig};

    fn engine() -> SearchEngine<HashedTextEmbedder> {
        let embedder = HashedTextEmbedder::new(
            TextEmbeddingConfig {
                dimensions: 64,
                use_idf: true,
            },
            CorpusOptions::default(),
        )
        .unwrap();
        let mut engine = SearchEngine::new(embedder);
        engine
            .ingest_documents(
                &[
                    SearchDocument {
                        id: "rust-search".to_string(),
                        title: Some("Rust search".to_string()),
                        body: "Rust search crates combine BM25 full text with semantic vectors."
                            .to_string(),
                        metadata: BTreeMap::from([
                            ("category".to_string(), "engineering".to_string()),
                            ("lang".to_string(), "en".to_string()),
                            ("rank".to_string(), "2".to_string()),
                            ("tags".to_string(), "rust search".to_string()),
                        ]),
                    },
                    SearchDocument {
                        id: "policy".to_string(),
                        title: Some("Policy archive".to_string()),
                        body: "Compliance archives need exact keyword search and faceted filters."
                            .to_string(),
                        metadata: BTreeMap::from([
                            ("category".to_string(), "governance".to_string()),
                            ("lang".to_string(), "en".to_string()),
                            ("rank".to_string(), "1".to_string()),
                            ("tags".to_string(), "policy search".to_string()),
                        ]),
                    },
                    SearchDocument {
                        id: "recipes".to_string(),
                        title: Some("Recipe notes".to_string()),
                        body: "Recipe collections are usually filtered by ingredient and season."
                            .to_string(),
                        metadata: BTreeMap::from([
                            ("category".to_string(), "food".to_string()),
                            ("lang".to_string(), "en".to_string()),
                            ("rank".to_string(), "3".to_string()),
                            ("tags".to_string(), "notes".to_string()),
                        ]),
                    },
                ],
                &IngestionOptions {
                    chunk_tokens: 32,
                    chunk_overlap_tokens: 0,
                    store_raw_text: true,
                },
            )
            .unwrap();
        engine
    }

    #[test]
    fn hybrid_search_returns_ranked_hits() {
        let response = engine()
            .search(&SearchOptions::new("rust semantic search").top_k(2))
            .unwrap();

        assert_eq!(response.hits[0].document_id, "rust-search");
        assert!(response.hits[0].score > 0.0);
    }

    #[test]
    fn full_text_search_can_prioritize_exact_terms() {
        let response = engine()
            .search(
                &SearchOptions::new("compliance archive")
                    .mode(SearchMode::FullText)
                    .top_k(1),
            )
            .unwrap();

        assert_eq!(response.hits[0].document_id, "policy");
        assert_eq!(response.hits[0].semantic_score, 0.0);
        assert!(response.hits[0].full_text_score > 0.0);
    }

    #[test]
    fn filters_by_metadata_tags_and_documents() {
        let filter = SearchFilter {
            metadata_equals: BTreeMap::from([("category".to_string(), "governance".to_string())]),
            required_tags: vec!["policy".to_string()],
            document_ids: BTreeSet::from(["policy".to_string()]),
            ..SearchFilter::default()
        };

        let response = engine()
            .search(&SearchOptions::new("search filters").filter(filter))
            .unwrap();

        assert_eq!(response.hits.len(), 1);
        assert_eq!(response.hits[0].document_id, "policy");
    }

    #[test]
    fn sorts_candidates_by_metadata() {
        let response = engine()
            .search(
                &SearchOptions::new("search")
                    .top_k(3)
                    .sort(SearchSort::Metadata {
                        key: "rank".to_string(),
                        order: SortOrder::Ascending,
                    }),
            )
            .unwrap();

        let ids = response
            .hits
            .iter()
            .map(|hit| hit.document_id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(ids, vec!["policy", "rust-search", "recipes"]);
    }

    #[test]
    fn builds_metadata_and_tag_facets() {
        let response = engine()
            .search(
                &SearchOptions::new("search")
                    .top_k(10)
                    .facet_field("category"),
            )
            .unwrap();

        assert_eq!(response.facets.metadata["category"]["engineering"], 1);
        assert_eq!(response.facets.metadata["category"]["governance"], 1);
        assert_eq!(response.facets.tags["search"], 2);
    }
}
