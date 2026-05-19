# video-analysis-mvs

Multi-View Stereo backend contracts and dense reconstruction outputs for
`video-analysis`.

This crate normalizes dense reconstruction outputs to workspace point-cloud and
mesh types so native COLMAP/OpenCV-style backends and Rust-native backends can
share one public API.

## Feature flags

- No optional feature flags today.

## Related crates

- `three-d-processing-core`
- `three-d-processing-mesh`
- `video-analysis-reconstruction`
