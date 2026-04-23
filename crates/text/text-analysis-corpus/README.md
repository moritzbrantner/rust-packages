# text-analysis-corpus

Corpus indexing, TF-IDF, and BM25 utilities for `video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use text_analysis_corpus::{Bm25Corpus, CorpusOptions};

let mut corpus = Bm25Corpus::new(CorpusOptions::default());
corpus.add_document("doc-1", "multimodal analysis with rust")?;

let _results = corpus.search("analysis rust", 5)?;
```

## Related crates

- `text-analysis-core`
- `text-analysis-semantics`
