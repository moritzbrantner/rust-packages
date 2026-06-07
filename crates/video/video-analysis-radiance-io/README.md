# video-analysis-radiance-io

COLMAP, Nerfstudio, and PLY I/O for `moritzbrantner-video-analysis` radiance workflows.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use video_analysis_radiance_io::read_nerfstudio_transforms;

let transforms = read_nerfstudio_transforms("transforms.json")?;
let _ = transforms;
```

## Package surface

Primary workflow: `radiance.io.colmapCameraSupport`.

Workflow operations:

- `radiance.io.colmapCameraSupport`: Inspect which COLMAP camera models can be converted by the current pure Rust helpers.
- `radiance.io.colmapSummary`: Summarize COLMAP camera, image, and point counts with camera support totals.
- `radiance.io.gaussianSplatSummary`: Summarize an in-memory Gaussian splat scene without reading model files.

Debug operations:

- `describe`: inspect package metadata and runtime support.

Runtime support: library, CLI, server, and WASM wrappers expose these operations.

Run the primary workflow through the CLI:

```bash
cargo run -p moritzbrantner-video-analysis-radiance-io-cli -- run \
  --operation radiance.io.colmapCameraSupport \
  --json '{"dataset":{"cameras":[{"height":480,"id":1,"params":[500,500,320,240],"rawModel":"PINHOLE","width":640}],"images":[],"points3d":[]}}'
```

Successful responses use the shared package-surface shape with `operation`,
`title`, `message`, `summary`, and `result`. Default surface calls are
deterministic, local-first, and do not download models, write persistent files,
or execute external tools unless an operation explicitly documents native or
external-tool execution.

## Related crates

- `video-analysis-radiance-fields`
- `video-analysis-reconstruction`
