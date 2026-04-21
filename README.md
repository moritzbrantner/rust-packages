# Rust Video Analysis Packages

This workspace contains a Rust-first reimplementation of PySceneDetect-style video scene analysis.
The vendored `references/pyscenedetect` directory is used only as an upstream behavior reference.

## Crates

- `video-analysis`: umbrella re-export crate.
- `video-analysis-core`: timecodes, frames, scene types, metrics, detector trait, and pipeline.
- `video-analysis-detectors`: content, adaptive, threshold, histogram, and perceptual hash detectors.
- `video-analysis-ffmpeg`: FFmpeg-backed raw RGB video source.
- `video-analysis-output`: scene/stats CSV and simple HTML output helpers.
- `video-analysis-split`: ffmpeg CLI based scene splitting.
- `video-analysis-cli`: `vanalyze` command-line tool.

## Example

```bash
cargo run -p video-analysis-cli -- detect --input video.mp4 --detector content --output scenes.csv --stats stats.csv
cargo run -p video-analysis-cli -- list --input video.mp4 --detector adaptive
cargo run -p video-analysis-cli -- split --input video.mp4 --detector content --output-dir scenes
```

The default test suite does not require FFmpeg to be installed.

