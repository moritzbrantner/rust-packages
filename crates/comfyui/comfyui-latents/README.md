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

## Package surface

Primary workflow: `comfy.latents.size`.

Workflow operations:

- `comfy.latents.size`: Converts image size to ComfyUI latent dimensions.
- `comfy.latents.batchSummary`: Validates and summarizes a rank-4 latent sample tensor.
- `comfy.latents.maskCompatibility`: Checks latent mask compatibility with a latent batch.

Debug operations:

- `describe`: inspect package metadata and runtime support.

Runtime support: library, CLI, server, and WASM wrappers expose these operations.

Run the primary workflow through the CLI:

```bash
cargo run -p moritzbrantner-comfyui-latents-cli -- run \
  --operation comfy.latents.size \
  --json '{"height":512,"width":512}'
```

Successful responses use the shared package-surface shape with `operation`,
`title`, `message`, `summary`, and `result`. Default surface calls are
deterministic, local-first, and do not download models, write persistent files,
or execute external tools unless an operation explicitly documents native or
external-tool execution.

## Related crates

- `tensor-data`
- `comfyui-data`
- `comfyui-models`
