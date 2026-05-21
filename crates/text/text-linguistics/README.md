# text-linguistics

Heuristic-first linguistic analysis pipeline for `video-analysis`.

## Highlights

- Language detection with script-aware fallbacks
- Tokenizer routing for word, subword, and mixed analysis modes
- Surface-to-subword alignment
- Lemmatization, morphology, POS tagging, chunking, and dependency parsing
- Heuristic named entities, coreference, events, discourse, topics, and style
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

## Related crates

- `text-core`
- `text-models`
- `text-transcripts` for SRT, WebVTT, Whisper JSON, and plain transcript parsing
- `text-lexical` for TF-IDF and BM25 corpus indexing from text inputs
- `video-analysis-core`
