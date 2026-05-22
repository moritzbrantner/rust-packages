# image-analysis-ocr

OCR model presets, rich text outputs, and backend contracts for `video-analysis`.

## Feature flags

- No optional feature flags today.

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
- `image-analysis-tasks`
- `video-analysis-models`
