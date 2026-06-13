# Text Workspace Guide

`text-analysis::TextWorkspace` is the high-level deterministic workflow for
text documents, transcript segments, lexical corpus analysis, retrieval, and
report generation.

Default workspace execution is pure Rust. It uses `TextCorpus`, hashed
embeddings, in-memory `text-index` state, and soft-legacy compatibility
`RetrievalIndex` state. New search/index workflows should use the index-first
`build_index()`/`search_index()` methods; `build_retrieval_index()` and
`search()` remain compatibility paths. It does not download model bundles or
execute native model runtimes unless callers explicitly choose model-backed
workspace options or lower-level APIs.

## Primary Workflow

```rust,no_run
use text_analysis::{
    ClassificationDepth, TextWorkspace, TextWorkspaceOptions, WorkspaceDocument,
};
use text_core::{AsTextSegmentContract, TextSegmentContract};
use text_index::IndexQuery;
use text_transcripts::{parse_srt, TranscriptSegmentContract};

let transcript = parse_srt(
    "1\n00:00:03,000 --> 00:00:05,000\nRust retrieval cites timed transcript chunks.\n",
)?;
let mut segment = TranscriptSegmentContract::from(transcript.segments[0].clone())
    .as_text_segment_contract();
segment.stream_id = Some("subs".to_string());

let mut options = TextWorkspaceOptions::default();
options.document_analysis.classification_depth = ClassificationDepth::LexicalFallback;
options.corpus_analysis.document.classification_depth = ClassificationDepth::LexicalFallback;

let mut workspace = TextWorkspace::new(options);
workspace.ingest_documents([WorkspaceDocument::SegmentContract(segment)])?;

let document = workspace.analyze_document("subs:0")?;
let corpus = workspace.analyze_corpus()?;
workspace.build_index()?;
let search = workspace.search_index(IndexQuery::new("timed transcript citations", 5))?;
let snapshot = workspace.snapshot();

assert!(document.classification.is_some());
assert!(corpus.classification.is_some());
assert_eq!(search.results[0].document_id, "subs:0");
assert_eq!(snapshot.documents[0].timestamp.unwrap().seconds(), 3.0);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Index-Backed Workspace Search

Use `WorkspaceIndexOptions` when a workspace should build a memory or SQLite
Text Index. SQLite writes require `commit: true` and a path, and require the
`text-analysis/sqlite` feature. Package surfaces remain request-scoped; they do
not create server-side index sessions or keep open index handles between calls.

```rust,no_run
use text_analysis::{
    TextWorkspace, TextWorkspaceOptions, WorkspaceDocument, WorkspaceIndexOptions,
    WorkspaceIndexStorage,
};
use text_core::TextDocumentContract;
use text_index::{IndexBuildOptions, IndexQuery};

let mut workspace = TextWorkspace::new(TextWorkspaceOptions {
    index: WorkspaceIndexOptions {
        storage: WorkspaceIndexStorage::Memory,
        build: IndexBuildOptions {
            chunk_tokens: 16,
            chunk_overlap_tokens: 0,
            ..IndexBuildOptions::default()
        },
        embedding_dimensions: 64,
        commit: false,
    },
    ..TextWorkspaceOptions::default()
});
workspace.ingest_documents([WorkspaceDocument::DocumentContract(
    TextDocumentContract::new("doc-1", "Hybrid workspace search cites indexed chunks."),
)])?;
workspace.build_index()?;
let search = workspace.search_index(IndexQuery::new("indexed chunks", 3))?;
assert_eq!(search.results[0].document_id, "doc-1");
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Model-Backed Workspace

`TextWorkspace::default()` and `TextWorkspaceOptions::default()` remain
deterministic and no-download. Use explicit model-backed options when native
features and model bundles are intended:

```rust,no_run
use text_analysis::{TextWorkspace, TextWorkspaceOptions, WorkspaceDocument};
use text_core::TextDocumentContract;

let options = TextWorkspaceOptions {
    document_analysis: text_analysis::DocumentAnalysisOptions::model_backed_with_downloads(),
    corpus_analysis: text_analysis::CorpusAnalysisOptions {
        document: text_analysis::DocumentAnalysisOptions::model_backed_with_downloads(),
        ..text_analysis::CorpusAnalysisOptions::default()
    },
    ..TextWorkspaceOptions::default()
};

let mut workspace = TextWorkspace::new(options);
workspace.ingest_documents([WorkspaceDocument::DocumentContract(
    TextDocumentContract::new("doc-1", "Rust text workflows are reliable."),
)])?;

let report = workspace.analyze_document("doc-1")?;
assert!(report.classification.is_some());
# Ok::<(), Box<dyn std::error::Error>>(())
```

This mode delegates classification to `text-classification` local model options.
It does not move model internals into `TextWorkspace`; the workspace only
orchestrates the focused crates.

## Rich Text Contracts

`TextDocumentContract` and `TextSegmentContract` preserve:

- `language`, `timestamp`, and string `attributes`
- `source` with source id/kind, URI, media timestamp, and duration
- `provenance` records for crate, operation, model, runtime, and confidence
- `annotations` with spans, token indexes, and source segment ids

Transcript segment conversion fills typed timing fields:

```rust,no_run
use text_core::AsTextSegmentContract;
use text_transcripts::TranscriptSegmentContract;

let mut transcript_segment = TranscriptSegmentContract::new(7, "Timed subtitle text.");
transcript_segment.start_seconds = Some(12.5);
transcript_segment.end_seconds = Some(14.0);

let mut segment = transcript_segment.as_text_segment_contract();
segment.stream_id = Some("subs".to_string());
let document = segment.to_text_document_contract();

assert_eq!(document.id, "subs:7");
assert_eq!(document.timestamp.unwrap().seconds(), 12.5);
assert_eq!(
    document.source.as_ref().and_then(|source| source.duration_seconds),
    Some(1.5)
);
```

## Lower-Level Escape Hatches

Use `text-lexical::TextCorpus` directly when you only need corpus ownership,
TF-IDF, BM25, snapshots, or full-fidelity export with
`text_document_contracts()`.

Use `text-index::TextIndex` directly when you need durable memory/SQLite
indexing, semantic facets, analysis attachments, source/time/provenance
filters, hybrid score explanations, or snapshot planning.

Use `text-retrieval::RetrievalIndex` directly when you need soft-legacy
compatibility, persisted JSON/JSONL retrieval snapshots, related chunks, or
runtime-backed reranking through `rerank_documents_with_context`.

Use `text-question-answering::answer_question_with_text_index` for new cited QA
workflows that should build on the primary text-index path. Use
`text-question-answering::answer_question_with_retrieval` for deterministic
soft-legacy retrieval-backed cited answers.

Use `text-classification` directly for imported predictions, caller-supplied
classification backends, or request-level local model options.
