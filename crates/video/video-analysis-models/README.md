# video-analysis-models

> Deprecated compatibility crate: generic model infrastructure now belongs in
> `model-runtime`. New domain APIs should hide model usage behind capability
> crates such as detection, recognition, OCR, segmentation, and embeddings.

Model bundle, download, and adapter contracts for `video-analysis`.

## Feature flags

- `external-tests`: enables ignored model download coverage

CUDA-Oxide support is represented as an explicit runtime plan via
`CudaOxideModelPlan` and `CudaOxideRuntimeConfig`. It is intentionally
dependency-free in this crate: actual kernels should live in a cuda-oxide
package built with `cargo oxide`, while this crate records the Hugging Face
model spec, CUDA device, target SM, module name, and kernel names used by that
runtime.

## Example

```rust,ignore
use video_analysis_models::{HuggingFaceModelSpec, ModelTask, ModelBundle};

let spec = HuggingFaceModelSpec::new("Xenova/yolos-tiny", ModelTask::ObjectDetection);
let bundle = ModelBundle::from_spec(&spec, ".video-analysis-models")?;

let _ = bundle.manifest();
```

```rust
use video_analysis_models::{CudaOxideRuntimeConfig, ModelPreset};

let plan = ModelPreset::MiniLmL6V2
    .spec()
    .cuda_oxide_plan("text_embed_cuda", ["mean_pool_embeddings"])
    .runtime(CudaOxideRuntimeConfig::new().target_sm("sm_80"));

assert_eq!(plan.attributes()["runtime.backend"], "cuda_oxide");
```

## Related crates

- `video-analysis-onnx`
- `text-linguistics`
- `text-embeddings`
- `video-analysis-cli`
