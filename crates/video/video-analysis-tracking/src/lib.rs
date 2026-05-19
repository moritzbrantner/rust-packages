#![doc = include_str!("../README.md")]

use std::collections::{BTreeMap, BTreeSet};

use video_analysis_core::{
    BoundingBox, DetectError, FramePosition, Observation, ObservationKind, Result, VideoAnalyzer,
    VideoFrame,
};

#[derive(Debug, Clone, PartialEq)]
/// Data type for tracked detection.
pub struct TrackedDetection {
    /// The kind value.
    pub kind: ObservationKind,
    /// The region value.
    pub region: BoundingBox,
    /// Label assigned to this value.
    pub label: Option<String>,
    /// Score assigned to this value.
    pub score: Option<f32>,
    /// The track hint value.
    pub track_hint: Option<String>,
    /// The attributes value.
    pub attributes: BTreeMap<String, String>,
}

impl TrackedDetection {
    /// Creates a new value.
    pub fn new(region: BoundingBox) -> Self {
        Self {
            kind: ObservationKind::Object,
            region,
            label: None,
            score: None,
            track_hint: None,
            attributes: BTreeMap::new(),
        }
    }

    /// Returns kind.
    pub fn kind(mut self, kind: ObservationKind) -> Self {
        self.kind = kind;
        self
    }

    /// Returns label.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Returns score.
    pub fn score(mut self, score: f32) -> Self {
        self.score = Some(score);
        self
    }

    /// Returns track hint.
    pub fn track_hint(mut self, track_hint: impl Into<String>) -> Self {
        self.track_hint = Some(track_hint.into());
        self
    }

