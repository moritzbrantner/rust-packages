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

## Related crates

- `three-d-processing-core`
- `three-d-processing-mesh`
