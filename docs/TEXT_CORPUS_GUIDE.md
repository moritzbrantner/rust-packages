# Text Corpus Guide

This guide explains how the text corpus and search types fit together. All
examples are deterministic and local-only.

## Benchmarking Corpus Workflows

The text package apps include a `Benchmarks` result tab for local browser WASM timing. Scenarios cover tokenization, lexical keyword extraction, semantic and hybrid search, document reports, corpus reports, linguistic analysis, fallback classification, imported-span QA, generation, and transcript parsing.

Run all browser text scenarios from the root:

```bash
bun run text-wasm:bench:all
```

Run native text Criterion benches:

```bash
bun run text-native:bench
```

Use benchmark output for local regressions only. Corpus size, browser version, CPU throttling, and build profile all affect the numbers.

## Corpus Types

`TextCorpus` is the user-facing document collection in `text-lexical`. It owns
document text, stable IDs, optional language tags, and metadata. Use it when you
are assembling a reusable set of raw documents before deriving indexes or
snapshots.

`TfIdfCorpus` is a lexical scoring/index structure. It stores deterministic term
counts and document frequencies for TF-IDF terms and cosine-style TF-IDF search.

`Bm25Corpus` is a lexical ranking structure. It stores deterministic term counts
and document frequencies for BM25 full-text ranking.

`SemanticTextIndex` is a vector search index over deterministic hashed
embeddings from `text-embeddings`. It is useful for local semantic similarity
without model downloads or native inference.

`RetrievalIndex` is a chunked, metadata-aware retrieval index from
`text-retrieval`. It combines token-window chunking, vector search, BM25
full-text search, hybrid ranking, metadata filters, and persistence-friendly
export helpers.

`CorpusAnalysisReport` is analysis output from `text-analysis`. It is not a
stored corpus. It reports corpus stats, per-document analysis, lexical search
results, near duplicates, semantic neighbors, and diagnostics.

## Corpus Creation

```rust,no_run
use std::collections::BTreeMap;
use text_lexical::{CorpusOptions, TextCorpus, TextCorpusDocument};

let mut document = TextCorpusDocument::new(
    "doc-1",
    "Rust text crates expose lexical, semantic, and retrieval workflows.",
);
document.language = Some("en".to_string());
document.metadata = BTreeMap::from([
    ("source".to_string(), "guide".to_string()),
    ("kind".to_string(), "overview".to_string()),
]);

let corpus = TextCorpus::from_documents([document], CorpusOptions::default())?;
assert_eq!(corpus.len(), 1);
# Ok::<(), video_analysis_core::DetectError>(())
```

## TF-IDF Search

```rust,no_run
use text_lexical::{CorpusOptions, TextCorpus};

let corpus = TextCorpus::from_texts(
    [
        "TF-IDF ranks lexical terms for local documents.",
        "Hashed embeddings support deterministic semantic search.",
    ],
    CorpusOptions::default(),
)?;
let tfidf = corpus.to_tfidf_corpus()?;

for result in tfidf.search("lexical document ranking", 3)? {
    println!("{} {:.3}", result.id, result.score);
}
# Ok::<(), video_analysis_core::DetectError>(())
```

## BM25 Search

```rust,no_run
use text_lexical::{Bm25Options, CorpusOptions, TextCorpus};

let corpus = TextCorpus::from_texts(
    [
        "BM25 ranks full-text matches.",
        "Semantic search uses hashed vectors.",
    ],
    CorpusOptions::default(),
)?;
let bm25 = corpus.to_bm25_corpus(Bm25Options::default())?;

for result in bm25.search("full text matches", 3)? {
    println!("{} {:.3}", result.id, result.score);
}
# Ok::<(), video_analysis_core::DetectError>(())
```

## Semantic Search

```rust,no_run
use text_core::TextDocument;
use text_embeddings::{HashedTextEmbedder, SemanticTextIndex, TextEmbeddingConfig};
use text_lexical::CorpusOptions;

let documents = [
    TextDocument::new("doc-1", "Lexical search ranks exact terms."),
    TextDocument::new("doc-2", "Semantic search compares hashed embeddings."),
];
let embedder = HashedTextEmbedder::new(
    TextEmbeddingConfig {
        dimensions: 64,
        use_idf: true,
    },
    CorpusOptions::default(),
)?;
let mut index = SemanticTextIndex::new(embedder);
index.add_documents(documents)?;

for result in index.search("local embedding similarity", 2)? {
    println!("{} {:.3}", result.id, result.score);
}
# Ok::<(), video_analysis_core::DetectError>(())
```

## Hybrid Retrieval

```rust,no_run
use std::collections::BTreeMap;
use text_embeddings::{HashedTextEmbedder, TextEmbeddingConfig};
use text_lexical::CorpusOptions;
use text_retrieval::{
    HybridConfig, IngestionOptions, RetrievalIndex, SearchDocument, SearchFilter, SearchQuery,
};

let mut document = SearchDocument::new(
    "doc-1",
    "Hybrid retrieval combines full-text scoring with hashed vector search.",
);
document.metadata = BTreeMap::from([
    ("source".to_string(), "guide".to_string()),
    ("kind".to_string(), "retrieval".to_string()),
]);

let embedder = HashedTextEmbedder::new(
    TextEmbeddingConfig {
        dimensions: 64,
        use_idf: false,
    },
    CorpusOptions::default(),
)?;
let mut index = RetrievalIndex::new(embedder);
index.ingest_documents(
    &[document],
    &IngestionOptions {
        chunk_tokens: 12,
        chunk_overlap_tokens: 3,
        store_raw_text: true,
    },
)?;

let mut metadata_equals = BTreeMap::new();
metadata_equals.insert("kind".to_string(), "retrieval".to_string());
let query = SearchQuery::hybrid(
    "full text vector search",
    5,
    HybridConfig {
        semantic_weight: 0.6,
        lexical_weight: 0.4,
        rerank_window: 16,
    },
)
.filter(SearchFilter {
    metadata_equals,
    ..SearchFilter::default()
});

for result in index.search(&query)? {
    println!("{} {} {:.3}", result.document_id, result.chunk_id, result.score);
}
# Ok::<(), video_analysis_core::DetectError>(())
```

## Snapshot Export And Import

```rust,no_run
use text_lexical::{CorpusOptions, TextCorpus, TextCorpusSnapshot};

let corpus = TextCorpus::from_texts(
    ["Snapshots preserve deterministic lexical term state."],
    CorpusOptions::default(),
)?;
let json = serde_json::to_string_pretty(&corpus.snapshot()?)?;
let snapshot: TextCorpusSnapshot = serde_json::from_str(&json)?;
snapshot.validate()?;

let tfidf = snapshot.to_tfidf_corpus()?;
assert_eq!(tfidf.search("lexical snapshots", 1)?.len(), 1);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Analysis Reports

Use `text-analysis` when you want a report rather than a stored corpus:

```rust,no_run
use text_analysis::{analyze_corpus, CorpusAnalysisOptions};
use text_core::TextDocument;

let documents = [
    TextDocument::new("doc-1", "Corpus analysis reports keywords."),
    TextDocument::new("doc-2", "Corpus analysis finds semantic neighbors."),
];
let report = analyze_corpus(documents, &CorpusAnalysisOptions::default())?;

println!("documents={}", report.stats.documents);
# Ok::<(), video_analysis_core::DetectError>(())
```
