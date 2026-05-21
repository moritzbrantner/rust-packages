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

## Config files

If `vanalyze.conf` or `video-analysis-cli.conf` is present in the current
directory, the CLI reads it automatically as shell-style arguments before the
real command line. Direct CLI flags still win when the same option is set in
both places. Use `--config path/to/file.conf` to read a specific config file
instead of the current directory default.

## Related crates

- `video-analysis-core`
- `video-analysis-output`
- `video-analysis-use-cases`
