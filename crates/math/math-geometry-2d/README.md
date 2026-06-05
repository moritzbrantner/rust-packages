# math-geometry-2d

Shared 2D geometry contracts for multimodal image, video, and layout
processing.

## Highlights

- Checked pixel and normalized 2D primitives
- Rectangle intersection, union, and clamping helpers
- Rectangle IoU and directional overlap ratios
- Finite line-segment intersection parameters
- Affine transforms with compose and invert support
- Polygon area, centroid, winding, containment, and bounds helpers
- Normalized and pixel coordinate conversion helpers

## Example

```rust,no_run
use math_geometry_2d::{Affine2, NormalizedPoint2, RectU32, Size2u};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rect = RectU32::new(10, 12, 32, 24)?;
    let point = NormalizedPoint2::new(0.5, 0.5)?.to_pixel_point(Size2u::new(64, 48)?);
    let shifted = Affine2::translation(4.0, -2.0).apply_point(rect.center_f32());
    assert_eq!(point.x, 32);
    assert!(shifted.x > 0.0);
    Ok(())
}
```

## Related crates

- `video-analysis-core`
- `image-analysis-processing`
- `video-analysis-posture`
