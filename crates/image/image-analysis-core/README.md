# image-analysis-core

Shared image views, pixel formats, and image statistics for `video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use image_analysis_core::{luma_histogram, mean_rgb, ImageView, OwnedImage};

let image = OwnedImage::new_rgb(1920, 1080, vec![0; 1920 * 1080 * 3])?;
let view: ImageView<'_> = image.as_view();

let _ = mean_rgb(&view)?;
let _ = luma_histogram(&view, 16)?;
# Ok(())
# }
```

## Related crates

- `image-analysis-processing`
- `image-analysis-segmentation`
- `video-analysis-core`
