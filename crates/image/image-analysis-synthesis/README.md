# image-analysis-synthesis

Deterministic image synthesis helpers for `video-analysis`.

## Feature flags

- No optional feature flags today.

## Runtime Surface

- `image.synthesis.solid` summarizes deterministic solid-image output.
- `image.synthesis.gradient` summarizes deterministic vertical gradients.
- `image.synthesis.histogram` summarizes luma-histogram image expansion.

Surface operations return dimensions, color statistics, and inversion traces
instead of encoded image bytes.

## Example

```rust
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use image_analysis_core::ImagePixelFormat;
use image_analysis_synthesis::{paint_regions, ImageSynthesisConfig, RegionPaint, RgbColor};
use video_analysis_core::BoundingBox;

let regions = paint_regions(
    RgbColor::BLACK,
    &[RegionPaint {
        region: BoundingBox::new(0, 0, 320, 180)?,
        color: RgbColor::new(255, 0, 0),
    }],
    ImageSynthesisConfig::new(320, 180, ImagePixelFormat::Rgb24)?,
)?;
let _ = regions;
# Ok(())
# }
```

## Related crates

- `data-inversion-core`
- `image-analysis-core`
