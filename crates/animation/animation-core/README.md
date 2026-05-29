# animation-core

Shared timeline, keyframe, track, clip, and skeleton contracts for animation
workflows.

## Feature flags

- No optional feature flags today.

## Example

```rust,no_run
use animation_core::{Interpolation, Keyframe, Track};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let track = Track::new(
        "opacity",
        [
            Keyframe::new(0.0, 0.0)?,
            Keyframe::new(1.0, 1.0)?,
        ],
        Interpolation::Linear,
    )?;
    assert_eq!(track.sample_f32(0.5)?, Some(0.5));
    Ok(())
}
```

## Related crates

- `three-d-processing-core`
- `video-analysis-posture`

## Package surface

Workflow operations:

- `animation.timeline.summary`
- `animation.keyframes.sample`

Debug operations:

- `describe`
- `animation.easing.preview`

Runtime limits:

Operations are deterministic, local-first, and side-effect free. They return inline JSON reports and do not download models, write files, or run native tools.

Invalid input returns a clear error through `run_surface_operation`; successful
responses include `operation`, `title`, `message`, `summary`, and `result` while
keeping existing top-level domain fields for compatibility.
