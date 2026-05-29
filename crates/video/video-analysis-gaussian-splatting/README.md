# video-analysis-gaussian-splatting

Gaussian splatting primitives and CPU compositing for `video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use video_analysis_gaussian_splatting::GaussianPrimitive;

let primitive = GaussianPrimitive::default();
let _ = primitive;
```

## Related crates

- `video-analysis-radiance-fields`
- `video-analysis-radiance-io`

## Package surface

Workflow operations:

- `video.splat.summary`

Debug operations:

- `describe`
- `video.splat.cameraPlan`
- `video.splat.trainingPlan`

Runtime limits:

Operations are deterministic, local-first, and side-effect free. They return inline JSON reports and do not download models, write files, or run native tools.

Invalid input returns a clear error through `run_surface_operation`; successful
responses include `operation`, `title`, `message`, `summary`, and `result` while
keeping existing top-level domain fields for compatibility.
