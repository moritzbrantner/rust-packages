# image-analysis-ocr

OCR model presets, rich text outputs, and backend contracts for `video-analysis`.

## Feature flags

- No optional feature flags today.

## Runtime Surface

- Workflow operations: `image.ocr.documentSummary` summarizes imported OCR
  document structure.
- Debug operations: `image.ocr.presets`, `image.ocr.requestSummary`, and
  `describe` inspect model presets, request options, and package metadata.
- The surface does not download models or run OCR.

## Example

```rust
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use image_analysis_ocr::{OcrPreset, OcrRequest};

let spec = OcrPreset::TrOcrBasePrinted.model_spec();
let request = OcrRequest::new()
    .languages(["en"])
    .preserve_layout(true);

assert_eq!(spec.repo_id, "microsoft/trocr-base-printed");
assert!(request.preserve_layout);
# Ok(())
# }
```

## Related crates

- `image-analysis-core`
- `image-analysis-classification, image-analysis-embeddings, image-analysis-captioning, image-analysis-ocr, image-analysis-segmentation, or image-analysis-detection`
- `model-runtime`
