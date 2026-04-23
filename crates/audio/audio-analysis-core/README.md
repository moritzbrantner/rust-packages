# audio-analysis-core

Shared audio frame conversion, windowing, and streaming helpers for `video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use audio_analysis_core::{FrameSpec, StreamingFrameBuffer};
use video_analysis_core::{AudioBuffer, OwnedAudioFrame, Timebase, Timestamp};

let frame = OwnedAudioFrame::new(
    Timestamp::new(0, Timebase::new(1, 48_000)),
    48_000,
    1,
    AudioBuffer::F32(vec![0.0; 4_096]),
)?;

let spec = FrameSpec::new(2_048, 512)?;
let mut windows = StreamingFrameBuffer::new(spec);
let frames = windows.push(frame.as_frame())?;

assert!(!frames.is_empty());
```

## Related crates

- `video-analysis-core`
- `audio-analysis-fourier`
- `audio-analysis-processing`
