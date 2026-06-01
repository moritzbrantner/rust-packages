# audio-analysis-pitch

Autocorrelation pitch detection for `moritzbrantner-video-analysis` audio pipelines.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use audio_analysis_pitch::{PitchAnalyzer, PitchAnalyzerOptions};

let analyzer = PitchAnalyzer::new(PitchAnalyzerOptions::default())?;
let _ = analyzer;
```

## Related crates

- `audio-analysis-core`
- `audio-analysis-rhythm`
- `video-analysis-core`
