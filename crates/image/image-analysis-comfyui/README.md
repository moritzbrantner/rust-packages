# image-analysis-comfyui

ComfyUI workflow builders for image generation and manipulation in `video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use image_analysis_comfyui::{build_generation_workflow, ImageGenerationRequest};

let workflow = build_generation_workflow(
    &ImageGenerationRequest::new("a geometric red fox").negative_prompt("blurry"),
)?;
assert!(!workflow.nodes.is_empty());
# Ok(())
# }
```

## Related crates

- `comfyui-data`
- `comfyui-models`
