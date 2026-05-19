# video-analysis-sfm-rust-backend

Rust-native SfM backend adapters for `video-analysis`.

This crate keeps the native Rust implementation behind the same
`video-analysis-sfm` traits used by COLMAP/OpenCV-compatible backends. The first
backend is a known-pose sparse mapper that consumes pre-extracted binary
features and pairwise matches.

## Feature flags

- No optional feature flags today.

## Related crates

- `video-analysis-sfm`
- `video-analysis-reconstruction`
