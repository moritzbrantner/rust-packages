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

## Related crates

- `comfyui-models`
- `video-analysis-test-support`
