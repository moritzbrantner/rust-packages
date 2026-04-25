# image-analysis-models

Image model presets and backend traits for `video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use image_analysis_core::{ImageView, OwnedImage};
use image_analysis_models::{ImageClassification, ImageClassifierBackend};
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

- `image-analysis-detection`
- `image-analysis-onnx`
- `video-analysis-models`
