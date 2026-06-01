# video-analysis-opencv-backend

Optional OpenCV backend contracts for COLMAP-like `moritzbrantner-video-analysis` pipelines.

The crate is intentionally feature-gated so the workspace can compile without a
native OpenCV installation. It does not provide a command adapter. Prefer
Rust-native color/object heuristics in `moritzbrantner-image-analysis-detection`, learned
detectors in `moritzbrantner-video-analysis-onnx`, or known-pose Rust SfM in
`moritzbrantner-video-analysis-sfm-rust-backend`.

## Feature flags

- `opencv-backend`: reserved for future native OpenCV-backed implementations.

## Related crates

- `moritzbrantner-video-analysis-sfm`
- `moritzbrantner-video-analysis-mvs`
- `moritzbrantner-video-analysis-sfm-rust-backend`
- `moritzbrantner-video-analysis-onnx`
- `moritzbrantner-image-analysis-detection`

## Package surface

Workflow operations:

- `video.opencv.capturePlan`
- `video.opencv.frameStats`

Debug operations:

- `describe`
- `video.opencv.detectorPlan`

Runtime limits:

The package surface reports deterministic OpenCV backend contracts. Native OpenCV execution remains opt-in behind `opencv-backend`.

Invalid input returns a clear error through `run_surface_operation`; successful
responses include `operation`, `title`, `message`, `summary`, and `result` while
keeping existing top-level domain fields for compatibility.
