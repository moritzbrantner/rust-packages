# audio-analysis-processing

Realtime-safe audio transforms, named effect presets, deterministic whole-clip
offline edits, and loudness-oriented metrics for
`moritzbrantner-video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use audio_analysis_core::{AudioClip, FadeCurve};
use audio_analysis_processing::{
    AudioProcessor, DelaySpec, DistortionMode, DistortionSpec, FadeSpec, NormalizeSpec,
    OfflineAudioProcessor,
};

let mut realtime = AudioProcessor::new()
    .gain(0.75)
    .distortion(DistortionSpec {
        mode: DistortionMode::Tanh,
        drive_db: 6.0,
        mix: 0.4,
        output_gain_db: -1.0,
    })
    .delay(DelaySpec {
        delay_seconds: 0.2,
        feedback: 0.25,
        wet: 0.3,
        dry: 1.0,
    });

let clip = AudioClip::new(48_000, 1, vec![0.0; 48_000])?;
let mut offline = OfflineAudioProcessor::new()
    .fade(FadeSpec {
        fade_in_seconds: 0.01,
        fade_out_seconds: 0.05,
        curve: FadeCurve::EqualPower,
    })
    .normalize(NormalizeSpec {
        target_peak: Some(0.95),
        target_rms: None,
    });
let _processed = offline.process_clip(clip)?;
let _ = realtime;
```

## Processing Layers

- `AudioProcessor` keeps the streaming frame API and now includes distortion,
  delay/echo, reverb, compressor, limiter, EQ, chorus, flanger, tremolo, pan,
  and stereo width.
- `OfflineAudioProcessor` handles duration/order-changing operations on
  `AudioClip`: trim, reverse, fade, normalize, resample, speed, and pitch shift.
- `analyze_loudness` returns peak dBFS, RMS dBFS, crest factor, an approximate
  LUFS-style value, and the shared `AudioFeatureSeries` frame data used to
  derive the report. The LUFS value is a lightweight RMS-gated approximation,
  not an EBU R128 compliance result.
- Pure Rust pitch/time operations are deterministic baseline implementations;
  FFmpeg-backed file output should be preferred when production pitch/time
  quality is required.

## Related crates

- `audio-analysis-core`
- `audio-analysis-io`
- `video-analysis-ingest`
