# image-analysis-comfyui

ComfyUI workflow builders and a small HTTP client for image generation and
manipulation in `video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use image_analysis_comfyui::{
    build_generation_workflow, ComfyImageEditExecutor, ComfyUiClientOptions, ImageGenerationRequest,
};

let workflow = build_generation_workflow(
    &ImageGenerationRequest::new("a geometric red fox").negative_prompt("blurry"),
)?;
assert!(!workflow.nodes.is_empty());
let executor = ComfyImageEditExecutor::new(ComfyUiClientOptions {
    base_url: None,
    ..ComfyUiClientOptions::default()
});
let status = executor.execute(&workflow, "edited.png")?;
assert_eq!(status.status, "planned");
# Ok(())
# }
```

## Related crates

- `comfyui-data`
- `comfyui-models`
