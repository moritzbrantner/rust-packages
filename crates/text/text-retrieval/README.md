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

## Related crates

- `text-embeddings`
- `text-lexical`
- `vector-analysis-index`
