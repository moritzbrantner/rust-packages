# audio-analysis-io

Audio input helpers and FFmpeg-backed source conveniences for `video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use audio_analysis_io::{open_audio_input, AudioInput, AudioInputOptions};

let input = AudioInput::from_path("fixtures/sample.wav");
let options = AudioInputOptions::default();
let source = open_audio_input(input, options)?;

let _ = source;
```

## Related crates

- `video-analysis-ffmpeg`
- `video-analysis-ingest`
- `audio-analysis-core`
