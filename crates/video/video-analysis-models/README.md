# video-analysis-models

Model bundle, download, and adapter contracts for `video-analysis`.

## Feature flags

- `external-tests`: enables ignored model download coverage

## Example

```rust,ignore
use video_analysis_models::{HuggingFaceModelSpec, ModelTask, ModelBundle};

let spec = HuggingFaceModelSpec::new("Xenova/yolos-tiny", ModelTask::ObjectDetection);
let bundle = ModelBundle::from_spec(&spec, ".video-analysis-models")?;

let _ = bundle.manifest();
```

## Related crates

- `video-analysis-onnx`
- `text-analysis-models`
- `video-analysis-cli`
