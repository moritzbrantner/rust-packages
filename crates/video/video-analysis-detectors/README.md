# video-analysis-detectors

Scene detection algorithms and detector adapters for `video-analysis`.

## Feature flags

- No optional feature flags today.

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
