# image-analysis-tasks

Aggregate image task schemas, catalogs, and backend contracts for
`video-analysis`.

This crate is the broad task surface for image classification, embedding,
captioning, OCR, segmentation, detection, face analysis, and generation. Core
image buffers, deterministic processing, segmentation masks, detections, OCR,
and synthesis remain owned by their focused crates.

## Feature flags

- No optional feature flags today.

## Example

```rust
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use image_analysis_tasks::{image_task_catalog, parse_task, ImageTask};

assert_eq!(parse_task("classify"), Some(ImageTask::ImageClassification));
assert!(!image_task_catalog(None).is_empty());
# Ok(())
# }
```

## Related crates

- `image-analysis-core`
- `image-analysis-detection`
- `image-analysis-ocr`
- `image-analysis-segmentation`
- `image-analysis-synthesis`
