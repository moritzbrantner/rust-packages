# audio-analysis-rhythm

Onset and tempo analysis for `moritzbrantner-video-analysis` audio pipelines.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use audio_analysis_rhythm::{RhythmAnalyzer, RhythmAnalyzerOptions};

let analyzer = RhythmAnalyzer::new(RhythmAnalyzerOptions::default())?;
let _ = analyzer;
```

## Related crates

- `audio-analysis-core`
- `audio-analysis-fourier`
- `audio-analysis-pitch`
