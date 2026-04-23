# video-analysis-reconstruction

Sparse reconstruction helpers for `video-analysis`.

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
