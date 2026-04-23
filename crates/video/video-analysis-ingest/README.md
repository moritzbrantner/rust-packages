# video-analysis-ingest

Media ingest traits and source adapters for `video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use video_analysis_ingest::{AudioFrameSource, TextSegmentSource, VideoFrameSource};

fn accept_sources(
    _video: &mut dyn VideoFrameSource,
    _audio: &mut dyn AudioFrameSource,
    _text: &mut dyn TextSegmentSource,
) {
}
```

## Related crates

- `video-analysis-core`
- `video-analysis-ffmpeg`
- `audio-analysis-io`
