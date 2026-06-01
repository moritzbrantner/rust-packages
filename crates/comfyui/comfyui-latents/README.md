# comfyui-latents

ComfyUI-oriented latent-space contracts for `moritzbrantner-video-analysis`.

## Highlights

- Validated latent batches over shared `moritzbrantner-tensor-data`
- Explicit latent mask compatibility checks
- Image-size helpers for ComfyUI-style 1/8 latent scaling

## Example

```rust,no_run
use comfyui_latents::LatentBatch;
use tensor_data::F32Tensor;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let samples = F32Tensor::from_dims([1, 4, 64, 64], vec![0.0; 16_384])?;
    let latents = LatentBatch::new(samples)?;
    assert_eq!(latents.image_size()?.width, 512);
    Ok(())
}
```

## Related crates

- `tensor-data`
- `comfyui-data`
- `comfyui-models`
