#![doc = include_str!("../README.md")]

use std::collections::BTreeMap;

use math_geometry_2d::{NormalizedPoint2, Point2f, Size2u};
use serde::{Deserialize, Serialize};
use three_d_processing_core::{LineSegment3, Point3, Vector3};
use video_analysis_core::{
    BoundingBox, DetectError, Observation, ObservationKind, Result, VideoAnalyzer, VideoFrame,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Keypoint {
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub score: Option<f32>,
    pub visible: Option<bool>,
}

impl Keypoint {
    pub fn new(name: impl Into<String>, x: f32, y: f32) -> Result<Self> {
        let keypoint = Self {
            name: name.into(),
            x,
            y,
            score: None,
            visible: None,
        };
        keypoint.validate()?;
        Ok(keypoint)
    }

    pub fn score(mut self, score: f32) -> Result<Self> {
        self.score = Some(score);
        self.validate()?;
        Ok(self)
    }

    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = Some(visible);
        self
    }

    pub fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            return Err(invalid_argument("keypoint name must not be empty"));
        }
        if !self.x.is_finite() || !self.y.is_finite() {
            return Err(invalid_argument("keypoint coordinates must be finite"));
        }
        if let Some(score) = self.score {
            if !score.is_finite() {
                return Err(invalid_argument("keypoint score must be finite"));
            }
        }
        Ok(())
    }

    pub fn to_point2f(&self) -> Result<Point2f> {
        Point2f::new(self.x, self.y)
    }

    pub fn from_point2f(name: impl Into<String>, point: Point2f) -> Result<Self> {
        Self::new(name, point.x, point.y)
    }

    pub fn to_normalized_point2(&self, image_size: Size2u) -> Result<NormalizedPoint2> {
        self.to_point2f()?.to_normalized(image_size)
    }

    pub fn from_normalized_point2(
        name: impl Into<String>,
        point: NormalizedPoint2,
        image_size: Size2u,
    ) -> Result<Self> {
        let point = point.to_pixel_point_f32(image_size);
        Self::new(name, point.x, point.y)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Keypoint3d {
    pub name: String,
    pub position: Point3,
    pub score: Option<f32>,
    pub visible: Option<bool>,
}

impl Keypoint3d {
    pub fn new(name: impl Into<String>, position: Point3) -> Result<Self> {
        let keypoint = Self {
            name: name.into(),
            position,
            score: None,
            visible: None,
        };
        keypoint.validate()?;
        Ok(keypoint)
    }

    pub fn score(mut self, score: f32) -> Result<Self> {
        self.score = Some(score);
        self.validate()?;
        Ok(self)
    }

    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = Some(visible);
        self
    }

    pub fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            return Err(invalid_argument("3D keypoint name must not be empty"));
        }
        if !self.position.is_finite() {
            return Err(invalid_argument("3D keypoint coordinates must be finite"));
        }
        if let Some(score) = self.score {
            if !score.is_finite() {
                return Err(invalid_argument("3D keypoint score must be finite"));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeypointSpace {
    Pixel,
    Normalized,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkeletonEdge {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
}

impl SkeletonEdge {
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            label: None,
        }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Skeleton {
    pub keypoints: Vec<String>,
    pub edges: Vec<SkeletonEdge>,
}

impl Skeleton {
    pub fn new(keypoints: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            keypoints: keypoints.into_iter().map(Into::into).collect(),
            edges: Vec::new(),
        }
    }

    pub fn edge(mut self, from: impl Into<String>, to: impl Into<String>) -> Self {
        self.edges.push(SkeletonEdge::new(from, to));
        self
    }

    pub fn edge_label(
        mut self,
        from: impl Into<String>,
        to: impl Into<String>,
        label: impl Into<String>,
    ) -> Self {
        self.edges.push(SkeletonEdge::new(from, to).label(label));
        self
    }

    pub fn coco_17() -> Self {
        Self::new([
            "nose",
            "left_eye",
            "right_eye",
            "left_ear",
            "right_ear",
            "left_shoulder",
            "right_shoulder",
            "left_elbow",
            "right_elbow",
            "left_wrist",
            "right_wrist",
            "left_hip",
            "right_hip",
            "left_knee",
            "right_knee",
            "left_ankle",
            "right_ankle",
        ])
        .edge("left_shoulder", "right_shoulder")
        .edge("left_shoulder", "left_elbow")
        .edge("left_elbow", "left_wrist")
        .edge("right_shoulder", "right_elbow")
        .edge("right_elbow", "right_wrist")
        .edge("left_shoulder", "left_hip")
        .edge("right_shoulder", "right_hip")
        .edge("left_hip", "right_hip")
        .edge("left_hip", "left_knee")
        .edge("left_knee", "left_ankle")
        .edge("right_hip", "right_knee")
        .edge("right_knee", "right_ankle")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PoseEstimate {
    pub id: Option<String>,
    pub label: Option<String>,
    pub score: Option<f32>,
    pub region: Option<BoundingBox>,
    pub keypoints: Vec<Keypoint>,
    pub attributes: BTreeMap<String, String>,
}

impl PoseEstimate {
    pub fn new(keypoints: impl IntoIterator<Item = Keypoint>) -> Result<Self> {
        let estimate = Self {
            id: None,
            label: None,
            score: None,
            region: None,
            keypoints: keypoints.into_iter().collect(),
            attributes: BTreeMap::new(),
        };
        estimate.validate()?;
        Ok(estimate)
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn score(mut self, score: f32) -> Result<Self> {
        self.score = Some(score);
        self.validate()?;
        Ok(self)
    }

    pub fn region(mut self, region: BoundingBox) -> Self {
        self.region = Some(region);
        self
    }

    pub fn attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }

    pub fn keypoint(&self, name: &str) -> Option<&Keypoint> {
        self.keypoints.iter().find(|keypoint| keypoint.name == name)
    }

    pub fn validate(&self) -> Result<()> {
        if self.keypoints.is_empty() {
            return Err(invalid_argument(
                "pose estimate must contain at least one keypoint",
            ));
        }
        if let Some(score) = self.score {
            if !score.is_finite() {
                return Err(invalid_argument("pose score must be finite"));
            }
        }
        for keypoint in &self.keypoints {
            keypoint.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pose3dEstimate {
    pub id: Option<String>,
    pub label: Option<String>,
    pub score: Option<f32>,
    pub keypoints: Vec<Keypoint3d>,
    pub attributes: BTreeMap<String, String>,
}

impl Pose3dEstimate {
    pub fn new(keypoints: impl IntoIterator<Item = Keypoint3d>) -> Result<Self> {
        let estimate = Self {
            id: None,
            label: None,
            score: None,
            keypoints: keypoints.into_iter().collect(),
            attributes: BTreeMap::new(),
        };
        estimate.validate()?;
        Ok(estimate)
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn score(mut self, score: f32) -> Result<Self> {
        self.score = Some(score);
        self.validate()?;
        Ok(self)
    }

    pub fn attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }

    pub fn keypoint(&self, name: &str) -> Option<&Keypoint3d> {
        self.keypoints.iter().find(|keypoint| keypoint.name == name)
    }

    pub fn validate(&self) -> Result<()> {
        if self.keypoints.is_empty() {
            return Err(invalid_argument(
                "3D pose estimate must contain at least one keypoint",
            ));
        }
        if let Some(score) = self.score {
            if !score.is_finite() {
                return Err(invalid_argument("3D pose score must be finite"));
            }
        }
        for keypoint in &self.keypoints {
            keypoint.validate()?;
        }
        Ok(())
    }

    pub fn to_stick_figure(&self, skeleton: Skeleton) -> Result<StickFigure3d> {
        StickFigure3d::new(skeleton, self.keypoints.clone())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StickFigure3d {
    pub skeleton: Skeleton,
    pub keypoints: Vec<Keypoint3d>,
}

impl StickFigure3d {
    pub fn new(skeleton: Skeleton, keypoints: impl Into<Vec<Keypoint3d>>) -> Result<Self> {
        let figure = Self {
            skeleton,
            keypoints: keypoints.into(),
        };
        figure.validate()?;
        Ok(figure)
    }

    pub fn validate(&self) -> Result<()> {
        for keypoint in &self.keypoints {
            keypoint.validate()?;
        }
        Ok(())
    }

    pub fn keypoint(&self, name: &str) -> Option<&Keypoint3d> {
        self.keypoints.iter().find(|keypoint| keypoint.name == name)
    }

    pub fn segments(&self) -> Result<Vec<LineSegment3>> {
        self.validate()?;
        let mut segments = Vec::new();
        for edge in &self.skeleton.edges {
            let Some(from) = self.keypoint(&edge.from) else {
                continue;
            };
            let Some(to) = self.keypoint(&edge.to) else {
                continue;
            };
            segments.push(LineSegment3::new(from.position, to.position)?);
        }
        Ok(segments)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PoseSequence<T> {
    pub frames: Vec<T>,
}

impl<T> PoseSequence<T> {
    pub fn new(frames: impl Into<Vec<T>>) -> Self {
        Self {
            frames: frames.into(),
        }
    }

    pub fn len(&self) -> usize {
        self.frames.len()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    pub fn frames(&self) -> &[T] {
        &self.frames
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PostureOptions {
    pub keypoint_space: KeypointSpace,
    pub min_pose_score: Option<f32>,
    pub min_keypoint_score: Option<f32>,
    pub infer_region: bool,
}

impl Default for PostureOptions {
    fn default() -> Self {
        Self {
            keypoint_space: KeypointSpace::Pixel,
            min_pose_score: None,
            min_keypoint_score: None,
            infer_region: true,
        }
    }
}

impl PostureOptions {
    pub fn validate(self) -> Result<()> {
        if let Some(score) = self.min_pose_score {
            if !score.is_finite() {
                return Err(invalid_argument("minimum pose score must be finite"));
            }
        }
        if let Some(score) = self.min_keypoint_score {
            if !score.is_finite() {
                return Err(invalid_argument("minimum keypoint score must be finite"));
            }
        }
        Ok(())
    }
}

pub trait PostureBackend {
    fn estimate_frame(&mut self, frame: &VideoFrame<'_>) -> Result<Vec<PoseEstimate>>;
}

pub trait PostureLiftBackend {
    fn lift_sequence(
        &mut self,
        sequence: &PoseSequence<PoseEstimate>,
    ) -> Result<PoseSequence<Pose3dEstimate>>;
}

pub struct PostureAnalyzer<B> {
    name: String,
    backend: B,
    options: PostureOptions,
}

impl<B> PostureAnalyzer<B> {
    pub fn new(name: impl Into<String>, backend: B) -> Self {
        Self {
            name: name.into(),
            backend,
            options: PostureOptions::default(),
        }
    }

    pub fn options(mut self, options: PostureOptions) -> Result<Self> {
        options.validate()?;
        self.options = options;
        Ok(self)
    }

    pub fn backend(&self) -> &B {
        &self.backend
    }

    pub fn backend_mut(&mut self) -> &mut B {
        &mut self.backend
    }
}

impl<B: PostureBackend> VideoAnalyzer for PostureAnalyzer<B> {
    fn name(&self) -> &str {
        &self.name
    }

    fn process_frame(&mut self, frame: &VideoFrame<'_>) -> Result<Vec<Observation>> {
        self.options.validate()?;
        let estimates = self.backend.estimate_frame(frame)?;
        let mut observations = Vec::new();
        for estimate in estimates {
            estimate.validate()?;
            if self
                .options
                .min_pose_score
                .zip(estimate.score)
                .map(|(minimum, score)| score < minimum)
                .unwrap_or(false)
            {
                continue;
            }
            let keypoints = filtered_keypoints(&estimate, self.options.min_keypoint_score);
            if keypoints.is_empty() {
                continue;
            }
            observations.push(observation_for_pose(
                self.name(),
                frame,
                estimate,
                keypoints,
                self.options,
            ));
        }
        Ok(observations)
    }
}

pub fn joint_angle_degrees(a: &Keypoint, b: &Keypoint, c: &Keypoint) -> Result<f32> {
    a.validate()?;
    b.validate()?;
    c.validate()?;
    let ab = (a.x - b.x, a.y - b.y);
    let cb = (c.x - b.x, c.y - b.y);
    angle_from_vectors(Vector3::new(ab.0, ab.1, 0.0), Vector3::new(cb.0, cb.1, 0.0))
}

pub fn joint_angle_3d_degrees(a: &Keypoint3d, b: &Keypoint3d, c: &Keypoint3d) -> Result<f32> {
    a.validate()?;
    b.validate()?;
    c.validate()?;
    angle_from_vectors(a.position - b.position, c.position - b.position)
}

pub fn bone_lengths(pose: &Pose3dEstimate, skeleton: &Skeleton) -> Result<BTreeMap<String, f32>> {
    pose.validate()?;
    let mut lengths = BTreeMap::new();
    for edge in &skeleton.edges {
        let Some(from) = pose.keypoint(&edge.from) else {
            continue;
        };
        let Some(to) = pose.keypoint(&edge.to) else {
            continue;
        };
        lengths.insert(
            edge.label
                .clone()
                .unwrap_or_else(|| format!("{}-{}", edge.from, edge.to)),
            from.position.distance(to.position),
        );
    }
    Ok(lengths)
}

pub fn normalize_pose3d(pose: &Pose3dEstimate, root_name: &str) -> Result<Pose3dEstimate> {
    pose.validate()?;
    let root = pose
        .keypoint(root_name)
        .ok_or_else(|| invalid_argument(format!("pose has no root keypoint `{root_name}`")))?;
    let max_radius = pose
        .keypoints
        .iter()
        .map(|keypoint| keypoint.position.distance(root.position))
        .fold(0.0_f32, f32::max)
        .max(1.0);
    let keypoints = pose
        .keypoints
        .iter()
        .map(|keypoint| {
            let relative = keypoint.position - root.position;
            let mut normalized = keypoint.clone();
            normalized.position = Point3::new(
                relative.x / max_radius,
                relative.y / max_radius,
                relative.z / max_radius,
            );
            normalized
        })
        .collect::<Vec<_>>();
    let mut normalized = Pose3dEstimate::new(keypoints)?;
    normalized.id = pose.id.clone();
    normalized.label = pose.label.clone();
    normalized.score = pose.score;
    normalized.attributes = pose.attributes.clone();
    Ok(normalized)
}

pub fn interpolate_missing_joints(
    pose: &Pose3dEstimate,
    skeleton: &Skeleton,
) -> Result<Pose3dEstimate> {
    pose.validate()?;
    let mut keypoints = pose.keypoints.clone();
    for name in &skeleton.keypoints {
        if keypoints.iter().any(|keypoint| &keypoint.name == name) {
            continue;
        }
        let neighbors = skeleton
            .edges
            .iter()
            .filter_map(|edge| {
                if &edge.from == name {
                    pose.keypoint(&edge.to)
                } else if &edge.to == name {
                    pose.keypoint(&edge.from)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        if neighbors.is_empty() {
            continue;
        }
        let mean = neighbors.iter().fold(Vector3::ZERO, |sum, neighbor| {
            sum + Vector3::new(
                neighbor.position.x,
                neighbor.position.y,
                neighbor.position.z,
            )
        }) / neighbors.len() as f32;
        keypoints.push(Keypoint3d::new(
            name.clone(),
            Point3::new(mean.x, mean.y, mean.z),
        )?);
    }
    let mut interpolated = Pose3dEstimate::new(keypoints)?;
    interpolated.id = pose.id.clone();
    interpolated.label = pose.label.clone();
    interpolated.score = pose.score;
    interpolated.attributes = pose.attributes.clone();
    Ok(interpolated)
}

pub fn smooth_pose_sequence(
    sequence: &PoseSequence<Pose3dEstimate>,
    window_radius: usize,
) -> Result<PoseSequence<Pose3dEstimate>> {
    if sequence.is_empty() {
        return Ok(PoseSequence::new(Vec::<Pose3dEstimate>::new()));
    }
    for pose in &sequence.frames {
        pose.validate()?;
    }
    let mut smoothed = Vec::with_capacity(sequence.frames.len());
    for frame_index in 0..sequence.frames.len() {
        let start = frame_index.saturating_sub(window_radius);
        let end = (frame_index + window_radius + 1).min(sequence.frames.len());
        let window = &sequence.frames[start..end];
        let current = &sequence.frames[frame_index];
        let mut keypoints = Vec::new();
        for keypoint in &current.keypoints {
            let matches = window
                .iter()
                .filter_map(|pose| pose.keypoint(&keypoint.name))
                .collect::<Vec<_>>();
            let mean = matches.iter().fold(Vector3::ZERO, |sum, item| {
                sum + Vector3::new(item.position.x, item.position.y, item.position.z)
            }) / matches.len() as f32;
            let mut smoothed_keypoint = keypoint.clone();
            smoothed_keypoint.position = Point3::new(mean.x, mean.y, mean.z);
            keypoints.push(smoothed_keypoint);
        }
        let mut pose = Pose3dEstimate::new(keypoints)?;
        pose.id = current.id.clone();
        pose.label = current.label.clone();
        pose.score = current.score;
        pose.attributes = current.attributes.clone();
        smoothed.push(pose);
    }
    Ok(PoseSequence::new(smoothed))
}

fn angle_from_vectors(ab: Vector3, cb: Vector3) -> Result<f32> {
    let ab_len = ab.length();
    let cb_len = cb.length();
    if ab_len <= f32::EPSILON || cb_len <= f32::EPSILON {
        return Err(invalid_argument(
            "joint angle requires non-overlapping keypoints",
        ));
    }
    let cosine = (ab.dot(cb) / (ab_len * cb_len)).clamp(-1.0, 1.0);
    Ok(cosine.acos().to_degrees())
}

fn filtered_keypoints(estimate: &PoseEstimate, min_score: Option<f32>) -> Vec<Keypoint> {
    estimate
        .keypoints
        .iter()
        .filter(|keypoint| {
            min_score
                .zip(keypoint.score)
                .map(|(minimum, score)| score >= minimum)
                .unwrap_or(true)
        })
        .cloned()
        .collect()
}

fn observation_for_pose(
    analyzer: &str,
    frame: &VideoFrame<'_>,
    estimate: PoseEstimate,
    keypoints: Vec<Keypoint>,
    options: PostureOptions,
) -> Observation {
    let mut observation =
        Observation::new(analyzer, ObservationKind::Custom("posture".to_string()))
            .at_frame(frame.position)
            .label(estimate.label.unwrap_or_else(|| "pose".to_string()))
            .attribute("keypoint_count", keypoints.len().to_string())
            .attribute("keypoints", encode_keypoints(&keypoints));
    if let Some(score) = estimate.score {
        observation = observation.score(score);
    }
    if let Some(id) = estimate.id {
        observation = observation.track_id(id);
    }
    let region = estimate.region.or_else(|| {
        options
            .infer_region
            .then(|| {
                infer_region(frame, &keypoints, options.keypoint_space)
                    .ok()
                    .flatten()
            })
            .flatten()
    });
    if let Some(region) = region {
        observation = observation.region(region);
    }
    for (key, value) in estimate.attributes {
        observation = observation.attribute(key, value);
    }
    observation
}

fn infer_region(
    frame: &VideoFrame<'_>,
    keypoints: &[Keypoint],
    keypoint_space: KeypointSpace,
) -> Result<Option<BoundingBox>> {
    let visible = keypoints
        .iter()
        .filter(|keypoint| keypoint.visible.unwrap_or(true))
        .collect::<Vec<_>>();
    if visible.is_empty() {
        return Ok(None);
    }

    let scale_x = if keypoint_space == KeypointSpace::Normalized {
        frame.width as f32
    } else {
        1.0
    };
    let scale_y = if keypoint_space == KeypointSpace::Normalized {
        frame.height as f32
    } else {
        1.0
    };

    let min_x = visible
        .iter()
        .map(|keypoint| keypoint.x * scale_x)
        .fold(f32::INFINITY, f32::min)
        .floor()
        .max(0.0) as u32;
    let min_y = visible
        .iter()
        .map(|keypoint| keypoint.y * scale_y)
        .fold(f32::INFINITY, f32::min)
        .floor()
        .max(0.0) as u32;
    let mut max_x = visible
        .iter()
        .map(|keypoint| keypoint.x * scale_x)
        .fold(f32::NEG_INFINITY, f32::max)
        .ceil()
        .min(frame.width as f32) as u32;
    let mut max_y = visible
        .iter()
        .map(|keypoint| keypoint.y * scale_y)
        .fold(f32::NEG_INFINITY, f32::max)
        .ceil()
        .min(frame.height as f32) as u32;
    if max_x <= min_x {
        max_x = (min_x + 1).min(frame.width);
    }
    if max_y <= min_y {
        max_y = (min_y + 1).min(frame.height);
    }
    if max_x <= min_x || max_y <= min_y {
        return Ok(None);
    }
    BoundingBox::new(min_x, min_y, max_x - min_x, max_y - min_y).map(Some)
}

fn encode_keypoints(keypoints: &[Keypoint]) -> String {
    keypoints
        .iter()
        .map(|keypoint| {
            let score = keypoint
                .score
                .map(|score| score.to_string())
                .unwrap_or_default();
            let visible = keypoint
                .visible
                .map(|visible| visible.to_string())
                .unwrap_or_default();
            format!(
                "{}:{:.3}:{:.3}:{score}:{visible}",
                keypoint.name, keypoint.x, keypoint.y
            )
        })
        .collect::<Vec<_>>()
        .join("|")
}

fn invalid_argument(message: impl Into<String>) -> DetectError {
    DetectError::InvalidArgument(message.into())
}

#[cfg(test)]
mod tests {
    use num_rational::Rational64;
    use video_analysis_core::{FramePosition, OwnedVideoFrame, PixelFormat};

    use super::*;

    fn frame() -> OwnedVideoFrame {
        OwnedVideoFrame {
            position: FramePosition::from_frame_index(0, Rational64::new(30, 1)),
            width: 100,
            height: 100,
            pixel_format: PixelFormat::Rgb24,
            data: vec![0; 100 * 100 * 3],
            stride: 100 * 3,
        }
    }

    #[test]
    fn joint_angle_reports_right_angle() {
        let a = Keypoint::new("a", 0.0, 0.0).unwrap();
        let b = Keypoint::new("b", 0.0, 1.0).unwrap();
        let c = Keypoint::new("c", 1.0, 1.0).unwrap();

        let angle = joint_angle_degrees(&a, &b, &c).unwrap();

        assert!((angle - 90.0).abs() < 0.001);
    }

    #[test]
    fn analyzer_emits_pose_observation() {
        struct Backend;

        impl PostureBackend for Backend {
            fn estimate_frame(&mut self, _frame: &VideoFrame<'_>) -> Result<Vec<PoseEstimate>> {
                Ok(vec![PoseEstimate::new([
                    Keypoint::new("left_shoulder", 10.0, 20.0)?.score(0.9)?,
                    Keypoint::new("right_shoulder", 30.0, 20.0)?.score(0.8)?,
                ])?
                .id("pose-1")
                .score(0.95)?])
            }
        }

        let owned = frame();
        let frame = owned.as_frame();
        let mut analyzer = PostureAnalyzer::new("posture", Backend);
        let observations = analyzer.process_frame(&frame).unwrap();
        assert_eq!(observations.len(), 1);
        assert_eq!(observations[0].track_id.as_deref(), Some("pose-1"));
        assert_eq!(observations[0].attributes["keypoint_count"], "2");
    }

    #[test]
    fn analyzer_filters_low_score_keypoints() {
        struct Backend;

        impl PostureBackend for Backend {
            fn estimate_frame(&mut self, _frame: &VideoFrame<'_>) -> Result<Vec<PoseEstimate>> {
                Ok(vec![PoseEstimate::new([
                    Keypoint::new("left_shoulder", 10.0, 20.0)?.score(0.2)?,
                    Keypoint::new("right_shoulder", 30.0, 20.0)?.score(0.3)?,
                ])?
                .score(0.95)?])
            }
        }

        let owned = frame();
        let frame = owned.as_frame();
        let mut analyzer = PostureAnalyzer::new("posture", Backend)
            .options(PostureOptions {
                min_keypoint_score: Some(0.5),
                ..PostureOptions::default()
            })
            .unwrap();
        let observations = analyzer.process_frame(&frame).unwrap();
        assert!(observations.is_empty());
    }

    #[test]
    fn keypoints_convert_through_shared_geometry() {
        let keypoint = Keypoint::new("nose", 32.0, 24.0).unwrap();
        let size = Size2u::new(64, 48).unwrap();
        let normalized = keypoint.to_normalized_point2(size).unwrap();
        let round_trip = Keypoint::from_normalized_point2("nose", normalized, size).unwrap();
        assert!((round_trip.x - keypoint.x).abs() < 1.0e-6);
        assert!((round_trip.y - keypoint.y).abs() < 1.0e-6);
    }

    #[test]
    fn three_d_posture_helpers_cover_normalization_interpolation_and_smoothing() {
        let skeleton = Skeleton::coco_17();
        let pose = Pose3dEstimate::new([
            Keypoint3d::new("left_shoulder", Point3::new(0.0, 0.0, 0.0)).unwrap(),
            Keypoint3d::new("right_shoulder", Point3::new(1.0, 0.0, 0.0)).unwrap(),
            Keypoint3d::new("left_elbow", Point3::new(0.0, -1.0, 0.0)).unwrap(),
        ])
        .unwrap();
        let normalized = normalize_pose3d(&pose, "left_shoulder").unwrap();
        assert_eq!(
            normalized.keypoint("left_shoulder").unwrap().position,
            Point3::new(0.0, 0.0, 0.0)
        );

        let interpolated = interpolate_missing_joints(&pose, &skeleton).unwrap();
        assert!(interpolated.keypoint("right_elbow").is_some());

        let sequence = PoseSequence::new([pose.clone(), normalized.clone()]);
        let smoothed = smooth_pose_sequence(&sequence, 1).unwrap();
        assert_eq!(smoothed.len(), 2);

        let figure = pose.to_stick_figure(skeleton).unwrap();
        assert!(!figure.segments().unwrap().is_empty());
    }

    #[test]
    fn three_d_joint_angle_and_bone_lengths_are_reported() {
        let a = Keypoint3d::new("a", Point3::new(0.0, 0.0, 0.0)).unwrap();
        let b = Keypoint3d::new("b", Point3::new(0.0, 1.0, 0.0)).unwrap();
        let c = Keypoint3d::new("c", Point3::new(1.0, 1.0, 0.0)).unwrap();
        let angle = joint_angle_3d_degrees(&a, &b, &c).unwrap();
        assert!((angle - 90.0).abs() < 0.001);

        let skeleton = Skeleton::new(["a", "b", "c"]).edge("a", "b").edge("b", "c");
        let pose = Pose3dEstimate::new([a, b, c]).unwrap();
        let lengths = bone_lengths(&pose, &skeleton).unwrap();
        assert_eq!(lengths.len(), 2);
    }
}
