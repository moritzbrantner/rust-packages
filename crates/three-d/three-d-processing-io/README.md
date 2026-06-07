# three-d-processing-io

OBJ, PLY, and minimal embedded glTF mesh I/O plus PLY point-cloud I/O for `moritzbrantner-video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use three_d_processing_io::{read_obj_mesh, write_ply_mesh};

let mesh = read_obj_mesh("mesh.obj")?;
write_ply_mesh("mesh.ply", &mesh)?;
```

## Package surface

Primary workflow: `threeD.io.supportedFormats`.

Workflow operations:

- `threeD.io.supportedFormats`: List mesh formats supported by the pure IO helpers.
- `threeD.io.objPreview`: Encode a mesh as OBJ and return a capped string preview.
- `threeD.io.plyPreview`: Encode a mesh as ASCII PLY and return a capped string preview.

Debug operations:

- `describe`: inspect package metadata and runtime support.

Runtime support: library, CLI, server, and WASM wrappers expose these operations.

Run the primary workflow through the CLI:

```bash
cargo run -p moritzbrantner-three-d-processing-io-cli -- run \
  --operation threeD.io.supportedFormats \
  --json '{}'
```

Successful responses use the shared package-surface shape with `operation`,
`title`, `message`, `summary`, and `result`. Default surface calls are
deterministic, local-first, and do not download models, write persistent files,
or execute external tools unless an operation explicitly documents native or
external-tool execution.

## Related crates

- `three-d-processing-core`
- `three-d-processing-mesh`
