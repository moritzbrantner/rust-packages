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

## Related crates

- `three-d-processing-core`
- `video-analysis-gaussian-splatting`
