# text-lexical

Corpus indexing, TF-IDF, and BM25 utilities for `video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use text_lexical::{Bm25Corpus, Bm25Options, CorpusOptions, TfIdfCorpus};
use text_transcripts::{parse_webvtt, segment_to_owned_text_segment};

let tfidf = TfIdfCorpus::from_texts(
    ["multimodal analysis with rust", "scene graphs and video reports"],
    CorpusOptions::default(),
)?;

assert_eq!(tfidf.documents()[0].id, "doc-0");

let mut corpus = Bm25Corpus::new(Bm25Options::default());
corpus.add_document("doc-1", "multimodal analysis with rust")?;

let _results = corpus.search("analysis rust", 5)?;

let subtitles = parse_webvtt(
    "WEBVTT\n\n00:00:00.000 --> 00:00:01.000\nRust cargo crates\n\n00:00:01.000 --> 00:00:02.000\nCargo build pipeline\n",
)?;
let mut subtitle_corpus = TfIdfCorpus::new(CorpusOptions::default());
for cue in &subtitles.segments {
    let segment = segment_to_owned_text_segment(cue);
    subtitle_corpus.add_text_segment("subs", &segment.as_segment())?;
}

assert_eq!(subtitle_corpus.documents()[0].id, "subs:0");
```

## Related crates

- `text-core`
- `text-transcripts`
- `text-embeddings`
