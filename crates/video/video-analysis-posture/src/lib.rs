#![doc = include_str!("../README.md")]

use std::collections::BTreeMap;

use video_analysis_core::{
    BoundingBox, DetectError, Observation, ObservationKind, Result, VideoAnalyzer, VideoFrame,
};

#[derive(Debug, Clone, PartialEq)]
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
            return Err(DetectError::InvalidArgument(
                "keypoint name must not be empty".to_string(),
            ));
        }
        if !self.x.is_finite() || !self.y.is_finite() {
            return Err(DetectError::InvalidArgument(
                "keypoint coordinates must be finite".to_string(),
            ));
        }
        if let Some(score) = self.score {
            if !score.is_finite() {
                return Err(DetectError::InvalidArgument(
                    "keypoint score must be finite".to_string(),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeypointSpace {
    Pixel,
    Normalized,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
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
            return Err(DetectError::InvalidArgument(
                "pose estimate must contain at least one keypoint".to_string(),
            ));
        }
        if let Some(score) = self.score {
            if !score.is_finite() {
                return Err(DetectError::InvalidArgument(
                    "pose score must be finite".to_string(),
                ));
            }
        }
        for keypoint in &self.keypoints {
            keypoint.validate()?;
        }
        Ok(())
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
                return Err(DetectError::InvalidArgument(
                    "minimum pose score must be finite".to_string(),
                ));
            }
        }
        if let Some(score) = self.min_keypoint_score {
            if !score.is_finite() {
                return Err(DetectError::InvalidArgument(
                    "minimum keypoint score must be finite".to_string(),
                ));
            }
        }
        Ok(())
    }
}

pub trait PostureBackend {
    fn estimate_frame(&mut self, frame: &VideoFrame<'_>) -> Result<Vec<PoseEstimate>>;
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
    let ab_len = (ab.0 * ab.0 + ab.1 * ab.1).sqrt();
    let cb_len = (cb.0 * cb.0 + cb.1 * cb.1).sqrt();
    if ab_len == 0.0 || cb_len == 0.0 {
        return Err(DetectError::InvalidArgument(
            "joint angle requires non-overlapping keypoints".to_string(),
        ));
    }
    let cosine = ((ab.0 * cb.0 + ab.1 * cb.1) / (ab_len * cb_len)).clamp(-1.0, 1.0);
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

        let mut analyzer = PostureAnalyzer::new("pose", Backend)
            .options(PostureOptions {
                min_keypoint_score: Some(0.5),
                ..PostureOptions::default()
            })
            .unwrap();
        let observations = analyzer.process_frame(&frame().as_frame()).unwrap();

        assert_eq!(observations.len(), 1);
        assert_eq!(
            observations[0].kind,
            ObservationKind::Custom("posture".to_string())
        );
        assert_eq!(observations[0].track_id.as_deref(), Some("pose-1"));
        assert_eq!(
            observations[0]
                .attributes
                .get("keypoint_count")
                .map(String::as_str),
            Some("2")
        );
        assert_eq!(
            observations[0].region,
            Some(BoundingBox::new(10, 20, 20, 1).unwrap())
        );
    }

    #[test]
    fn analyzer_filters_low_score_keypoints() {
        let estimate = PoseEstimate::new([
            Keypoint::new("strong", 1.0, 1.0)
                .unwrap()
                .score(0.9)
                .unwrap(),
            Keypoint::new("weak", 2.0, 2.0).unwrap().score(0.1).unwrap(),
        ])
        .unwrap();

        let filtered = filtered_keypoints(&estimate, Some(0.5));

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "strong");
    }
}
