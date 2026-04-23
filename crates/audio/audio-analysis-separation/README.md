# audio-analysis-separation

Demucs-based audio stem separation command wrapper for `video-analysis`.

## Feature flags

- `external-tests`: enables the real Demucs smoke test.

## Example

```rust,ignore
use audio_analysis_separation::{DemucsOptions, DemucsRunner};

let runner = DemucsRunner::default();
let options = DemucsOptions::default();
let result = runner.separate_file("input.wav", &options)?;

let _ = result;
```

## Related crates

- `audio-analysis-io`
- `audio-analysis-processing`
- `video-analysis-core`
