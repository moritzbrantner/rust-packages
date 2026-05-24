# comfyui-models

ComfyUI model folder, inventory, and extra path contracts.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use comfyui_models::{scan_model_inventory, write_extra_model_paths_yaml};

let inventory = scan_model_inventory("ComfyUI/models")?;
write_extra_model_paths_yaml("extra_model_paths.yaml", &inventory)?;
```

## Related crates

- `comfyui-data`
- `model-runtime`
