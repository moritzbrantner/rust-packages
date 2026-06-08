use video_analysis_core::{DetectError, Result};

use crate::{SfmBackend, SfmPipelineOutput, SfmRequest};

fn unavailable() -> DetectError {
    DetectError::InvalidArgument(
        "OpenCV SfM execution is intentionally optional and unavailable in this build; use video-analysis-sfm provider planning or a concrete SfM backend."
            .to_string(),
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Data type for OpenCV backend capabilities.
pub struct OpenCvBackendCapabilities {
    /// Whether feature extraction is available.
    pub feature_extraction: bool,
    /// Whether descriptor matching is available.
    pub descriptor_matching: bool,
    /// Whether geometric verification is available.
    pub geometric_verification: bool,
    /// Whether sparse SfM is available.
    pub sparse_sfm: bool,
    /// Whether dense MVS is available.
    pub dense_mvs: bool,
}

impl OpenCvBackendCapabilities {
    /// Returns capabilities for the current build.
    pub const fn current() -> Self {
        Self {
            feature_extraction: false,
            descriptor_matching: false,
            geometric_verification: false,
            sparse_sfm: false,
            dense_mvs: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Data type for OpenCV SfM backend configuration.
pub struct OpenCvSfmBackendConfig {
    /// Feature detector name such as SIFT, ORB, or AKAZE.
    pub detector: String,
    /// Descriptor matcher name such as BFMatcher or FlannBased.
    pub matcher: String,
    /// Whether to prefer opencv_contrib sfm when available.
    pub prefer_sfm_module: bool,
}

impl Default for OpenCvSfmBackendConfig {
    fn default() -> Self {
        Self {
            detector: "SIFT".to_string(),
            matcher: "BFMatcher".to_string(),
            prefer_sfm_module: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
/// Data type for OpenCV SfM backend.
pub struct OpenCvSfmBackend {
    /// Configuration value.
    pub config: OpenCvSfmBackendConfig,
}

impl OpenCvSfmBackend {
    /// Creates a new value.
    pub fn new(config: OpenCvSfmBackendConfig) -> Self {
        Self { config }
    }

    /// Returns current capabilities.
    pub const fn capabilities(&self) -> OpenCvBackendCapabilities {
        OpenCvBackendCapabilities::current()
    }
}

impl SfmBackend for OpenCvSfmBackend {
    fn name(&self) -> &'static str {
        "opencv-sfm-backend"
    }

    fn reconstruct(&mut self, _request: &SfmRequest) -> Result<SfmPipelineOutput> {
        Err(unavailable())
    }
}

#[cfg(test)]
mod tests {
    use video_analysis_radiance_fields::CameraIntrinsics;
    use video_analysis_reconstruction::{CameraId, ImageId};

    use super::*;
    use crate::SfmInputImage;

    #[test]
    fn opencv_sfm_capabilities_are_unavailable_without_native_binding() {
        let backend = OpenCvSfmBackend::default();
        let capabilities = backend.capabilities();
        assert!(!capabilities.feature_extraction);
        assert!(!capabilities.descriptor_matching);
        assert!(!capabilities.geometric_verification);
        assert!(!capabilities.sparse_sfm);
        assert!(!capabilities.dense_mvs);
    }

    #[test]
    fn opencv_sfm_backend_returns_unavailable_error() {
        let intrinsics = CameraIntrinsics::pinhole(16, 16, 1.0).unwrap();
        let image = SfmInputImage::new(ImageId(1), CameraId(1), "a.png", intrinsics).unwrap();
        let request = SfmRequest::new([image]).unwrap();
        let mut backend = OpenCvSfmBackend::default();
        let error = backend.reconstruct(&request).unwrap_err();
        assert!(error.to_string().contains("OpenCV SfM execution"));
    }

    #[test]
    fn opencv_sfm_config_defaults_to_sift_bfmatcher() {
        let config = OpenCvSfmBackendConfig::default();
        assert_eq!(config.detector, "SIFT");
        assert_eq!(config.matcher, "BFMatcher");
        assert!(config.prefer_sfm_module);
    }
}
