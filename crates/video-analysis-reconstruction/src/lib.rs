use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

use video_analysis_core::{DetectError, Result};
use video_analysis_radiance_fields::{CameraIntrinsics, CameraPose, ColorRgb, Ray, Vec2, Vec3};

fn invalid_argument(message: impl Into<String>) -> DetectError {
    DetectError::InvalidArgument(message.into())
}

fn validate_finite(value: f32, name: &str) -> Result<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(invalid_argument(format!("{name} must be finite")))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CameraId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ImageId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Point3dId(pub u64);

#[derive(Debug, Clone, PartialEq)]
pub struct ReconstructionCamera {
    pub id: CameraId,
    pub intrinsics: CameraIntrinsics,
}

impl ReconstructionCamera {
    pub fn new(id: CameraId, intrinsics: CameraIntrinsics) -> Result<Self> {
        intrinsics.validate()?;
        Ok(Self { id, intrinsics })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Feature2d {
    pub pixel: Vec2,
    pub color: Option<ColorRgb>,
    pub score: f32,
}

impl Feature2d {
    pub fn new(pixel: Vec2) -> Result<Self> {
        let feature = Self {
            pixel,
            color: None,
            score: 1.0,
        };
        feature.validate()?;
        Ok(feature)
    }

    pub fn with_color(mut self, color: ColorRgb) -> Result<Self> {
        self.color = Some(color);
        self.validate()?;
        Ok(self)
    }

    pub fn with_score(mut self, score: f32) -> Result<Self> {
        self.score = score;
        self.validate()?;
        Ok(self)
    }

    pub fn validate(self) -> Result<()> {
        if !self.pixel.is_finite() {
            return Err(invalid_argument("feature pixel must be finite"));
        }
        validate_finite(self.score, "feature score")?;
        if self.score < 0.0 {
            return Err(invalid_argument(
                "feature score must be greater than or equal to zero",
            ));
        }
        if let Some(color) = self.color {
            if !color.is_finite() {
                return Err(invalid_argument("feature color must be finite"));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BinaryFeature {
    pub keypoint: Feature2d,
    pub descriptor: Vec<u8>,
}

impl BinaryFeature {
    pub fn new(keypoint: Feature2d, descriptor: impl Into<Vec<u8>>) -> Result<Self> {
        let feature = Self {
            keypoint,
            descriptor: descriptor.into(),
        };
        feature.validate()?;
        Ok(feature)
    }

    pub fn validate(&self) -> Result<()> {
        self.keypoint.validate()?;
        if self.descriptor.is_empty() {
            return Err(invalid_argument("binary descriptor must not be empty"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FeatureMatch {
    pub left_feature: usize,
    pub right_feature: usize,
    pub distance: u32,
    pub confidence: f32,
}

impl FeatureMatch {
    pub fn new(
        left_feature: usize,
        right_feature: usize,
        distance: u32,
        confidence: f32,
    ) -> Result<Self> {
        let feature_match = Self {
            left_feature,
            right_feature,
            distance,
            confidence,
        };
        feature_match.validate()?;
        Ok(feature_match)
    }

    pub fn validate(self) -> Result<()> {
        validate_finite(self.confidence, "match confidence")?;
        if !(0.0..=1.0).contains(&self.confidence) {
            return Err(invalid_argument(
                "match confidence must be in the range [0, 1]",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MatchConfig {
    pub max_distance: u32,
    pub ratio: f32,
    pub cross_check: bool,
}

impl MatchConfig {
    pub fn validate(self) -> Result<()> {
        validate_finite(self.ratio, "ratio")?;
        if self.ratio <= 0.0 || self.ratio > 1.0 {
            return Err(invalid_argument("ratio must be in the range (0, 1]"));
        }
        Ok(())
    }
}

impl Default for MatchConfig {
    fn default() -> Self {
        Self {
            max_distance: 96,
            ratio: 0.8,
            cross_check: true,
        }
    }
}

pub fn hamming_distance(left: &[u8], right: &[u8]) -> Result<u32> {
    if left.len() != right.len() {
        return Err(invalid_argument(
            "binary descriptors must have the same length",
        ));
    }
    if left.is_empty() {
        return Err(invalid_argument("binary descriptors must not be empty"));
    }
    Ok(left
        .iter()
        .zip(right)
        .map(|(left, right)| (left ^ right).count_ones())
        .sum())
}

pub fn match_binary_features(
    left: &[BinaryFeature],
    right: &[BinaryFeature],
    config: MatchConfig,
) -> Result<Vec<FeatureMatch>> {
    config.validate()?;
    validate_binary_feature_set(left, "left")?;
    validate_binary_feature_set(right, "right")?;

    let mut matches = Vec::new();
    for (left_index, left_feature) in left.iter().enumerate() {
        if let Some((right_index, distance, second_distance)) =
            nearest_binary_descriptor(&left_feature.descriptor, right)?
        {
            if distance > config.max_distance {
                continue;
            }
            if let Some(second_distance) = second_distance {
                if (distance as f32) > config.ratio * (second_distance as f32) {
                    continue;
                }
            }
            if config.cross_check {
                let reverse = nearest_binary_descriptor(&right[right_index].descriptor, left)?;
                if reverse
                    .map(|(reverse_index, _, _)| reverse_index != left_index)
                    .unwrap_or(true)
                {
                    continue;
                }
            }

            let max_bits = (left_feature.descriptor.len() * 8) as f32;
            let confidence = (1.0 - distance as f32 / max_bits).clamp(0.0, 1.0);
            matches.push(FeatureMatch::new(
                left_index,
                right_index,
                distance,
                confidence,
            )?);
        }
    }
    Ok(matches)
}

fn validate_binary_feature_set(features: &[BinaryFeature], name: &str) -> Result<()> {
    if features.is_empty() {
        return Err(invalid_argument(format!(
            "{name} binary feature set must not be empty"
        )));
    }
    let descriptor_len = features[0].descriptor.len();
    for feature in features {
        feature.validate()?;
        if feature.descriptor.len() != descriptor_len {
            return Err(invalid_argument(format!(
                "{name} binary descriptors must have consistent lengths"
            )));
        }
    }
    Ok(())
}

fn nearest_binary_descriptor(
    descriptor: &[u8],
    candidates: &[BinaryFeature],
) -> Result<Option<(usize, u32, Option<u32>)>> {
    let mut best: Option<(usize, u32)> = None;
    let mut second: Option<u32> = None;
    for (index, candidate) in candidates.iter().enumerate() {
        let distance = hamming_distance(descriptor, &candidate.descriptor)?;
        match best {
            None => best = Some((index, distance)),
            Some((_, best_distance)) if distance < best_distance => {
                second = Some(best_distance);
                best = Some((index, distance));
            }
            Some((_, best_distance)) if distance != best_distance => {
                second = Some(second.map_or(distance, |current| current.min(distance)));
            }
            _ => {}
        }
    }
    Ok(best.map(|(index, distance)| (index, distance, second)))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ImageFeatureKey {
    pub image_id: ImageId,
    pub feature_index: usize,
}

impl ImageFeatureKey {
    pub const fn new(image_id: ImageId, feature_index: usize) -> Self {
        Self {
            image_id,
            feature_index,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImagePairMatches {
    pub left_image_id: ImageId,
    pub right_image_id: ImageId,
    pub matches: Vec<FeatureMatch>,
}

impl ImagePairMatches {
    pub fn new(
        left_image_id: ImageId,
        right_image_id: ImageId,
        matches: impl Into<Vec<FeatureMatch>>,
    ) -> Result<Self> {
        let pair = Self {
            left_image_id,
            right_image_id,
            matches: matches.into(),
        };
        pair.validate()?;
        Ok(pair)
    }

    pub fn validate(&self) -> Result<()> {
        if self.left_image_id == self.right_image_id {
            return Err(invalid_argument("pair matches must reference two images"));
        }
        for feature_match in &self.matches {
            feature_match.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrackElement {
    pub image_id: ImageId,
    pub feature_index: usize,
}

impl TrackElement {
    pub const fn new(image_id: ImageId, feature_index: usize) -> Self {
        Self {
            image_id,
            feature_index,
        }
    }

    pub const fn key(&self) -> ImageFeatureKey {
        ImageFeatureKey::new(self.image_id, self.feature_index)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Track {
    pub elements: Vec<TrackElement>,
}

impl Track {
    pub fn new(elements: impl Into<Vec<TrackElement>>) -> Result<Self> {
        let track = Self {
            elements: elements.into(),
        };
        track.validate()?;
        Ok(track)
    }

    pub fn from_match(
        left_image_id: ImageId,
        right_image_id: ImageId,
        feature_match: FeatureMatch,
    ) -> Result<Self> {
        feature_match.validate()?;
        Self::new([
            TrackElement::new(left_image_id, feature_match.left_feature),
            TrackElement::new(right_image_id, feature_match.right_feature),
        ])
    }

    pub fn validate(&self) -> Result<()> {
        if self.elements.len() < 2 {
            return Err(invalid_argument("track must contain at least two elements"));
        }
        let mut images = BTreeSet::new();
        for element in &self.elements {
            if !images.insert(element.image_id) {
                return Err(invalid_argument(
                    "track must not contain multiple features from the same image",
                ));
            }
        }
        Ok(())
    }
}

pub fn build_tracks(pair_matches: &[ImagePairMatches]) -> Result<Vec<Track>> {
    let mut indices = BTreeMap::new();
    for pair in pair_matches {
        pair.validate()?;
        for feature_match in &pair.matches {
            let left_key = ImageFeatureKey::new(pair.left_image_id, feature_match.left_feature);
            let right_key = ImageFeatureKey::new(pair.right_image_id, feature_match.right_feature);
            let next_index = indices.len();
            indices.entry(left_key).or_insert(next_index);
            let next_index = indices.len();
            indices.entry(right_key).or_insert(next_index);
        }
    }

    let mut union_find = UnionFind::new(indices.len());
    for pair in pair_matches {
        for feature_match in &pair.matches {
            let left_key = ImageFeatureKey::new(pair.left_image_id, feature_match.left_feature);
            let right_key = ImageFeatureKey::new(pair.right_image_id, feature_match.right_feature);
            let left_index = *indices.get(&left_key).expect("feature key was indexed");
            let right_index = *indices.get(&right_key).expect("feature key was indexed");
            union_find.union(left_index, right_index);
        }
    }

    let mut grouped: BTreeMap<usize, Vec<ImageFeatureKey>> = BTreeMap::new();
    for (key, index) in indices {
        grouped.entry(union_find.find(index)).or_default().push(key);
    }

    let mut tracks = Vec::new();
    for mut keys in grouped.into_values() {
        if keys.len() < 2 {
            continue;
        }
        keys.sort();
        if has_duplicate_track_images(&keys) {
            continue;
        }
        let elements = keys
            .into_iter()
            .map(|key| TrackElement::new(key.image_id, key.feature_index))
            .collect::<Vec<_>>();
        tracks.push(Track::new(elements)?);
    }
    Ok(tracks)
}

fn has_duplicate_track_images(keys: &[ImageFeatureKey]) -> bool {
    let mut images = BTreeSet::new();
    keys.iter().any(|key| !images.insert(key.image_id))
}

#[derive(Debug, Clone)]
struct UnionFind {
    parents: Vec<usize>,
    ranks: Vec<u8>,
}

impl UnionFind {
    fn new(len: usize) -> Self {
        Self {
            parents: (0..len).collect(),
            ranks: vec![0; len],
        }
    }

    fn find(&mut self, index: usize) -> usize {
        let parent = self.parents[index];
        if parent != index {
            let root = self.find(parent);
            self.parents[index] = root;
        }
        self.parents[index]
    }

    fn union(&mut self, left: usize, right: usize) {
        let left_root = self.find(left);
        let right_root = self.find(right);
        if left_root == right_root {
            return;
        }
        let left_rank = self.ranks[left_root];
        let right_rank = self.ranks[right_root];
        if left_rank < right_rank {
            self.parents[left_root] = right_root;
        } else if left_rank > right_rank {
            self.parents[right_root] = left_root;
        } else {
            self.parents[right_root] = left_root;
            self.ranks[left_root] += 1;
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReconstructionImage {
    pub id: ImageId,
    pub camera_id: CameraId,
    pub name: String,
    pub pose: CameraPose,
    pub features: Vec<Feature2d>,
}

impl ReconstructionImage {
    pub fn new(
        id: ImageId,
        camera_id: CameraId,
        name: impl Into<String>,
        pose: CameraPose,
    ) -> Result<Self> {
        pose.validate()?;
        Ok(Self {
            id,
            camera_id,
            name: name.into(),
            pose,
            features: Vec::new(),
        })
    }

    pub fn add_feature(&mut self, feature: Feature2d) -> Result<usize> {
        feature.validate()?;
        let index = self.features.len();
        self.features.push(feature);
        Ok(index)
    }

    pub fn feature(&self, index: usize) -> Result<Feature2d> {
        self.features.get(index).copied().ok_or_else(|| {
            invalid_argument(format!(
                "image {:?} does not contain feature {index}",
                self.id
            ))
        })
    }

    pub fn validate(&self) -> Result<()> {
        self.pose.validate()?;
        for feature in &self.features {
            feature.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SparsePoint3d {
    pub id: Point3dId,
    pub position: Vec3,
    pub color: ColorRgb,
    pub track: Track,
    pub reprojection_error: f32,
}

impl SparsePoint3d {
    pub fn validate(&self) -> Result<()> {
        if !self.position.is_finite() {
            return Err(invalid_argument("sparse point position must be finite"));
        }
        if !self.color.is_finite() {
            return Err(invalid_argument("sparse point color must be finite"));
        }
        self.track.validate()?;
        validate_finite(self.reprojection_error, "reprojection_error")?;
        if self.reprojection_error < 0.0 {
            return Err(invalid_argument(
                "reprojection_error must be greater than or equal to zero",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TriangulationConfig {
    pub min_angle_radians: f32,
    pub max_reprojection_error: f32,
    pub min_depth: f32,
    pub max_ray_distance: f32,
}

impl TriangulationConfig {
    pub fn validate(self) -> Result<()> {
        for (name, value) in [
            ("min_angle_radians", self.min_angle_radians),
            ("max_reprojection_error", self.max_reprojection_error),
            ("min_depth", self.min_depth),
            ("max_ray_distance", self.max_ray_distance),
        ] {
            validate_finite(value, name)?;
        }
        if self.min_angle_radians < 0.0 || self.min_angle_radians >= std::f32::consts::PI {
            return Err(invalid_argument(
                "min_angle_radians must be in the range [0, pi)",
            ));
        }
        if self.max_reprojection_error < 0.0 {
            return Err(invalid_argument(
                "max_reprojection_error must be greater than or equal to zero",
            ));
        }
        if self.min_depth <= 0.0 {
            return Err(invalid_argument("min_depth must be positive"));
        }
        if self.max_ray_distance < 0.0 {
            return Err(invalid_argument(
                "max_ray_distance must be greater than or equal to zero",
            ));
        }
        Ok(())
    }
}

impl Default for TriangulationConfig {
    fn default() -> Self {
        Self {
            min_angle_radians: 1.0_f32.to_radians(),
            max_reprojection_error: 4.0,
            min_depth: 1.0e-4,
            max_ray_distance: 0.25,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TriangulatedPoint {
    pub position: Vec3,
    pub ray_distance: f32,
    pub angle_radians: f32,
    pub reprojection_error: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SparseReconstruction {
    cameras: BTreeMap<CameraId, ReconstructionCamera>,
    images: BTreeMap<ImageId, ReconstructionImage>,
    points: BTreeMap<Point3dId, SparsePoint3d>,
    next_point_id: u64,
}

impl SparseReconstruction {
    pub fn new() -> Self {
        Self {
            cameras: BTreeMap::new(),
            images: BTreeMap::new(),
            points: BTreeMap::new(),
            next_point_id: 1,
        }
    }

    pub fn cameras(&self) -> &BTreeMap<CameraId, ReconstructionCamera> {
        &self.cameras
    }

    pub fn images(&self) -> &BTreeMap<ImageId, ReconstructionImage> {
        &self.images
    }

    pub fn points(&self) -> &BTreeMap<Point3dId, SparsePoint3d> {
        &self.points
    }

    pub fn add_camera(&mut self, camera: ReconstructionCamera) -> Result<()> {
        if self.cameras.contains_key(&camera.id) {
            return Err(invalid_argument(format!(
                "duplicate camera id {:?}",
                camera.id
            )));
        }
        camera.intrinsics.validate()?;
        self.cameras.insert(camera.id, camera);
        Ok(())
    }

    pub fn add_image(&mut self, image: ReconstructionImage) -> Result<()> {
        if !self.cameras.contains_key(&image.camera_id) {
            return Err(invalid_argument(format!(
                "image {:?} references missing camera {:?}",
                image.id, image.camera_id
            )));
        }
        if self.images.contains_key(&image.id) {
            return Err(invalid_argument(format!(
                "duplicate image id {:?}",
                image.id
            )));
        }
        image.validate()?;
        self.images.insert(image.id, image);
        Ok(())
    }

    pub fn image_mut(&mut self, image_id: ImageId) -> Result<&mut ReconstructionImage> {
        self.images
            .get_mut(&image_id)
            .ok_or_else(|| invalid_argument(format!("missing image {:?}", image_id)))
    }

    pub fn camera_for_image(&self, image_id: ImageId) -> Result<&ReconstructionCamera> {
        let image = self.image(image_id)?;
        self.cameras.get(&image.camera_id).ok_or_else(|| {
            invalid_argument(format!(
                "image {:?} references missing camera {:?}",
                image.id, image.camera_id
            ))
        })
    }

    pub fn image(&self, image_id: ImageId) -> Result<&ReconstructionImage> {
        self.images
            .get(&image_id)
            .ok_or_else(|| invalid_argument(format!("missing image {:?}", image_id)))
    }

    pub fn insert_point(
        &mut self,
        position: Vec3,
        color: ColorRgb,
        track: Track,
        reprojection_error: f32,
    ) -> Result<Point3dId> {
        let id = Point3dId(self.next_point_id);
        self.next_point_id += 1;
        let point = SparsePoint3d {
            id,
            position,
            color,
            track,
            reprojection_error,
        };
        point.validate()?;
        self.points.insert(id, point);
        Ok(id)
    }

    pub fn triangulate_track(
        &self,
        track: &Track,
        config: TriangulationConfig,
    ) -> Result<TriangulatedPoint> {
        track.validate()?;
        config.validate()?;

        let observations = self.track_observations(track)?;
        let mut candidates = Vec::new();
        for left_index in 0..observations.len() {
            for right_index in (left_index + 1)..observations.len() {
                let left = observations[left_index];
                let right = observations[right_index];
                if let Ok(candidate) = triangulate_observation_pair(
                    left.image,
                    left.intrinsics,
                    left.feature,
                    right.image,
                    right.intrinsics,
                    right.feature,
                    config,
                ) {
                    candidates.push(candidate.position);
                }
            }
        }
        if candidates.is_empty() {
            return Err(invalid_argument(
                "track did not produce a valid triangulation",
            ));
        }

        let position = average_vec3(&candidates);
        let reprojection_error = mean_track_reprojection_error(&observations, position)?;
        if reprojection_error > config.max_reprojection_error {
            return Err(invalid_argument(format!(
                "track reprojection error {reprojection_error:.3} exceeds maximum {:.3}",
                config.max_reprojection_error
            )));
        }

        let mut min_angle = f32::INFINITY;
        let mut max_ray_distance = 0.0_f32;
        for left_index in 0..observations.len() {
            for right_index in (left_index + 1)..observations.len() {
                let left = observations[left_index];
                let right = observations[right_index];
                let left_ray = left.image.pose.pixel_ray(
                    left.intrinsics,
                    left.feature.pixel,
                    config.min_depth,
                    f32::MAX,
                )?;
                let right_ray = right.image.pose.pixel_ray(
                    right.intrinsics,
                    right.feature.pixel,
                    config.min_depth,
                    f32::MAX,
                )?;
                min_angle = min_angle.min(ray_angle(left_ray, right_ray));
                max_ray_distance = max_ray_distance.max(point_to_ray_distance(position, left_ray));
                max_ray_distance = max_ray_distance.max(point_to_ray_distance(position, right_ray));
            }
        }

        if min_angle < config.min_angle_radians {
            return Err(invalid_argument("track triangulation angle is too small"));
        }
        if max_ray_distance > config.max_ray_distance {
            return Err(invalid_argument(format!(
                "track ray distance {max_ray_distance:.3} exceeds maximum {:.3}",
                config.max_ray_distance
            )));
        }

        Ok(TriangulatedPoint {
            position,
            ray_distance: max_ray_distance,
            angle_radians: min_angle,
            reprojection_error,
        })
    }

    pub fn insert_triangulated_track(
        &mut self,
        track: Track,
        config: TriangulationConfig,
    ) -> Result<Point3dId> {
        let triangulated = self.triangulate_track(&track, config)?;
        let color = self.track_color(&track)?;
        self.insert_point(
            triangulated.position,
            color,
            track,
            triangulated.reprojection_error,
        )
    }

    pub fn retain_points_with_max_error(&mut self, max_reprojection_error: f32) -> Result<()> {
        validate_finite(max_reprojection_error, "max_reprojection_error")?;
        if max_reprojection_error < 0.0 {
            return Err(invalid_argument(
                "max_reprojection_error must be greater than or equal to zero",
            ));
        }
        self.points
            .retain(|_, point| point.reprojection_error <= max_reprojection_error);
        Ok(())
    }

    pub fn export_points_as_ply(&self) -> Result<String> {
        let mut output = String::new();
        writeln!(output, "ply").expect("write to String cannot fail");
        writeln!(output, "format ascii 1.0").expect("write to String cannot fail");
        writeln!(output, "element vertex {}", self.points.len())
            .expect("write to String cannot fail");
        writeln!(output, "property float x").expect("write to String cannot fail");
        writeln!(output, "property float y").expect("write to String cannot fail");
        writeln!(output, "property float z").expect("write to String cannot fail");
        writeln!(output, "property uchar red").expect("write to String cannot fail");
        writeln!(output, "property uchar green").expect("write to String cannot fail");
        writeln!(output, "property uchar blue").expect("write to String cannot fail");
        writeln!(output, "end_header").expect("write to String cannot fail");
        for point in self.points.values() {
            point.validate()?;
            let color = point.color.clamp01();
            writeln!(
                output,
                "{} {} {} {} {} {}",
                point.position.x,
                point.position.y,
                point.position.z,
                color_to_u8(color.r),
                color_to_u8(color.g),
                color_to_u8(color.b)
            )
            .expect("write to String cannot fail");
        }
        Ok(output)
    }

    fn track_observations<'a>(&'a self, track: &'a Track) -> Result<Vec<TrackObservation<'a>>> {
        let mut observations = Vec::with_capacity(track.elements.len());
        for element in &track.elements {
            let image = self.image(element.image_id)?;
            let camera = self.camera_for_image(element.image_id)?;
            let feature = image.feature(element.feature_index)?;
            observations.push(TrackObservation {
                image,
                intrinsics: camera.intrinsics,
                feature,
            });
        }
        Ok(observations)
    }

    fn track_color(&self, track: &Track) -> Result<ColorRgb> {
        let observations = self.track_observations(track)?;
        let colors = observations
            .iter()
            .filter_map(|observation| observation.feature.color)
            .collect::<Vec<_>>();
        if colors.is_empty() {
            return Ok(ColorRgb::WHITE);
        }

        let mut color = ColorRgb::BLACK;
        for sample in &colors {
            color += *sample;
        }
        Ok((color * (1.0 / colors.len() as f32)).clamp01())
    }
}

impl Default for SparseReconstruction {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy)]
struct TrackObservation<'a> {
    image: &'a ReconstructionImage,
    intrinsics: CameraIntrinsics,
    feature: Feature2d,
}

pub fn triangulate_observation_pair(
    left_image: &ReconstructionImage,
    left_intrinsics: CameraIntrinsics,
    left_feature: Feature2d,
    right_image: &ReconstructionImage,
    right_intrinsics: CameraIntrinsics,
    right_feature: Feature2d,
    config: TriangulationConfig,
) -> Result<TriangulatedPoint> {
    left_image.validate()?;
    right_image.validate()?;
    left_intrinsics.validate()?;
    right_intrinsics.validate()?;
    left_feature.validate()?;
    right_feature.validate()?;
    config.validate()?;

    let left_ray = left_image.pose.pixel_ray(
        left_intrinsics,
        left_feature.pixel,
        config.min_depth,
        f32::MAX,
    )?;
    let right_ray = right_image.pose.pixel_ray(
        right_intrinsics,
        right_feature.pixel,
        config.min_depth,
        f32::MAX,
    )?;
    let angle_radians = ray_angle(left_ray, right_ray);
    if angle_radians < config.min_angle_radians {
        return Err(invalid_argument("triangulation angle is too small"));
    }

    let closest = closest_points_between_rays(left_ray, right_ray)?;
    if closest.left_t < config.min_depth || closest.right_t < config.min_depth {
        return Err(invalid_argument("triangulated point is behind a camera"));
    }
    if closest.distance > config.max_ray_distance {
        return Err(invalid_argument(format!(
            "ray distance {:.3} exceeds maximum {:.3}",
            closest.distance, config.max_ray_distance
        )));
    }

    let position = midpoint(closest.left_point, closest.right_point);
    let left_error = reprojection_error(left_image.pose, left_intrinsics, position, left_feature)?;
    let right_error =
        reprojection_error(right_image.pose, right_intrinsics, position, right_feature)?;
    let reprojection_error = (left_error + right_error) * 0.5;
    if reprojection_error > config.max_reprojection_error {
        return Err(invalid_argument(format!(
            "reprojection error {reprojection_error:.3} exceeds maximum {:.3}",
            config.max_reprojection_error
        )));
    }

    Ok(TriangulatedPoint {
        position,
        ray_distance: closest.distance,
        angle_radians,
        reprojection_error,
    })
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ClosestRayPoints {
    left_point: Vec3,
    right_point: Vec3,
    left_t: f32,
    right_t: f32,
    distance: f32,
}

fn closest_points_between_rays(left: Ray, right: Ray) -> Result<ClosestRayPoints> {
    left.validate()?;
    right.validate()?;
    let offset = left.origin - right.origin;
    let a = left.direction.dot(left.direction);
    let b = left.direction.dot(right.direction);
    let c = left.direction.dot(offset);
    let e = right.direction.dot(right.direction);
    let f = right.direction.dot(offset);
    let denom = a.mul_add(e, -(b * b));
    if denom.abs() <= 1.0e-6 {
        return Err(invalid_argument("rays are nearly parallel"));
    }

    let left_t = b.mul_add(f, -(c * e)) / denom;
    let right_t = a.mul_add(f, -(b * c)) / denom;
    let left_point = left.at(left_t);
    let right_point = right.at(right_t);
    Ok(ClosestRayPoints {
        left_point,
        right_point,
        left_t,
        right_t,
        distance: (left_point - right_point).length(),
    })
}

pub fn project_point(
    pose: CameraPose,
    intrinsics: CameraIntrinsics,
    point: Vec3,
) -> Result<Option<Vec2>> {
    pose.validate()?;
    intrinsics.validate()?;
    if !point.is_finite() {
        return Err(invalid_argument("point must be finite"));
    }
    let camera_space = pose.world_to_camera_point(point);
    if camera_space.z <= 0.0 {
        return Ok(None);
    }
    Ok(Some(Vec2::new(
        intrinsics.fx * (camera_space.x / camera_space.z) + intrinsics.cx,
        intrinsics.fy * (camera_space.y / camera_space.z) + intrinsics.cy,
    )))
}

pub fn reprojection_error(
    pose: CameraPose,
    intrinsics: CameraIntrinsics,
    point: Vec3,
    feature: Feature2d,
) -> Result<f32> {
    feature.validate()?;
    let projected = project_point(pose, intrinsics, point)?
        .ok_or_else(|| invalid_argument("point projects behind the camera"))?;
    Ok((projected - feature.pixel).length())
}

fn mean_track_reprojection_error(
    observations: &[TrackObservation<'_>],
    position: Vec3,
) -> Result<f32> {
    let mut sum = 0.0_f32;
    for observation in observations {
        sum += reprojection_error(
            observation.image.pose,
            observation.intrinsics,
            position,
            observation.feature,
        )?;
    }
    Ok(sum / observations.len() as f32)
}

fn average_vec3(points: &[Vec3]) -> Vec3 {
    let mut sum = Vec3::ZERO;
    for point in points {
        sum += *point;
    }
    sum / points.len() as f32
}

fn midpoint(left: Vec3, right: Vec3) -> Vec3 {
    (left + right) * 0.5
}

pub fn ray_angle(left: Ray, right: Ray) -> f32 {
    left.direction
        .dot(right.direction)
        .clamp(-1.0, 1.0)
        .abs()
        .acos()
}

fn point_to_ray_distance(point: Vec3, ray: Ray) -> f32 {
    let offset = point - ray.origin;
    offset.cross(ray.direction).length()
}

fn color_to_u8(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(left: f32, right: f32) {
        assert!(
            (left - right).abs() < 1.0e-3,
            "expected {left} to be approximately {right}"
        );
    }

    fn test_intrinsics() -> CameraIntrinsics {
        CameraIntrinsics::new(101, 101, 100.0, 100.0, 50.0, 50.0).unwrap()
    }

    fn feature_from_projection(
        pose: CameraPose,
        intrinsics: CameraIntrinsics,
        point: Vec3,
        color: ColorRgb,
    ) -> Feature2d {
        Feature2d::new(project_point(pose, intrinsics, point).unwrap().unwrap())
            .unwrap()
            .with_color(color)
            .unwrap()
    }

    #[test]
    fn binary_matching_uses_hamming_ratio_and_cross_check() {
        let keypoint = Feature2d::new(Vec2::ZERO).unwrap();
        let left = vec![
            BinaryFeature::new(keypoint, [0b0000_0000]).unwrap(),
            BinaryFeature::new(keypoint, [0b1111_0000]).unwrap(),
        ];
        let right = vec![
            BinaryFeature::new(keypoint, [0b0000_0001]).unwrap(),
            BinaryFeature::new(keypoint, [0b1111_1111]).unwrap(),
        ];

        let matches = match_binary_features(&left, &right, MatchConfig::default()).unwrap();

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].left_feature, 0);
        assert_eq!(matches[0].right_feature, 0);
        assert_eq!(matches[0].distance, 1);
    }

    #[test]
    fn hamming_distance_rejects_mismatched_descriptor_lengths() {
        let error = hamming_distance(&[0b0000_0000], &[0b0000_0000, 0b1111_1111]).unwrap_err();

        assert!(error
            .to_string()
            .contains("binary descriptors must have the same length"));
    }

    #[test]
    fn binary_matching_rejects_inconsistent_descriptor_lengths() {
        let keypoint = Feature2d::new(Vec2::ZERO).unwrap();
        let left = vec![
            BinaryFeature::new(keypoint, [0b0000_0000]).unwrap(),
            BinaryFeature::new(keypoint, [0b1111_0000, 0b0000_1111]).unwrap(),
        ];
        let right = vec![BinaryFeature::new(keypoint, [0b0000_0001]).unwrap()];

        let error = match_binary_features(&left, &right, MatchConfig::default()).unwrap_err();

        assert!(error
            .to_string()
            .contains("left binary descriptors must have consistent lengths"));
    }

    #[test]
    fn track_builder_merges_pairwise_matches() {
        let pair_01 = ImagePairMatches::new(
            ImageId(0),
            ImageId(1),
            [FeatureMatch::new(0, 2, 4, 0.9).unwrap()],
        )
        .unwrap();
        let pair_12 = ImagePairMatches::new(
            ImageId(1),
            ImageId(2),
            [FeatureMatch::new(2, 3, 5, 0.8).unwrap()],
        )
        .unwrap();

        let tracks = build_tracks(&[pair_01, pair_12]).unwrap();

        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].elements.len(), 3);
    }

    #[test]
    fn track_builder_drops_tracks_with_duplicate_image_observations() {
        let pair_01 = ImagePairMatches::new(
            ImageId(0),
            ImageId(1),
            [FeatureMatch::new(0, 0, 4, 0.9).unwrap()],
        )
        .unwrap();
        let pair_12 = ImagePairMatches::new(
            ImageId(1),
            ImageId(2),
            [FeatureMatch::new(1, 0, 5, 0.8).unwrap()],
        )
        .unwrap();
        let pair_02 = ImagePairMatches::new(
            ImageId(0),
            ImageId(2),
            [FeatureMatch::new(0, 0, 3, 0.95).unwrap()],
        )
        .unwrap();

        let tracks = build_tracks(&[pair_01, pair_12, pair_02]).unwrap();

        assert!(tracks.is_empty());
    }

    #[test]
    fn reconstruction_rejects_duplicate_camera_and_missing_camera_image() {
        let intrinsics = test_intrinsics();
        let mut reconstruction = SparseReconstruction::new();
        reconstruction
            .add_camera(ReconstructionCamera::new(CameraId(0), intrinsics).unwrap())
            .unwrap();

        let duplicate = reconstruction
            .add_camera(ReconstructionCamera::new(CameraId(0), intrinsics).unwrap())
            .unwrap_err();
        assert!(duplicate.to_string().contains("duplicate camera id"));

        let image = ReconstructionImage::new(
            ImageId(0),
            CameraId(99),
            "missing-camera",
            CameraPose::identity(),
        )
        .unwrap();
        let missing_camera = reconstruction.add_image(image).unwrap_err();
        assert!(missing_camera
            .to_string()
            .contains("references missing camera"));
    }

    #[test]
    fn project_point_culls_points_behind_camera() {
        let projected = project_point(
            CameraPose::identity(),
            test_intrinsics(),
            Vec3::new(0.0, 0.0, -1.0),
        )
        .unwrap();

        assert!(projected.is_none());
    }

    #[test]
    fn reprojection_error_measures_pixel_distance() {
        let intrinsics = test_intrinsics();
        let point = Vec3::new(0.0, 0.0, 4.0);
        let pixel = project_point(CameraPose::identity(), intrinsics, point)
            .unwrap()
            .unwrap();
        let shifted_feature = Feature2d::new(pixel + Vec2::new(3.0, 4.0)).unwrap();

        let error =
            reprojection_error(CameraPose::identity(), intrinsics, point, shifted_feature).unwrap();

        approx_eq(error, 5.0);
    }

    #[test]
    fn triangulates_two_observations() {
        let intrinsics = test_intrinsics();
        let left_pose = CameraPose::identity();
        let right_pose =
            CameraPose::look_at(Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 4.0), Vec3::Y)
                .unwrap();
        let point = Vec3::new(0.0, 0.0, 4.0);
        let left_feature = feature_from_projection(left_pose, intrinsics, point, ColorRgb::WHITE);
        let right_feature = feature_from_projection(right_pose, intrinsics, point, ColorRgb::WHITE);
        let left_image =
            ReconstructionImage::new(ImageId(0), CameraId(0), "left", left_pose).unwrap();
        let right_image =
            ReconstructionImage::new(ImageId(1), CameraId(0), "right", right_pose).unwrap();

        let triangulated = triangulate_observation_pair(
            &left_image,
            intrinsics,
            left_feature,
            &right_image,
            intrinsics,
            right_feature,
            TriangulationConfig::default(),
        )
        .unwrap();

        approx_eq(triangulated.position.x, point.x);
        approx_eq(triangulated.position.y, point.y);
        approx_eq(triangulated.position.z, point.z);
        assert!(triangulated.reprojection_error < 1.0e-3);
    }

    #[test]
    fn triangulation_rejects_parallel_rays() {
        let intrinsics = test_intrinsics();
        let left_pose = CameraPose::identity();
        let right_pose =
            CameraPose::new(Vec3::new(1.0, 0.0, 0.0), Vec3::X, Vec3::Y, Vec3::Z).unwrap();
        let feature = Feature2d::new(Vec2::new(50.0, 50.0)).unwrap();
        let left_image =
            ReconstructionImage::new(ImageId(0), CameraId(0), "left", left_pose).unwrap();
        let right_image =
            ReconstructionImage::new(ImageId(1), CameraId(0), "right", right_pose).unwrap();

        let error = triangulate_observation_pair(
            &left_image,
            intrinsics,
            feature,
            &right_image,
            intrinsics,
            feature,
            TriangulationConfig::default(),
        )
        .unwrap_err();

        assert!(error
            .to_string()
            .contains("triangulation angle is too small"));
    }

    #[test]
    fn sparse_reconstruction_inserts_triangulated_track_and_exports_ply() {
        let intrinsics = test_intrinsics();
        let left_pose = CameraPose::identity();
        let right_pose =
            CameraPose::look_at(Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 4.0), Vec3::Y)
                .unwrap();
        let point = Vec3::new(0.0, 0.0, 4.0);
        let red = ColorRgb::new(1.0, 0.0, 0.0);

        let mut reconstruction = SparseReconstruction::new();
        reconstruction
            .add_camera(ReconstructionCamera::new(CameraId(0), intrinsics).unwrap())
            .unwrap();

        let mut left_image =
            ReconstructionImage::new(ImageId(0), CameraId(0), "left", left_pose).unwrap();
        let mut right_image =
            ReconstructionImage::new(ImageId(1), CameraId(0), "right", right_pose).unwrap();
        let left_feature = left_image
            .add_feature(feature_from_projection(left_pose, intrinsics, point, red))
            .unwrap();
        let right_feature = right_image
            .add_feature(feature_from_projection(right_pose, intrinsics, point, red))
            .unwrap();
        reconstruction.add_image(left_image).unwrap();
        reconstruction.add_image(right_image).unwrap();

        let track = Track::new([
            TrackElement::new(ImageId(0), left_feature),
            TrackElement::new(ImageId(1), right_feature),
        ])
        .unwrap();
        let point_id = reconstruction
            .insert_triangulated_track(track, TriangulationConfig::default())
            .unwrap();

        let sparse_point = reconstruction.points().get(&point_id).unwrap();
        approx_eq(sparse_point.position.z, 4.0);
        assert!(sparse_point.reprojection_error < 1.0e-3);
        let ply = reconstruction.export_points_as_ply().unwrap();
        assert!(ply.contains("element vertex 1"));
        assert!(ply.contains("255 0 0"));
    }

    #[test]
    fn sparse_reconstruction_filters_points_by_reprojection_error() {
        let mut reconstruction = SparseReconstruction::new();
        let track = Track::new([
            TrackElement::new(ImageId(0), 0),
            TrackElement::new(ImageId(1), 0),
        ])
        .unwrap();
        reconstruction
            .insert_point(
                Vec3::new(0.0, 0.0, 1.0),
                ColorRgb::WHITE,
                track.clone(),
                0.5,
            )
            .unwrap();
        reconstruction
            .insert_point(Vec3::new(1.0, 0.0, 1.0), ColorRgb::WHITE, track, 3.0)
            .unwrap();

        reconstruction.retain_points_with_max_error(1.0).unwrap();

        assert_eq!(reconstruction.points().len(), 1);
        let remaining = reconstruction.points().values().next().unwrap();
        approx_eq(remaining.reprojection_error, 0.5);
    }
}
