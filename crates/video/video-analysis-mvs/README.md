# video-analysis-mvs

Multi-View Stereo backend contracts and dense reconstruction outputs for
`moritzbrantner-video-analysis`.

This crate normalizes dense reconstruction outputs to workspace point-cloud and
mesh types so native COLMAP/OpenCV-style backends and Rust-native backends can
share one public API.

## Feature flags

- No optional feature flags today.

## Related crates

- `moritzbrantner-three-d-processing-core`
- `moritzbrantner-three-d-processing-mesh`
- `moritzbrantner-video-analysis-reconstruction`

## Package surface

Workflow operations:

- `video.mvs.depthPlan`

Debug operations:

- `describe`
- `video.mvs.fusionPlan`
- `video.mvs.outputSummary`

Runtime limits:

Operations are deterministic, local-first, and side-effect free. They return inline JSON reports and do not download models, write files, or run native tools.

Invalid input returns a clear error through `run_surface_operation`; successful
responses include `operation`, `title`, `message`, `summary`, and `result` while
keeping existing top-level domain fields for compatibility.
