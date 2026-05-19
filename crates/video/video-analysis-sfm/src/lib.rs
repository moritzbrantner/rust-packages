#![doc = include_str!("../README.md")]

use std::collections::{BTreeMap, BTreeSet};

use video_analysis_core::{DetectError, Result};
use video_analysis_radiance_fields::{CameraIntrinsics, CameraPose};
use video_analysis_reconstruction::{
    build_tracks, BinaryFeature, CameraId, FeatureMatch, ImageId, ImagePairMatches,
    ReconstructionCamera, ReconstructionImage, SparseReconstruction, TriangulationConfig,
};

fn invalid_argument(message: impl Into<String>) -> DetectError {
    DetectError::InvalidArgument(message.into())
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for an input image in an SfM run.
pub struct SfmInputImage {
    /// Identifier for this value.
    pub id: ImageId,
    /// Camera identifier associated with this value.
    pub camera_id: CameraId,
    /// Human-readable image name.
    pub name: String,
    /// Camera intrinsics for this image.
    pub intrinsics: CameraIntrinsics,
    /// Optional known or initialized camera pose.
    pub pose: Option<CameraPose>,
    /// Optional pre-extracted binary features.
    pub features: Vec<BinaryFeature>,
}

impl SfmInputImage {
    /// Creates a new value.
    pub fn new(
        id: ImageId,
        camera_id: CameraId,
        name: impl Into<String>,
        intrinsics: CameraIntrinsics,
    ) -> Result<Self> {
        intrinsics.validate()?;
        Ok(Self {
            id,
            camera_id,
            name: name.into(),
            intrinsics,
            pose: None,
            features: Vec::new(),
        })
    }

    /// Returns this value with pose.
    pub fn pose(mut self, pose: CameraPose) -> Result<Self> {
        pose.validate()?;
        self.pose = Some(pose);
        Ok(self)
    }

    /// Returns this value with features.
    pub fn features(mut self, features: impl Into<Vec<BinaryFeature>>) -> Result<Self> {
        self.features = features.into();
        self.validate()?;
        Ok(self)
    }

    /// Validates this value.
    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            return Err(invalid_argument("SfM image name must not be empty"));
        }
        self.intrinsics.validate()?;
        if let Some(pose) = self.pose {
            pose.validate()?;
        }
        for feature in &self.features {
            feature.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for an SfM request.
pub struct SfmRequest {
    /// Images available to the pipeline.
    pub images: Vec<SfmInputImage>,
    /// Optional pairwise matches supplied by a caller or upstream backend.
    pub pair_matches: Vec<ImagePairMatches>,
    /// Triangulation settings.
    pub triangulation: TriangulationConfig,
}

impl SfmRequest {
    /// Creates a new value.
    pub fn new(images: impl Into<Vec<SfmInputImage>>) -> Result<Self> {
        let request = Self {
            images: images.into(),
            pair_matches: Vec::new(),
            triangulation: TriangulationConfig::default(),
        };
        request.validate()?;
        Ok(request)
    }

    /// Returns this value with pair matches.
    pub fn pair_matches(mut self, pair_matches: impl Into<Vec<ImagePairMatches>>) -> Result<Self> {
        self.pair_matches = pair_matches.into();
        self.validate()?;
        Ok(self)
    }

    /// Returns this value with triangulation config.
    pub fn triangulation(mut self, triangulation: TriangulationConfig) -> Result<Self> {
        triangulation.validate()?;
        self.triangulation = triangulation;
        Ok(self)
    }

    /// Validates this value.
    pub fn validate(&self) -> Result<()> {
        if self.images.is_empty() {
            return Err(invalid_argument("SfM request requires at least one image"));
        }
        let mut image_ids = BTreeMap::new();
        for image in &self.images {
            image.validate()?;
            if image_ids.insert(image.id, image.camera_id).is_some() {
                return Err(invalid_argument(format!(
                    "duplicate image id {:?}",
                    image.id
                )));
            }
        }
        for pair in &self.pair_matches {
            pair.validate()?;
            if !image_ids.contains_key(&pair.left_image_id) {
                return Err(invalid_argument(format!(
                    "pair references missing left image {:?}",
                    pair.left_image_id
                )));
            }
            if !image_ids.contains_key(&pair.right_image_id) {
                return Err(invalid_argument(format!(
                    "pair references missing right image {:?}",
                    pair.right_image_id
                )));
            }
        }
        self.triangulation.validate()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Variants describing geometric verification status.
pub enum VerificationStatus {
    /// Matches passed backend verification.
    Verified,
    /// Matches were not verified by a geometric model.
    Unverified,
    /// Matches were rejected.
    Rejected,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for a verified image pair.
pub struct VerifiedImagePair {
    /// The left image identifier value.
    pub left_image_id: ImageId,
    /// The right image identifier value.
    pub right_image_id: ImageId,
    /// Matches retained after verification.
    pub matches: Vec<FeatureMatch>,
    /// Verification status.
    pub status: VerificationStatus,
    /// Optional model name such as fundamental, essential, or homography.
    pub model: Option<String>,
    /// Number of input matches before filtering.
    pub input_match_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for a registered image.
pub struct RegisteredImage {
    /// The image identifier value.
    pub image_id: ImageId,
    /// Estimated or known camera pose.
    pub pose: CameraPose,
    /// Number of observations used for registration.
    pub observation_count: usize,
    /// Mean reprojection error for this registration, if available.
    pub mean_reprojection_error: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Variants describing bundle-adjustment engine.
pub enum BundleAdjustmentEngine {
    /// No bundle adjustment was run.
    None,
    /// A native engine handled bundle adjustment.
    Native,
    /// Apex Solver handled bundle adjustment.
    ApexSolver,
    /// Sparse Levenberg-Marquardt handled bundle adjustment.
    LevenbergMarquardtSparse,
    /// A custom engine handled bundle adjustment.
    Custom,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for bundle-adjustment report.
pub struct BundleAdjustmentReport {
    /// Engine used for this report.
    pub engine: BundleAdjustmentEngine,
    /// Number of iterations completed.
    pub iterations: usize,
    /// Initial mean reprojection error.
    pub initial_mean_reprojection_error: Option<f32>,
    /// Final mean reprojection error.
    pub final_mean_reprojection_error: Option<f32>,
    /// Whether the backend reported convergence.
    pub converged: bool,
}

impl Default for BundleAdjustmentReport {
    fn default() -> Self {
        Self {
            engine: BundleAdjustmentEngine::None,
            iterations: 0,
            initial_mean_reprojection_error: None,
            final_mean_reprojection_error: None,
            converged: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for SfM run report.
pub struct SfmRunReport {
    /// Backend name associated with this run.
    pub backend: String,
    /// Number of input images.
    pub image_count: usize,
    /// Number of registered images.
    pub registered_image_count: usize,
    /// Number of cameras in the reconstruction.
    pub camera_count: usize,
    /// Number of sparse points in the reconstruction.
    pub sparse_point_count: usize,
    /// Mean reprojection error across sparse points, if any.
    pub mean_reprojection_error: Option<f32>,
    /// Histogram keyed by track length.
    pub track_length_histogram: BTreeMap<usize, usize>,
    /// Bundle-adjustment report.
    pub bundle_adjustment: BundleAdjustmentReport,
}

impl SfmRunReport {
    /// Builds a report from a sparse reconstruction.
    pub fn from_reconstruction(
        backend: impl Into<String>,
        image_count: usize,
        reconstruction: &SparseReconstruction,
    ) -> Result<Self> {
        reconstruction_report(backend, image_count, reconstruction)
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for SfM pipeline output.
pub struct SfmPipelineOutput {
    /// Sparse reconstruction produced by the backend.
    pub reconstruction: SparseReconstruction,
    /// Pipeline report.
    pub report: SfmRunReport,
    /// Verified pairs, if exposed by the backend.
    pub verified_pairs: Vec<VerifiedImagePair>,
    /// Registered images, if exposed by the backend.
    pub registered_images: Vec<RegisteredImage>,
}

/// Trait for feature extractor backends.
pub trait FeatureExtractor {
    /// Extracts binary features for one image.
    fn extract_features(&mut self, image: &SfmInputImage) -> Result<Vec<BinaryFeature>>;
}

/// Trait for feature matcher backends.
pub trait FeatureMatcher {
    /// Matches binary features for two images.
    fn match_features(
        &mut self,
        left_image_id: ImageId,
        left: &[BinaryFeature],
        right_image_id: ImageId,
        right: &[BinaryFeature],
    ) -> Result<ImagePairMatches>;
}

/// Trait for geometric verifier backends.
pub trait GeometricVerifier {
    /// Verifies matches for an image pair.
    fn verify_pair(&mut self, pair: &ImagePairMatches) -> Result<VerifiedImagePair>;
}

/// Trait for image registration backends.
pub trait ImageRegistrar {
    /// Registers images into a sparse reconstruction.
    fn register_images(&mut self, request: &SfmRequest) -> Result<Vec<RegisteredImage>>;
}

/// Trait for bundle adjuster backends.
pub trait BundleAdjuster {
    /// Adjusts a sparse reconstruction.
    fn adjust_bundle(
        &mut self,
        reconstruction: &mut SparseReconstruction,
    ) -> Result<BundleAdjustmentReport>;
}

/// Trait for sparse mapper backends.
pub trait SparseMapper {
    /// Reconstructs a sparse model.
    fn reconstruct_sparse(&mut self, request: &SfmRequest) -> Result<SfmPipelineOutput>;
}

/// Trait for complete SfM backends.
pub trait SfmBackend {
    /// Returns backend name.
    fn name(&self) -> &'static str;

    /// Reconstructs a sparse model.
    fn reconstruct(&mut self, request: &SfmRequest) -> Result<SfmPipelineOutput>;
}

#[derive(Debug)]
/// Data type for SfM pipeline.
pub struct SfmPipeline<B> {
    backend: B,
}

impl<B: SfmBackend> SfmPipeline<B> {
    /// Creates a new value.
    pub fn new(backend: B) -> Self {
        Self { backend }
    }

    /// Returns backend.
    pub fn backend(&self) -> &B {
        &self.backend
    }

    /// Returns backend mut.
    pub fn backend_mut(&mut self) -> &mut B {
        &mut self.backend
    }

    /// Runs the pipeline.
    pub fn run(&mut self, request: &SfmRequest) -> Result<SfmPipelineOutput> {
        request.validate()?;
        self.backend.reconstruct(request)
    }
}

#[derive(Debug, Clone, Copy, Default)]
/// Sparse mapper for inputs that already contain poses, features, and matches.
pub struct KnownPoseSparseMapper;

impl SfmBackend for KnownPoseSparseMapper {
    fn name(&self) -> &'static str {
        "known-pose-sparse-mapper"
    }

    fn reconstruct(&mut self, request: &SfmRequest) -> Result<SfmPipelineOutput> {
        self.reconstruct_sparse(request)
    }
}

impl SparseMapper for KnownPoseSparseMapper {
    fn reconstruct_sparse(&mut self, request: &SfmRequest) -> Result<SfmPipelineOutput> {
        request.validate()?;
        let mut reconstruction = SparseReconstruction::new();
        let mut inserted_cameras = BTreeSet::new();
        for image in &request.images {
            if inserted_cameras.insert(image.camera_id) {
                reconstruction.add_camera(ReconstructionCamera::new(
                    image.camera_id,
                    image.intrinsics,
                )?)?;
            }
        }
        for image in &request.images {
            let pose = image.pose.ok_or_else(|| {
                invalid_argument(format!(
                    "image {:?} does not have an initialized pose",
                    image.id
                ))
            })?;
            let mut reconstruction_image =
                ReconstructionImage::new(image.id, image.camera_id, image.name.clone(), pose)?;
            for feature in &image.features {
                reconstruction_image.add_feature(feature.keypoint)?;
            }
            reconstruction.add_image(reconstruction_image)?;
        }

        let tracks = build_tracks(&request.pair_matches)?;
        for track in tracks {
            let _ = reconstruction.insert_triangulated_track(track, request.triangulation);
        }

        let registered_images = request
            .images
            .iter()
            .filter_map(|image| {
                image.pose.map(|pose| RegisteredImage {
                    image_id: image.id,
                    pose,
                    observation_count: image.features.len(),
                    mean_reprojection_error: None,
                })
            })
            .collect::<Vec<_>>();

        let report = reconstruction_report(self.name(), request.images.len(), &reconstruction)?;
        Ok(SfmPipelineOutput {
            reconstruction,
            report,
            verified_pairs: request
                .pair_matches
                .iter()
                .map(|pair| VerifiedImagePair {
                    left_image_id: pair.left_image_id,
                    right_image_id: pair.right_image_id,
                    matches: pair.matches.clone(),
                    status: VerificationStatus::Unverified,
                    model: None,
                    input_match_count: pair.matches.len(),
                })
                .collect(),
            registered_images,
        })
    }
}

/// Returns a report for a sparse reconstruction.
pub fn reconstruction_report(
    backend: impl Into<String>,
    image_count: usize,
    reconstruction: &SparseReconstruction,
) -> Result<SfmRunReport> {
    let track_length_histogram = track_length_histogram(reconstruction);
    let mean_reprojection_error = mean_reprojection_error(reconstruction)?;
    Ok(SfmRunReport {
        backend: backend.into(),
        image_count,
        registered_image_count: reconstruction.images().len(),
        camera_count: reconstruction.cameras().len(),
        sparse_point_count: reconstruction.points().len(),
        mean_reprojection_error,
        track_length_histogram,
        bundle_adjustment: BundleAdjustmentReport::default(),
    })
}

/// Returns track-length histogram for a sparse reconstruction.
pub fn track_length_histogram(reconstruction: &SparseReconstruction) -> BTreeMap<usize, usize> {
    let mut histogram = BTreeMap::new();
    for point in reconstruction.points().values() {
        *histogram.entry(point.track.elements.len()).or_insert(0) += 1;
    }
    histogram
}

/// Returns mean reprojection error for sparse points.
pub fn mean_reprojection_error(reconstruction: &SparseReconstruction) -> Result<Option<f32>> {
    let points = reconstruction.points();
    if points.is_empty() {
        return Ok(None);
    }
    let mut sum = 0.0_f32;
    for point in points.values() {
        point.validate()?;
        sum += point.reprojection_error;
    }
    Ok(Some(sum / points.len() as f32))
}

#[cfg(test)]
mod tests {
    use video_analysis_radiance_fields::{ColorRgb, Vec2, Vec3};
    use video_analysis_reconstruction::{Feature2d, FeatureMatch, Track, TrackElement};

    use super::*;

    #[test]
    fn reports_reconstruction_counts_and_track_lengths() {
        let intrinsics = CameraIntrinsics::pinhole(32, 32, 1.0).unwrap();
        let mut reconstruction = SparseReconstruction::new();
        reconstruction
            .add_camera(ReconstructionCamera::new(CameraId(1), intrinsics).unwrap())
            .unwrap();
        let mut image =
            ReconstructionImage::new(ImageId(1), CameraId(1), "a.png", CameraPose::identity())
                .unwrap();
        image
            .add_feature(Feature2d::new(Vec2::new(16.0, 16.0)).unwrap())
            .unwrap();
        reconstruction.add_image(image).unwrap();
        reconstruction
            .insert_point(
                Vec3::new(0.0, 0.0, 2.0),
                ColorRgb::WHITE,
                Track::new([
                    TrackElement::new(ImageId(1), 0),
                    TrackElement::new(ImageId(2), 0),
                ])
                .unwrap(),
                0.5,
            )
            .unwrap();

        let report = reconstruction_report("test", 2, &reconstruction).unwrap();
        assert_eq!(report.camera_count, 1);
        assert_eq!(report.sparse_point_count, 1);
        assert_eq!(report.track_length_histogram[&2], 1);
        assert_eq!(report.mean_reprojection_error, Some(0.5));
    }

    #[test]
    fn validates_known_pose_pipeline_inputs() {
        let intrinsics = CameraIntrinsics::pinhole(32, 32, 1.0).unwrap();
        let image = SfmInputImage::new(ImageId(1), CameraId(1), "a.png", intrinsics)
            .unwrap()
            .pose(CameraPose::identity())
            .unwrap()
            .features([
                BinaryFeature::new(Feature2d::new(Vec2::new(16.0, 16.0)).unwrap(), [0_u8]).unwrap(),
            ])
            .unwrap();
        let request = SfmRequest::new([image]).unwrap();
        let mut pipeline = SfmPipeline::new(KnownPoseSparseMapper);
        let output = pipeline.run(&request).unwrap();
        assert_eq!(output.report.registered_image_count, 1);
        assert_eq!(output.report.sparse_point_count, 0);
    }

    #[test]
    fn verified_pair_preserves_input_match_count() {
        let pair = ImagePairMatches::new(
            ImageId(1),
            ImageId(2),
            [FeatureMatch::new(0, 1, 2, 0.9).unwrap()],
        )
        .unwrap();
        let verified = VerifiedImagePair {
            left_image_id: pair.left_image_id,
            right_image_id: pair.right_image_id,
            matches: pair.matches.clone(),
            status: VerificationStatus::Verified,
            model: Some("fundamental".to_string()),
            input_match_count: pair.matches.len(),
        };
        assert_eq!(verified.input_match_count, 1);
        assert_eq!(verified.model.as_deref(), Some("fundamental"));
    }
}
