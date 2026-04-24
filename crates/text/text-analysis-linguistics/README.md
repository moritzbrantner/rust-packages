# text-analysis-linguistics

Heuristic-first linguistic analysis pipeline for `video-analysis`.

## Highlights

- Language detection with script-aware fallbacks
- Tokenizer routing for word, subword, and mixed analysis modes
- Surface-to-subword alignment
- Lemmatization, morphology, POS tagging, chunking, and dependency parsing
- Heuristic named entities, coreference, events, discourse, topics, and style
- `TextAnalyzer` adapter for text pipelines

## Example

```rust,ignore
use text_analysis_linguistics::{analyze_text, LinguisticAnalysisOptions};

let analysis = analyze_text(
    "Alice visited Berlin and presented the roadmap.",
    &LinguisticAnalysisOptions::default(),
)?;

assert_eq!(analysis.language.primary.as_ref().map(|p| p.language.as_str()), Some("en"));
assert!(!analysis.entities.is_empty());
```

## Related crates

- `text-analysis-core`
- `text-analysis-models`
- `text-analysis-corpus` for TF-IDF and BM25 corpus indexing from text inputs
- `video-analysis-core`
