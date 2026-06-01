# video-analysis-synthesis

Deterministic storyboard and frame synthesis for `moritzbrantner-video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use video_analysis_synthesis::Storyboard;

let storyboard = Storyboard::default();
let _ = storyboard.render()?;
```

## Related crates

- `video-analysis-core`
- `data-inversion-core`
