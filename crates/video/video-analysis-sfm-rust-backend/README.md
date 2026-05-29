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

## Package surface

Workflow operations:

- `video.sfmRust.matchSummary`

Debug operations:

- `describe`
- `video.sfmRust.trackPlan`
- `video.sfmRust.bundlePlan`

Runtime limits:

Operations are deterministic, local-first, and side-effect free. They return inline JSON reports and do not download models, write files, or run native tools.

Invalid input returns a clear error through `run_surface_operation`; successful
responses include `operation`, `title`, `message`, `summary`, and `result` while
keeping existing top-level domain fields for compatibility.
