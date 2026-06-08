# video-analysis-sfm

Structure-from-Motion backend contracts and sparse pipeline orchestration for
`moritzbrantner-video-analysis`.

This crate defines the stable Rust API for COLMAP-like sparse reconstruction.
It also owns the SfM provider adapters that are useful package surfaces without
being durable standalone crates.

## Modules

- `colmap`: COLMAP text baseline loading, parity comparison, command specs, and
  the `ColmapTextBackend` adapter.
- `colmap_scene`: browser-friendly sparse scene summaries and camera/point
  views for parsed COLMAP text models.
- `colmap_reconstruct_video`: explicit native server-only COLMAP video
  reconstruction workflow and deterministic command-plan request defaults.
- `rust_backend`: deterministic Rust known-pose SfM backend.
- `opencv`: unavailable OpenCV SfM placeholder and capability planning types.

## Feature flags

- No optional feature flags today.

## Related crates

- `moritzbrantner-video-analysis-reconstruction`
- `moritzbrantner-video-analysis-radiance-fields`
- `moritzbrantner-video-analysis-radiance-io`

## Package surface

Workflow operations:

- `video.sfm.matchPlan`
- `video.sfm.cameraGraph`
- `video.colmap.reconstructVideo` (server-only)

Debug operations:

- `describe`
- `video.sfm.reconstructionSummary`
- `video.colmap.commandPlan`
- `video.colmap.imageList`
- `video.colmap.sparseSummary`
- `video.sfmRust.matchSummary`
- `video.sfmRust.trackPlan`
- `video.sfmRust.bundlePlan`
- `video.opencv.sfmPlan`

Runtime limits:

Default operations are deterministic, local-first, and side-effect free. They
return inline JSON reports and do not download models, write files, or run native
tools. `video.colmap.reconstructVideo` is native-server-only and validates local
workspace paths before invoking `ffmpeg` or `colmap`.

Invalid input returns a clear error through `run_surface_operation`; successful
responses include `operation`, `title`, `message`, `summary`, and `result` while
keeping existing top-level domain fields for compatibility.
