# image-analysis-segmentation

Image segmentation primitives and SAM model defaults for `video-analysis`.

## Feature flags

- No optional feature flags today.

## Runtime Surface

- `image.segmentation.model` returns the default SAM model spec.
- `image.segmentation.promptSummary` validates prompt/request JSON.
- `image.segmentation.maskSummary` summarizes raw or rectangle-built masks
  without running SAM.

## Example

```rust
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use image_analysis_segmentation::{
    ImageSegmentationPrompt, ImageSegmentationRequest, SegmentationPoint,
};

let prompt = ImageSegmentationPrompt::new()
    .point(SegmentationPoint::foreground(200, 120))
    .multimask_output(false);

let request = ImageSegmentationRequest::new(prompt);
let _ = request.min_mask_pixels(32);
# Ok(())
# }
```

## Related crates

- `image-analysis-core`
- `image-analysis-detection`
- `image-analysis-classification, image-analysis-embeddings, image-analysis-captioning, image-analysis-ocr, image-analysis-segmentation, or image-analysis-detection`
