# text-linguistics

Local model-backed linguistic analysis pipeline for `video-analysis`.

## Highlights

- Language detection with script-aware fallbacks
- Tokenizer routing for word, subword, and mixed analysis modes
- Surface-to-subword alignment
- Lemmatization, morphology, POS tagging, chunking, and dependency parsing
- Local model-backed named entities through `CandleTokenClassifier`
- Heuristic rule extraction remains available through `LinguisticAnalysisOptions::heuristic()`
- Coreference, events, discourse, topics, and style analysis
- `TextAnalyzer` adapter for text pipelines

## Example

```rust,no_run
use text_linguistics::{TextNlpPipeline, TextNlpConfig, LinguisticAnalysisOptions};

let pipeline = TextNlpPipeline::new(TextNlpConfig {
    options: LinguisticAnalysisOptions::heuristic(),
    ..TextNlpConfig::fast()
});
let analysis = pipeline
    .analyze_text("Alice visited Berlin and presented the roadmap.")
    .unwrap();

assert!(!analysis.entities.is_empty());
assert_eq!(analysis.graph.tokens.len(), analysis.tokens.len());
```

When the `candle` feature is enabled, rich profiles can use a local
`bert-base-ner` token-classification model. The public Hugging Face bundle is
materialized into `.model-runtime` on first use through a `jobs-core`
download job and then runs locally through Candle; no OpenAI, Claude, or hosted
LLM token is required. Transcript-specific analysis is available behind the
`transcripts` feature. For deterministic offline tests or constrained
environments, use `LinguisticAnalysisOptions::heuristic()`.

## Related crates

- `text-core`
- `text-transcripts` for SRT, WebVTT, Whisper JSON, and plain transcript parsing
- `text-lexical` for TF-IDF and BM25 corpus indexing from text inputs
- `video-analysis-core`