    /// Returns attribute.
    pub fn attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Options for object collision detection.
pub struct CollisionOptions {
    /// Minimum IoU required for two overlapping objects to count as a collision.
    pub min_iou: f32,
}

impl Default for CollisionOptions {
    fn default() -> Self {
        Self { min_iou: 0.0 }
    }
}

impl CollisionOptions {
    /// Validates this value.
    pub fn validate(self) -> Result<()> {
        if !self.min_iou.is_finite() || !(0.0..=1.0).contains(&self.min_iou) {
            return Err(DetectError::InvalidArgument(
                "collision min_iou must be finite and between 0.0 and 1.0".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for a pair of colliding objects.
pub struct ObjectCollision {
    /// Index of the first object in the input collection.
    pub left_index: usize,
    /// Index of the second object in the input collection.
    pub right_index: usize,
    /// Track identifier for the first object, when available.
    pub left_id: Option<String>,
    /// Track identifier for the second object, when available.
    pub right_id: Option<String>,
    /// Region of the first object.
    pub left_region: BoundingBox,
    /// Region of the second object.
    pub right_region: BoundingBox,
    /// Overlapping region shared by both objects.
    pub intersection: BoundingBox,
    /// Intersection-over-union score for the two regions.
    pub iou: f32,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for object track.
pub struct ObjectTrack {
    /// Identifier for this value.
    pub id: String,
    /// The kind value.
    pub kind: ObservationKind,
    /// The region value.
    pub region: BoundingBox,
    /// Label assigned to this value.
    pub label: Option<String>,
    /// Score assigned to this value.
    pub score: Option<f32>,
    /// The first position value.
    pub first_position: FramePosition,
    /// The last position value.
    pub last_position: FramePosition,
    /// The age frames value.
    pub age_frames: u64,
    /// The missed frames value.
    pub missed_frames: u64,
    /// The attributes value.
    pub attributes: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Data type for tracking options.
pub struct TrackingOptions {
    /// The min IoU value.
    pub min_iou: f32,
    /// The max missed frames value.
    pub max_missed_frames: u64,
    /// The min score value.
    pub min_score: Option<f32>,
}

impl Default for TrackingOptions {
    fn default() -> Self {
        Self {
            min_iou: 0.3,
            max_missed_frames: 15,
            min_score: None,
        }
    }
}

impl TrackingOptions {
    /// Validates this value.
    pub fn validate(self) -> Result<()> {
        if !self.min_iou.is_finite() || !(0.0..=1.0).contains(&self.min_iou) {
            return Err(DetectError::InvalidArgument(
                "tracking min_iou must be finite and between 0.0 and 1.0".to_string(),
            ));
        }
        if let Some(min_score) = self.min_score {
            if !min_score.is_finite() {
                return Err(DetectError::InvalidArgument(
                    "tracking min_score must be finite".to_string(),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
/// Data type for IoU tracker.
pub struct IouTracker {
    options: TrackingOptions,
    tracks: BTreeMap<String, ObjectTrack>,
    next_id: u64,
}

impl IouTracker {
    /// Creates a new value.
    pub fn new(options: TrackingOptions) -> Result<Self> {
        options.validate()?;
        Ok(Self {
            options,
            tracks: BTreeMap::new(),
            next_id: 1,
        })
    }

    /// Returns options.
    pub fn options(&self) -> TrackingOptions {
        self.options
    }

    /// Returns tracks.
    pub fn tracks(&self) -> impl Iterator<Item = &ObjectTrack> {
        self.tracks.values()
    }

    /// Returns collisions between active tracks.
    pub fn collisions(&self, options: CollisionOptions) -> Result<Vec<ObjectCollision>> {
        detect_track_collisions(self.tracks.values(), options)
    }

    /// Returns update.
    pub fn update(
        &mut self,
        position: FramePosition,
        detections: impl IntoIterator<Item = TrackedDetection>,
    ) -> Result<Vec<ObjectTrack>> {
        self.options.validate()?;
        let detections = detections
            .into_iter()
            .filter(|detection| {
                self.options
                    .min_score
                    .zip(detection.score)
                    .map(|(minimum, score)| score >= minimum)
                    .unwrap_or(true)
            })
            .collect::<Vec<_>>();

        let active_ids = self.tracks.keys().cloned().collect::<Vec<_>>();
        let mut assignments = vec![None::<String>; detections.len()];
        let mut used_tracks = BTreeSet::new();

        for (index, detection) in detections.iter().enumerate() {
            let Some(hint) = &detection.track_hint else {
                continue;
            };
            let Some(track) = self.tracks.get(hint) else {
                continue;
            };
            if !used_tracks.contains(hint) && compatible(track, detection) {
                assignments[index] = Some(hint.clone());
                used_tracks.insert(hint.clone());
            }
        }

        for (index, detection) in detections.iter().enumerate() {
            if assignments[index].is_some() {
                continue;
            }
            let mut best = None;
            for track_id in &active_ids {
                if used_tracks.contains(track_id) {
                    continue;
                }
                let Some(track) = self.tracks.get(track_id) else {
                    continue;
                };
                if !compatible(track, detection) {
                    continue;
                }
                let iou = bbox_iou(track.region, detection.region);
                if iou >= self.options.min_iou
                    && best
                        .as_ref()
                        .map(|(_, best_iou)| iou > *best_iou)
                        .unwrap_or(true)
                {
                    best = Some((track_id.clone(), iou));
                }
            }
            if let Some((track_id, _)) = best {
                assignments[index] = Some(track_id.clone());
                used_tracks.insert(track_id);
            }
        }

        let assigned = assignments
            .iter()
            .filter_map(|assignment| assignment.as_ref())
            .cloned()
            .collect::<BTreeSet<_>>();
        for track_id in active_ids {
            if !assigned.contains(&track_id) {
                if let Some(track) = self.tracks.get_mut(&track_id) {
                    track.missed_frames += 1;
                }
            }
        }

        let mut visible = Vec::new();
        for (detection, assignment) in detections.into_iter().zip(assignments) {
            let track_id = assignment.unwrap_or_else(|| self.allocate_track_id());
            let track = self
                .tracks
                .entry(track_id.clone())
                .or_insert_with(|| ObjectTrack {
                    id: track_id.clone(),
                    kind: detection.kind.clone(),
                    region: detection.region,
                    label: detection.label.clone(),
                    score: detection.score,
                    first_position: position,
                    last_position: position,
                    age_frames: 0,
                    missed_frames: 0,
                    attributes: BTreeMap::new(),
                });
            track.kind = detection.kind;
            track.region = detection.region;
            track.label = detection.label;
            track.score = detection.score;
            track.last_position = position;
            track.age_frames += 1;
            track.missed_frames = 0;
            track.attributes = detection.attributes;
            visible.push(track.clone());
        }

        self.tracks
            .retain(|_, track| track.missed_frames <= self.options.max_missed_frames);
        Ok(visible)
    }

    /// Returns reset.
    pub fn reset(&mut self) {
        self.tracks.clear();
        self.next_id = 1;
    }

    fn allocate_track_id(&mut self) -> String {
        let id = format!("track-{}", self.next_id);
        self.next_id += 1;
        id
    }
}

/// Trait for object detection backend implementations.
pub trait ObjectDetectionBackend {
    /// Returns detect frame.
    fn detect_frame(&mut self, frame: &VideoFrame<'_>) -> Result<Vec<TrackedDetection>>;
}

/// Data type for object tracking analyzer.
pub struct ObjectTrackingAnalyzer<B> {
    name: String,
    backend: B,
    tracker: IouTracker,
}

impl<B> ObjectTrackingAnalyzer<B> {
    /// Creates a new value.
    pub fn new(name: impl Into<String>, backend: B, options: TrackingOptions) -> Result<Self> {
        Ok(Self {
            name: name.into(),
            backend,
            tracker: IouTracker::new(options)?,
        })
    }

    /// Returns backend.
    pub fn backend(&self) -> &B {
        &self.backend
    }

    /// Returns backend mut.
    pub fn backend_mut(&mut self) -> &mut B {
        &mut self.backend
    }

    /// Returns tracker.
    pub fn tracker(&self) -> &IouTracker {
        &self.tracker
    }

    /// Returns tracker mut.
    pub fn tracker_mut(&mut self) -> &mut IouTracker {
        &mut self.tracker
    }
}

impl<B: ObjectDetectionBackend> VideoAnalyzer for ObjectTrackingAnalyzer<B> {
    fn name(&self) -> &str {
        &self.name
    }

    fn process_frame(&mut self, frame: &VideoFrame<'_>) -> Result<Vec<Observation>> {
        let detections = self.backend.detect_frame(frame)?;
        let tracks = self.tracker.update(frame.position, detections)?;
        Ok(tracks
            .into_iter()
            .map(|track| observation_for_track(self.name(), track))
            .collect())
    }
}

/// Returns bounding box intersection.
pub fn bbox_intersection(left: BoundingBox, right: BoundingBox) -> Option<BoundingBox> {
    let left_x1 = left.x.saturating_add(left.width);
    let left_y1 = left.y.saturating_add(left.height);
    let right_x1 = right.x.saturating_add(right.width);
    let right_y1 = right.y.saturating_add(right.height);

    let ix0 = left.x.max(right.x);
    let iy0 = left.y.max(right.y);
    let ix1 = left_x1.min(right_x1);
    let iy1 = left_y1.min(right_y1);
    if ix1 <= ix0 || iy1 <= iy0 {
        return None;
    }

    BoundingBox::new(ix0, iy0, ix1 - ix0, iy1 - iy0).ok()
}

/// Returns whether two bounding boxes overlap.
pub fn bbox_intersects(left: BoundingBox, right: BoundingBox) -> bool {
    bbox_intersection(left, right).is_some()
}

/// Returns bbox IoU.
pub fn bbox_iou(left: BoundingBox, right: BoundingBox) -> f32 {
    let Some(intersection) = bbox_intersection(left, right) else {
        return 0.0;
    };
    let intersection = intersection.width as f32 * intersection.height as f32;
    let left_area = left.width as f32 * left.height as f32;
    let right_area = right.width as f32 * right.height as f32;
    intersection / (left_area + right_area - intersection)
}

/// Returns collisions between tracked detections.
pub fn detect_detection_collisions(
    detections: &[TrackedDetection],
    options: CollisionOptions,
) -> Result<Vec<ObjectCollision>> {
    options.validate()?;
    let regions = detections
        .iter()
        .map(|detection| CollisionRegion {
            id: None,
            region: detection.region,
        })
        .collect::<Vec<_>>();
    Ok(detect_collisions(&regions, options))
}

/// Returns collisions between object tracks.
pub fn detect_track_collisions<'a>(
    tracks: impl IntoIterator<Item = &'a ObjectTrack>,
    options: CollisionOptions,
) -> Result<Vec<ObjectCollision>> {
    options.validate()?;
    let regions = tracks
        .into_iter()
        .map(|track| CollisionRegion {
            id: Some(track.id.as_str()),
            region: track.region,
        })
        .collect::<Vec<_>>();
    Ok(detect_collisions(&regions, options))
}

fn compatible(track: &ObjectTrack, detection: &TrackedDetection) -> bool {
    track.kind == detection.kind
        && match (&track.label, &detection.label) {
            (Some(left), Some(right)) => left == right,
            _ => true,
        }
}

#[derive(Debug, Clone, Copy)]
struct CollisionRegion<'a> {
    id: Option<&'a str>,
    region: BoundingBox,
}

fn detect_collisions(
    regions: &[CollisionRegion<'_>],
    options: CollisionOptions,
) -> Vec<ObjectCollision> {
    let mut collisions = Vec::new();
    for left_index in 0..regions.len() {
        for right_index in (left_index + 1)..regions.len() {
            let left = regions[left_index];
            let right = regions[right_index];
            let Some(intersection) = bbox_intersection(left.region, right.region) else {
                continue;
            };
            let iou = bbox_iou(left.region, right.region);
            if iou < options.min_iou {
                continue;
            }
            collisions.push(ObjectCollision {
                left_index,
                right_index,
                left_id: left.id.map(str::to_string),
                right_id: right.id.map(str::to_string),
                left_region: left.region,
                right_region: right.region,
                intersection,
                iou,
            });
        }
    }
    collisions
}

fn observation_for_track(analyzer: &str, track: ObjectTrack) -> Observation {
    let mut observation = Observation::new(analyzer, track.kind)
        .at_frame(track.last_position)
        .region(track.region)
        .track_id(track.id)
        .attribute("track_age_frames", track.age_frames.to_string())
        .attribute("track_missed_frames", track.missed_frames.to_string())
        .attribute(
            "track_first_frame",
            track.first_position.frame_index.to_string(),
        );
    if let Some(label) = track.label {
        observation = observation.label(label);
    }
    if let Some(score) = track.score {
        observation = observation.score(score);
    }
    for (key, value) in track.attributes {
        observation = observation.attribute(key, value);
    }
    observation
}

#[cfg(test)]
mod tests {
    use num_rational::Rational64;
    use video_analysis_core::{FramePosition, OwnedVideoFrame, PixelFormat};

    use super::*;

    fn position(frame_index: u64) -> FramePosition {
        FramePosition::from_frame_index(frame_index, Rational64::new(30, 1))
    }

    fn frame(frame_index: u64) -> OwnedVideoFrame {
        OwnedVideoFrame {
            position: position(frame_index),
            width: 64,
            height: 64,
            pixel_format: PixelFormat::Rgb24,
            data: vec![0; 64 * 64 * 3],
            stride: 64 * 3,
        }
    }

    #[test]
    fn tracker_keeps_id_for_overlapping_detections() {
        let mut tracker = IouTracker::new(TrackingOptions::default()).unwrap();

        let first = tracker
            .update(
                position(0),
                [
                    TrackedDetection::new(BoundingBox::new(10, 10, 20, 20).unwrap())
                        .label("person"),
                ],
            )
            .unwrap();
        let second = tracker
            .update(
                position(1),
                [
                    TrackedDetection::new(BoundingBox::new(12, 10, 20, 20).unwrap())
                        .label("person"),
                ],
            )
            .unwrap();

        assert_eq!(first[0].id, second[0].id);
        assert_eq!(second[0].age_frames, 2);
    }

    #[test]
    fn tracker_uses_new_id_for_incompatible_label() {
        let mut tracker = IouTracker::new(TrackingOptions::default()).unwrap();

        let first = tracker
            .update(
                position(0),
                [
                    TrackedDetection::new(BoundingBox::new(10, 10, 20, 20).unwrap())
                        .label("person"),
                ],
            )
            .unwrap();
        let second = tracker
            .update(
                position(1),
                [TrackedDetection::new(BoundingBox::new(11, 11, 20, 20).unwrap()).label("car")],
            )
            .unwrap();

        assert_ne!(first[0].id, second[0].id);
    }

    #[test]
    fn detects_collisions_between_overlapping_detections() {
        let detections = [
            TrackedDetection::new(BoundingBox::new(0, 0, 10, 10).unwrap()).label("person"),
            TrackedDetection::new(BoundingBox::new(5, 4, 10, 10).unwrap()).label("bike"),
            TrackedDetection::new(BoundingBox::new(30, 30, 4, 4).unwrap()).label("car"),
        ];

        let collisions =
            detect_detection_collisions(&detections, CollisionOptions::default()).unwrap();

        assert_eq!(collisions.len(), 1);
        assert_eq!(collisions[0].left_index, 0);
        assert_eq!(collisions[0].right_index, 1);
        assert_eq!(
            collisions[0].intersection,
            BoundingBox::new(5, 4, 5, 6).unwrap()
        );
        assert!(collisions[0].iou > 0.0);
    }

    #[test]
    fn filters_collisions_by_iou() {
        let detections = [
            TrackedDetection::new(BoundingBox::new(0, 0, 10, 10).unwrap()),
            TrackedDetection::new(BoundingBox::new(9, 9, 10, 10).unwrap()),
        ];

        let collisions =
            detect_detection_collisions(&detections, CollisionOptions { min_iou: 0.1 }).unwrap();

        assert!(collisions.is_empty());
    }

    #[test]
    fn tracker_reports_collisions_between_active_tracks() {
        let mut tracker = IouTracker::new(TrackingOptions::default()).unwrap();
        tracker
            .update(
                position(0),
                [
                    TrackedDetection::new(BoundingBox::new(0, 0, 12, 12).unwrap())
                        .track_hint("left"),
                    TrackedDetection::new(BoundingBox::new(6, 0, 12, 12).unwrap())
                        .track_hint("right"),
                ],
            )
            .unwrap();

        let collisions = tracker.collisions(CollisionOptions::default()).unwrap();

        assert_eq!(collisions.len(), 1);
        assert_eq!(collisions[0].left_id.as_deref(), Some("track-1"));
        assert_eq!(collisions[0].right_id.as_deref(), Some("track-2"));
    }

    #[test]
    fn analyzer_emits_track_observations() {
        struct Backend;

        impl ObjectDetectionBackend for Backend {
            fn detect_frame(&mut self, _frame: &VideoFrame<'_>) -> Result<Vec<TrackedDetection>> {
                Ok(vec![TrackedDetection::new(
                    BoundingBox::new(1, 2, 10, 12).unwrap(),
                )
                .label("person")
                .score(0.9)])
            }
        }

        let mut analyzer =
            ObjectTrackingAnalyzer::new("tracker", Backend, TrackingOptions::default()).unwrap();
        let observations = analyzer.process_frame(&frame(0).as_frame()).unwrap();

        assert_eq!(observations.len(), 1);
        assert_eq!(observations[0].kind, ObservationKind::Object);
        assert_eq!(observations[0].track_id.as_deref(), Some("track-1"));
        assert_eq!(observations[0].label.as_deref(), Some("person"));
    }
}
