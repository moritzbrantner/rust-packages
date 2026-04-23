# audio-analysis-processing

Realtime-safe audio transforms and processed sources for `video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use audio_analysis_processing::{AudioProcessorChain, GainProcessor, MonoProcessor};

let mut chain = AudioProcessorChain::default();
chain.push(GainProcessor::linear(0.75)?);
chain.push(MonoProcessor::default());

let _ = chain;
```

## Related crates

- `audio-analysis-core`
- `audio-analysis-io`
- `video-analysis-ingest`
