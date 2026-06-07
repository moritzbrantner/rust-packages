# three-d-processing-mesh

Triangle mesh validation and geometry algorithms for `moritzbrantner-video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use three_d_processing_mesh::Mesh;

let mesh = Mesh::new(vec![], vec![])?;
let _ = mesh.bounds()?;
let _ = mesh.surface_centroid()?;
```

## Package surface

Primary workflow: `threeD.mesh.diagnostics`.

Workflow operations:

- `threeD.mesh.diagnostics`: Returns mesh diagnostics, topology counts, area, bounds, and manifold flags.
- `threeD.mesh.repairPreview`: Runs in-memory degenerate-triangle removal and vertex welding previews.
- `threeD.mesh.sample`: Samples deterministic points from mesh surface with a capped sample count.

Debug operations:

- `describe`: inspect package metadata and runtime support.

Runtime support: library, CLI, server, and WASM wrappers expose these operations.

Run the primary workflow through the CLI:

```bash
cargo run -p moritzbrantner-three-d-processing-mesh-cli -- run \
  --operation threeD.mesh.diagnostics \
  --json '{"mesh":{"triangles":[[0,1,2]],"vertices":[[0,0,0],[1,0,0],[0,1,0]]}}'
```

Successful responses use the shared package-surface shape with `operation`,
`title`, `message`, `summary`, and `result`. Default surface calls are
deterministic, local-first, and do not download models, write persistent files,
or execute external tools unless an operation explicitly documents native or
external-tool execution.

## Related crates

- `three-d-processing-core`
- `video-analysis-gaussian-splatting`
