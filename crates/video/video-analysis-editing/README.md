# video-analysis-editing

CPU video frame editing primitives for `video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use video_analysis_editing::{FrameEdit, FrameEditor};

let editor = FrameEditor::new(vec![FrameEdit::Grayscale, FrameEdit::Invert]);
let _ = editor;
```

## Related crates

- `video-analysis-core`
- `image-analysis-processing`
