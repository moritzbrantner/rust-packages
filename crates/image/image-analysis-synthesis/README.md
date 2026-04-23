# image-analysis-synthesis

Deterministic image synthesis helpers for `video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use image_analysis_synthesis::{paint_regions, RgbColor};

let regions = paint_regions(320, 180, &[(RgbColor::new(255, 0, 0), 0, 0, 320, 180)])?;
let _ = regions;
```

## Related crates

- `data-inversion-core`
- `image-analysis-core`
