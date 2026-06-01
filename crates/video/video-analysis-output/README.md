# video-analysis-output

CSV and HTML report helpers for `moritzbrantner-video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use video_analysis_output::write_scene_list_csv;

let mut bytes = Vec::new();
write_scene_list_csv(&mut bytes, &[])?;

let _ = bytes;
```

## Related crates

- `video-analysis-core`
- `video-analysis-features`
- `@moritzbrantner/video-analysis-ui`
