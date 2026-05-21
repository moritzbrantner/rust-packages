# text-linguistics

Local model-backed linguistic analysis pipeline for `video-analysis`.

## Highlights

- Language detection with script-aware fallbacks
- Tokenizer routing for word, subword, and mixed analysis modes
- Surface-to-subword alignment
- Lemmatization, morphology, POS tagging, chunking, and dependency parsing
- Local model-backed named entities through `text_models::CandleTokenClassifier`
- Heuristic rule extraction remains available through `LinguisticAnalysisOptions::heuristic()`
- Coreference, events, discourse, topics, and style analysis
- `TextAnalyzer` adapter for text pipelines

## Example

```rust,no_run
use text_linguistics::{TextNlpPipeline, TextNlpConfig};
use text_transcripts::parse_srt;

let subtitles = parse_srt(
    "1\n00:00:00,000 --> 00:00:01,000\nAlice visited Berlin\n\n2\n00:00:01,000 --> 00:00:02,000\nShe presented the roadmap\n",
) .unwrap();
let pipeline = TextNlpPipeline::new(TextNlpConfig::rich());
let analysis = pipeline.analyze_transcription(&subtitles).unwrap();

assert_eq!(analysis.cues.len(), 2);
assert!(!analysis.aggregate.entities.is_empty());
assert_eq!(analysis.aggregate.graph.tokens.len(), analysis.aggregate.tokens.len());
```

`analyze_text` uses a local `bert-base-ner` token-classification model by
default. The public Hugging Face bundle is materialized into
`.video-analysis-models` on first use and then runs locally through Candle; no
OpenAI, Claude, or hosted LLM token is required. For deterministic offline tests
or constrained environments, use `LinguisticAnalysisOptions::heuristic()`.

## Related crates

- `text-core`
- `text-models`
- `text-transcripts` for SRT, WebVTT, Whisper JSON, and plain transcript parsing
- `text-lexical` for TF-IDF and BM25 corpus indexing from text inputs
- `video-analysis-core`
