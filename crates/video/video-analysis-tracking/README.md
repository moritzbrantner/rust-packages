# video-analysis-tracking

IoU-based object tracking for `video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use video_analysis_core::BoundingBox;
use video_analysis_tracking::{
    detect_detection_collisions, CollisionOptions, IouTracker, TrackedDetection,
};

let detections = [
    TrackedDetection::new(BoundingBox::new(0, 0, 32, 32)?),
    TrackedDetection::new(BoundingBox::new(16, 16, 32, 32)?),
];

let collisions = detect_detection_collisions(&detections, CollisionOptions::default())?;
assert_eq!(collisions.len(), 1);

let _tracker = IouTracker::new(Default::default())?;
```

## Related crates

- `video-analysis-recognition`
- `video-analysis-posture`
