# image-analysis-core

Shared image views, pixel formats, and image statistics for `video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use image_analysis_core::{ImageView, OwnedImage};

let image = OwnedImage::new_rgb(1920, 1080, vec![0; 1920 * 1080 * 3])?;
let view: ImageView<'_> = image.as_view();

let _ = view.mean_rgb();
let _ = view.luma_histogram(16);
```

## Related crates

- `image-analysis-processing`
- `image-analysis-segmentation`
- `video-analysis-core`
