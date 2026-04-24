# video-analysis-use-cases

Runnable end-to-end workflows built from the `video-analysis` crates.

## Feature flags

- No optional feature flags today.

## Example

```bash
cargo run -p video-analysis-use-cases -- youtube-video \
  --url "https://www.youtube.com/watch?v=dQw4w9WgXcQ" \
  --work-dir use-case-output/youtube-video \
  --output use-case-output/youtube-video/analysis.json
```

## Config files

If `video-analysis-use-cases.conf` is present in the current directory, the
binary reads it automatically as shell-style arguments before the real command
line. Direct CLI flags still win when the same option is set in both places.

## Related crates

- `video-analysis-cli`
- `video-analysis-models`
- `@video-analysis/ui`
