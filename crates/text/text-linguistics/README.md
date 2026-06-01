# text-linguistics

Local model-backed linguistic analysis pipeline for `moritzbrantner-video-analysis`.

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

## Package surface

- Primary workflow: `linguistics.analyze` runs the deterministic linguistic
  pipeline and returns tokens, language, entities, topics, and style signals.
- Workflow operations: `linguistics.analyze` and `linguistics.entities`.
- Debug operations: `describe` inspects package metadata and operation support.
- Runtime support: deterministic package-surface operations are pure Rust and
  available through library, CLI, server, and WASM wrappers; richer model-backed
  library APIs remain feature-gated.
- Sample output includes `title`, `message`, `summary`, `result`, and
  operation-specific fields such as `tokens`, `language`, `entities`,
  `relations`, or `events`.
- The package surface does not download models or execute hosted LLMs.

## Model Loading

The deterministic heuristic pipeline is always available. `dslim/bert-base-NER`
is the implemented opt-in Candle token-classification path; it becomes loadable
when the local bundle exists under `.model-runtime` and the crate is built with
`candle,model-bundles`.

```bash
scripts/sync_model_bundles.sh
cargo test -p text-linguistics --features external-tests -- --ignored
```

Browser package apps expose local benchmark scenarios in the `Benchmarks` tab.
Root scripts `bun run text-native:bench` and `bun run text-wasm:bench:all`
exercise native and browser paths respectively.

## Related crates

- `text-core`
- `text-transcripts` for SRT, WebVTT, Whisper JSON, and plain transcript parsing
- `text-lexical` for TF-IDF and BM25 corpus indexing from text inputs
- `video-analysis-core`
