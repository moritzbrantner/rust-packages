# video-analysis-ffmpeg

FFmpeg-backed media ingest for `video-analysis`.

## Feature flags

- `ffmpeg-backend`: enables the backend surface
- `ffmpeg-tests`: enables decode-oriented tests
- `external-tests`: extends ignored external coverage
- `test-utils`: utilities for integration tests

## Example

```rust,ignore
use video_analysis_ffmpeg::{FfmpegVideoSource, FfmpegVideoSourceOptions};

let source = FfmpegVideoSource::open("video.mp4", FfmpegVideoSourceOptions::recorded())?;
let _ = source.stream_info()?;
```

## Related crates

- `video-analysis-ingest`
- `audio-analysis-io`
- `video-analysis-cli`
