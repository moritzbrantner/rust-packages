# text-analysis-corpus

Corpus indexing, TF-IDF, and BM25 utilities for `video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use text_analysis_corpus::{Bm25Corpus, Bm25Options, CorpusOptions, TfIdfCorpus};

let tfidf = TfIdfCorpus::from_texts(
    ["multimodal analysis with rust", "scene graphs and video reports"],
    CorpusOptions::default(),
)?;

assert_eq!(tfidf.documents()[0].id, "doc-0");

let mut corpus = Bm25Corpus::new(Bm25Options::default());
corpus.add_document("doc-1", "multimodal analysis with rust")?;

let _results = corpus.search("analysis rust", 5)?;
```

## Related crates

- `text-analysis-core`
- `text-analysis-semantics`
