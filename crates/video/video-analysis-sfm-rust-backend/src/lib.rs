#![doc = include_str!("../README.md")]

use video_analysis_core::Result;
use video_analysis_reconstruction::{match_binary_features, ImagePairMatches, MatchConfig};
use video_analysis_sfm::{
    KnownPoseSparseMapper, SfmBackend, SfmPipelineOutput, SfmRequest, SparseMapper,
};

#[derive(Debug, Clone, Copy, PartialEq)]
/// Data type for Rust-native SfM backend configuration.
pub struct RustSfmBackendConfig {
    /// Match configuration for supplied binary features.
    pub match_config: MatchConfig,
    /// Whether to build missing adjacent image-pair matches from features.
    pub match_adjacent_when_missing: bool,
}

impl Default for RustSfmBackendConfig {
    fn default() -> Self {
        Self {
            match_config: MatchConfig::default(),
            match_adjacent_when_missing: true,
        }
    }
}

#[derive(Debug, Clone, Default)]
/// Rust-native known-pose SfM backend.
pub struct RustKnownPoseSfmBackend {
    /// Configuration value.
    pub config: RustSfmBackendConfig,
}

impl RustKnownPoseSfmBackend {
    /// Creates a new value.
    pub fn new(config: RustSfmBackendConfig) -> Self {
        Self { config }
    }

    fn request_with_matches(&self, request: &SfmRequest) -> Result<SfmRequest> {
        if !request.pair_matches.is_empty() || !self.config.match_adjacent_when_missing {
            return Ok(request.clone());
        }
        let mut pairs = Vec::new();
        for images in request.images.windows(2) {
            let left = &images[0];
            let right = &images[1];
            if left.features.is_empty() || right.features.is_empty() {
                continue;
            }
            let matches =
                match_binary_features(&left.features, &right.features, self.config.match_config)?;
            pairs.push(ImagePairMatches::new(left.id, right.id, matches)?);
        }
        let mut next = request.clone();
        next.pair_matches = pairs;
        next.validate()?;
        Ok(next)
    }
}

impl SfmBackend for RustKnownPoseSfmBackend {
    fn name(&self) -> &'static str {
        "rust-known-pose-sfm-backend"
    }

    fn reconstruct(&mut self, request: &SfmRequest) -> Result<SfmPipelineOutput> {
        let request = self.request_with_matches(request)?;
        let mut mapper = KnownPoseSparseMapper;
        let mut output = mapper.reconstruct_sparse(&request)?;
        output.report.backend = self.name().to_string();
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use video_analysis_radiance_fields::{CameraIntrinsics, CameraPose, Vec2, Vec3};
    use video_analysis_reconstruction::{
        BinaryFeature, CameraId, Feature2d, ImageId, ImagePairMatches,
    };
    use video_analysis_sfm::{SfmInputImage, SfmPipeline, SfmRequest};

    use super::*;

    #[test]
    fn rust_backend_uses_supplied_matches() {
        let intrinsics = CameraIntrinsics::new(32, 32, 30.0, 30.0, 15.0, 15.0).unwrap();
        let left = SfmInputImage::new(ImageId(1), CameraId(1), "a.png", intrinsics)
            .unwrap()
            .pose(CameraPose::identity())
            .unwrap()
            .features([
                BinaryFeature::new(Feature2d::new(Vec2::new(15.0, 15.0)).unwrap(), [0_u8]).unwrap(),
            ])
            .unwrap();
        let right_pose = CameraPose::look_at(
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 3.0),
            Vec3::new(0.0, 1.0, 0.0),
        )
        .unwrap();
        let right = SfmInputImage::new(ImageId(2), CameraId(2), "b.png", intrinsics)
            .unwrap()
            .pose(right_pose)
            .unwrap()
            .features([
                BinaryFeature::new(Feature2d::new(Vec2::new(15.0, 15.0)).unwrap(), [0_u8]).unwrap(),
            ])
            .unwrap();
        let pair = ImagePairMatches::new(
            ImageId(1),
            ImageId(2),
            [video_analysis_reconstruction::FeatureMatch::new(0, 0, 0, 1.0).unwrap()],
        )
        .unwrap();
        let request = SfmRequest::new([left, right])
            .unwrap()
            .pair_matches([pair])
            .unwrap();
        let mut pipeline = SfmPipeline::new(RustKnownPoseSfmBackend::default());
        let output = pipeline.run(&request).unwrap();
        assert_eq!(output.report.backend, "rust-known-pose-sfm-backend");
        assert_eq!(output.report.registered_image_count, 2);
    }
}
