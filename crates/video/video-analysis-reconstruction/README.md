# video-analysis-reconstruction

Sparse reconstruction helpers for `moritzbrantner-video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use video_analysis_reconstruction::SparseReconstruction;

let reconstruction = SparseReconstruction::default();
let _ = reconstruction.export_ply("points.ply")?;
```

## Related crates

- `video-analysis-radiance-fields`
- `video-analysis-radiance-io`

## Package surface

Workflow operations:

- `video.reconstruction.plan`

Debug operations:

- `describe`
- `video.reconstruction.cameraSummary`
- `video.reconstruction.assetSummary`

Runtime limits:

Operations are deterministic, local-first, and side-effect free. They return inline JSON reports and do not download models, write files, or run native tools.

Invalid input returns a clear error through `run_surface_operation`; successful
responses include `operation`, `title`, `message`, `summary`, and `result` while
keeping existing top-level domain fields for compatibility.
