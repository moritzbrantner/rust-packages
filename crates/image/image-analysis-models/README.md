# image-analysis-models

Deprecated compatibility crate for `image-analysis-tasks`.

New code should import the owning capability crates directly:

- `image-analysis-segmentation` owns SAM presets and segmentation backends.
- `image-analysis-detection` owns detections, face detections, and detector backends.
- `image-analysis-ocr` owns OCR presets, rich text outputs, and OCR backends.
- `image-analysis-tasks` owns aggregate task catalogs plus classification,
  embedding, captioning, and face-embedding contracts.

## Feature flags

- No optional feature flags today.

## Example

```rust
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use image_analysis_core::{ImageView, OwnedImage};
use image_analysis_tasks::{ImageClassification, ImageClassifierBackend};
use video_analysis_core::Result;

struct StubClassifier;

impl ImageClassifierBackend for StubClassifier {
    fn classify_image(&mut self, _image: &ImageView<'_>) -> Result<Vec<ImageClassification>> {
        Ok(vec![ImageClassification::new("demo").score(1.0)])
    }
}

let image = OwnedImage::new_rgb(1, 1, vec![0, 0, 0])?;
let labels = StubClassifier.classify_image(&image.as_view())?;
assert_eq!(labels[0].label, "demo");
# Ok(())
# }
```

## Related crates

- `image-analysis-tasks`
- `image-analysis-detection`
- `image-analysis-onnx`
- `video-analysis-models`
