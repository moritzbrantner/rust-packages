# video-analysis-radiance-io

COLMAP, Nerfstudio, and PLY I/O for `video-analysis` radiance workflows.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use video_analysis_radiance_io::read_nerfstudio_transforms;

let transforms = read_nerfstudio_transforms("transforms.json")?;
let _ = transforms;
```

## Related crates

- `video-analysis-radiance-fields`
- `video-analysis-reconstruction`
