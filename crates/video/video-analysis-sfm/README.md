# video-analysis-sfm

Structure-from-Motion backend contracts and sparse pipeline orchestration for
`moritzbrantner-video-analysis`.

This crate defines the stable Rust API for COLMAP-like sparse reconstruction.
Concrete backends can wrap native engines such as COLMAP/OpenCV or provide a
Rust-native implementation while returning the same `SparseReconstruction` data
model.

## Feature flags

- No optional feature flags today.

## Related crates

- `moritzbrantner-video-analysis-reconstruction`
- `moritzbrantner-video-analysis-colmap-backend`
- `moritzbrantner-video-analysis-opencv-backend`
- `moritzbrantner-video-analysis-sfm-rust-backend`

## Package surface

Workflow operations:

- `video.sfm.matchPlan`
- `video.sfm.cameraGraph`

Debug operations:

- `describe`
- `video.sfm.reconstructionSummary`

Runtime limits:

Operations are deterministic, local-first, and side-effect free. They return inline JSON reports and do not download models, write files, or run native tools.

Invalid input returns a clear error through `run_surface_operation`; successful
responses include `operation`, `title`, `message`, `summary`, and `result` while
keeping existing top-level domain fields for compatibility.
