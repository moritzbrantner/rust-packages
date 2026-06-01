# audio-analysis-synthesis

Deterministic audio synthesis from analysis events for `moritzbrantner-video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use audio_analysis_synthesis::{synthesize_tone_timeline, ToneEvent};

let events = vec![ToneEvent::new(440.0, 0.0, 0.5)?];
let frames = synthesize_tone_timeline(&events, 48_000)?;

let _ = frames;
```

## Related crates

- `audio-analysis-core`
- `data-inversion-core`
- `video-analysis-core`
