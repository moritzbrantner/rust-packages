# video-analysis-cli

Command-line entry point for `video-analysis` workflows.

## Feature flags

- `onnx`: enables the ONNX command surface
- `onnxruntime`: enables runtime-backed ONNX execution
- `ffmpeg-tests`: extra FFmpeg-oriented test coverage

## Example

```bash
cargo run -p video-analysis-cli -- detect \
  --input video.mp4 \
  --output scenes.csv

cargo run -p video-analysis-cli -- models inspect \
  --manifest .video-analysis-models/yolos-tiny/main/manifest.json
```

## Related crates

- `video-analysis-core`
- `video-analysis-output`
- `video-analysis-use-cases`
