# video-analysis-sfm

Structure-from-Motion backend contracts and sparse pipeline orchestration for
`video-analysis`.

This crate defines the stable Rust API for COLMAP-like sparse reconstruction.
Concrete backends can wrap native engines such as COLMAP/OpenCV or provide a
Rust-native implementation while returning the same `SparseReconstruction` data
model.

## Feature flags

- No optional feature flags today.

## Related crates

- `video-analysis-reconstruction`
- `video-analysis-colmap-backend`
- `video-analysis-opencv-backend`
- `video-analysis-sfm-rust-backend`
