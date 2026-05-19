# video-analysis-ffmpeg

FFmpeg-backed media ingest for `video-analysis`.

## Feature flags

- `ffmpeg-backend`: enables the backend surface
- `ffmpeg-command`: keeps the process-backed `ffmpeg`/`ffprobe` runtime enabled
- `ffmpeg-native`: enables native runtime API selection without requiring system FFmpeg development packages
- `ffmpeg-next-bindings`: reserved for future `ffmpeg-next`/system FFmpeg probing support
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
