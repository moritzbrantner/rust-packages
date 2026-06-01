# video-analysis-detectors

Scene detection algorithms and detector adapters for `moritzbrantner-video-analysis`.

## Feature flags

- No optional feature flags today.

## Runtime surface

- `video.detectors.registry` summarizes detector families and score algorithms.
- `video.detectors.flashFilter` runs the deterministic flash filter over frame
  threshold decisions.
- `video.detectors.compositePlan` validates weighted composite detector
  configuration without processing media.

## Example

```rust,ignore
use video_analysis_detectors::ContentDetector;

let detector = ContentDetector::new(27.0)?;
let _ = detector;
```

## Related crates

- `video-analysis-core`
- `video-analysis-cli`
- `video-analysis-split`
