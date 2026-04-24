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
use text_analysis_linguistics::{analyze_transcription, LinguisticAnalysisOptions};
use text_analysis_transcription::parse_srt;

let subtitles = parse_srt(
    "1\n00:00:00,000 --> 00:00:01,000\nAlice visited Berlin\n\n2\n00:00:01,000 --> 00:00:02,000\nShe presented the roadmap\n",
)?;
let analysis = analyze_transcription(
    &subtitles,
    &LinguisticAnalysisOptions::default(),
)?;

assert_eq!(analysis.cues.len(), 2);
assert!(!analysis.aggregate.entities.is_empty());
```

## Related crates

- `text-analysis-core`
- `text-analysis-models`
- `text-analysis-transcription` for SRT, WebVTT, Whisper JSON, and plain transcript parsing
- `text-analysis-corpus` for TF-IDF and BM25 corpus indexing from text inputs
- `video-analysis-core`
