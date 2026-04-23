# image-analysis-segmentation

Image segmentation primitives and SAM model defaults for `video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use image_analysis_segmentation::{default_sam_model_spec, ImageSegmentationPrompt, SegmentationPoint};

let prompt = ImageSegmentationPrompt::new()
    .point(SegmentationPoint::foreground(200, 120))
    .multimask_output(false);

let _ = default_sam_model_spec();
let _ = prompt;
```

## Related crates

- `image-analysis-core`
- `image-analysis-detection`
- `video-analysis-segmentation`
