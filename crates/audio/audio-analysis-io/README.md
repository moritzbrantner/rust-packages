# audio-analysis-io

Audio input helpers, clip decode/write utilities, and FFmpeg-backed file editing
conveniences for `moritzbrantner-video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use audio_analysis_io::{
    build_ffmpeg_audio_filter_chain, decode_audio_to_clip, AudioInput, AudioInputOptions,
    FfmpegAudioEditSpec, FfmpegAudioEffect,
};

let input = AudioInput::File("fixtures/sample.wav".into());
let (_metadata, clip) = decode_audio_to_clip(input, AudioInputOptions::default())?;

let filter = build_ffmpeg_audio_filter_chain(&FfmpegAudioEditSpec {
    speed_factor: Some(1.25),
    pitch_shift_semitones: None,
    effects: vec![FfmpegAudioEffect::Normalize],
    output_sample_rate: Some(48_000),
    output_channels: Some(2),
})?;

let _ = (clip, filter);
```

## Hybrid File Editing

`decode_audio_to_clip` and `write_clip_as_wav` bridge file IO and the pure Rust
`AudioClip` API. File-level split, join, and process helpers use FFmpeg and
return clear errors when `ffmpeg` is unavailable.

The runtime surface remains preview-safe: `audio.io.editPlan`,
`audio.io.splitPlan`, `audio.io.joinPlan`, and `audio.io.ffmpegFilterPlan` return
deterministic plans and do not execute commands.

## Related crates

- `video-analysis-ffmpeg`
- `video-analysis-ingest`
- `audio-analysis-core`
