# text-retrieval

Library-first semantic and hybrid retrieval for `video-analysis`.

Default builds are deterministic and local-first. Transcript integration is
feature-gated, and native model execution stays outside the default dependency
closure.

## Highlights

- Deterministic text chunking with token overlap
- Exact semantic retrieval over `vector-analysis-index`
- BM25 lexical retrieval over `text-lexical`
- Hybrid weighted ranking with metadata filters
- Related-content lookup and persistence-friendly export helpers

## Package surface

- Primary workflow: `retrieval.search` builds a transient in-memory retrieval
  index and searches it.
- Workflow operations: `retrieval.chunk`, `retrieval.search`, and
  `retrieval.rerank`.
- Debug operations: `describe` inspects package metadata and operation support.
- Runtime support: pure Rust, available through library, CLI, server, and WASM
  wrappers.
- Sample output includes `title`, `message`, `summary`, `result`, and
  operation-specific fields such as `chunks`, `report`, `mode`, or `results`.
- Package-surface operations do not write persistence artifacts or run native
  model inference.

## Related crates

- `text-embeddings`
- `text-lexical`
- `vector-analysis-index`
