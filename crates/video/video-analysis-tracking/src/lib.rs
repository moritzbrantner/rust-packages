#![doc = include_str!("../README.md")]

use std::collections::{BTreeMap, BTreeSet};

use video_analysis_core::{
    BoundingBox, DetectError, FramePosition, Observation, ObservationKind, Result, VideoAnalyzer,
    VideoFrame,
};

#[derive(Debug, Clone, PartialEq)]
pub struct TrackedDetection {
    pub kind: ObservationKind,
    pub region: BoundingBox,
    pub label: Option<String>,
    pub score: Option<f32>,
    pub track_hint: Option<String>,
    pub attributes: BTreeMap<String, String>,
}

impl TrackedDetection {
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

    pub fn kind(mut self, kind: ObservationKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn score(mut self, score: f32) -> Self {
        self.score = Some(score);
        self
    }

    pub fn track_hint(mut self, track_hint: impl Into<String>) -> Self {
        self.track_hint = Some(track_hint.into());
        self
    }

    pub fn attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectTrack {
    pub id: String,
    pub kind: ObservationKind,
    pub region: BoundingBox,
    pub label: Option<String>,
    pub score: Option<f32>,
    pub first_position: FramePosition,
    pub last_position: FramePosition,
    pub age_frames: u64,
    pub missed_frames: u64,
    pub attributes: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrackingOptions {
    pub min_iou: f32,
    pub max_missed_frames: u64,
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
pub struct IouTracker {
    options: TrackingOptions,
    tracks: BTreeMap<String, ObjectTrack>,
    next_id: u64,
}

impl IouTracker {
    pub fn new(options: TrackingOptions) -> Result<Self> {
        options.validate()?;
        Ok(Self {
            options,
            tracks: BTreeMap::new(),
            next_id: 1,
        })
    }

    pub fn options(&self) -> TrackingOptions {
        self.options
    }

    pub fn tracks(&self) -> impl Iterator<Item = &ObjectTrack> {
        self.tracks.values()
    }

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

pub trait ObjectDetectionBackend {
    fn detect_frame(&mut self, frame: &VideoFrame<'_>) -> Result<Vec<TrackedDetection>>;
}

pub struct ObjectTrackingAnalyzer<B> {
    name: String,
    backend: B,
    tracker: IouTracker,
}

impl<B> ObjectTrackingAnalyzer<B> {
    pub fn new(name: impl Into<String>, backend: B, options: TrackingOptions) -> Result<Self> {
        Ok(Self {
            name: name.into(),
            backend,
            tracker: IouTracker::new(options)?,
        })
    }

    pub fn backend(&self) -> &B {
        &self.backend
    }

    pub fn backend_mut(&mut self) -> &mut B {
        &mut self.backend
    }

    pub fn tracker(&self) -> &IouTracker {
        &self.tracker
    }

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

pub fn bbox_iou(left: BoundingBox, right: BoundingBox) -> f32 {
    let left_x1 = left.x.saturating_add(left.width);
    let left_y1 = left.y.saturating_add(left.height);
    let right_x1 = right.x.saturating_add(right.width);
    let right_y1 = right.y.saturating_add(right.height);

    let ix0 = left.x.max(right.x);
    let iy0 = left.y.max(right.y);
    let ix1 = left_x1.min(right_x1);
    let iy1 = left_y1.min(right_y1);
    if ix1 <= ix0 || iy1 <= iy0 {
        return 0.0;
    }

    let intersection = (ix1 - ix0) as f32 * (iy1 - iy0) as f32;
    let left_area = left.width as f32 * left.height as f32;
    let right_area = right.width as f32 * right.height as f32;
    intersection / (left_area + right_area - intersection)
}

fn compatible(track: &ObjectTrack, detection: &TrackedDetection) -> bool {
    track.kind == detection.kind
        && match (&track.label, &detection.label) {
            (Some(left), Some(right)) => left == right,
            _ => true,
        }
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
