# comfyui-data

Serde contracts for ComfyUI workflow and prompt data.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use comfyui_data::Workflow;

let workflow = Workflow::from_json_str(include_str!("../../../../tests/fixtures/comfyui-workflow.json"))?;
let _ = workflow.validate_links()?;
```

## Package surface

Primary workflow: `comfy.workflow.validate`.

Workflow operations:

- `comfy.workflow.validate`: Validates ComfyUI workflow node and link references.
- `comfy.prompt.links`: Lists parsed ComfyUI prompt graph input links and invalid link-shaped values.

Debug operations:

- `describe`: inspect package metadata and runtime support.
- `comfy.workflow.inventory`: Returns normalized workflow socket inventory by input, output, and link position.

Runtime support: library, CLI, server, and WASM wrappers expose these operations.

Run the primary workflow through the CLI:

```bash
cargo run -p moritzbrantner-comfyui-data-cli -- run \
  --operation comfy.workflow.validate \
  --json '{"workflow":{"links":[],"nodes":[]}}'
```

Successful responses use the shared package-surface shape with `operation`,
`title`, `message`, `summary`, and `result`. Default surface calls are
deterministic, local-first, and do not download models, write persistent files,
or execute external tools unless an operation explicitly documents native or
external-tool execution.

## Related crates

- `comfyui-models`
- `video-analysis-test-support`
