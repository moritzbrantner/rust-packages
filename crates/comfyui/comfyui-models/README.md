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

## Package surface

Primary workflow: `comfy.models.defaults`.

Workflow operations:

- `comfy.models.defaults`: Lists ComfyUI core model folder keys and default relative paths.
- `comfy.models.reference`: Builds a typed ComfyUI model reference from role and name.

Debug operations:

- `describe`: inspect package metadata and runtime support.
- `comfy.models.extraPathsPlan`: Builds an extra_model_paths YAML plan without writing files.

Runtime support: library, CLI, server, and WASM wrappers expose these operations.

Run the primary workflow through the CLI:

```bash
cargo run -p moritzbrantner-comfyui-models-cli -- run \
  --operation comfy.models.defaults \
  --json '{}'
```

Successful responses use the shared package-surface shape with `operation`,
`title`, `message`, `summary`, and `result`. Default surface calls are
deterministic, local-first, and do not download models, write persistent files,
or execute external tools unless an operation explicitly documents native or
external-tool execution.

## Related crates

- `comfyui-data`
- `model-runtime`
