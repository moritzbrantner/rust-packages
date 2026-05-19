# video-analysis-opencv-backend

Optional OpenCV backend contracts for COLMAP-like `video-analysis` pipelines.

The crate is intentionally feature-gated so the workspace can compile without a
native OpenCV installation. Enabling native OpenCV execution can be implemented
behind the `opencv-backend` feature without changing the public SfM/MVS traits.

## Feature flags

- `opencv-backend`: reserved for native OpenCV-backed implementations.

## Related crates

- `video-analysis-sfm`
- `video-analysis-mvs`
