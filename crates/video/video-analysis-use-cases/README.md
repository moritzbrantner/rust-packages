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

## Related crates

- `video-analysis-cli`
- `video-analysis-models`
- `@video-analysis/ui`
