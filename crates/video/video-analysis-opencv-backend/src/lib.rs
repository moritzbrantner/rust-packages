#![doc = include_str!("../README.md")]

use video_analysis_core::{DetectError, Result};
use video_analysis_mvs::{DenseReconstructor, MvsOutput, MvsRequest};
use video_analysis_sfm::{SfmBackend, SfmPipelineOutput, SfmRequest};

fn unavailable() -> DetectError {
    DetectError::InvalidArgument(
        "OpenCV backend is intentionally optional and has no command adapter; use image-analysis-detection for color/object heuristics, video-analysis-onnx for learned detectors, or video-analysis-sfm-rust-backend for Rust SfM"
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
        #[cfg(feature = "opencv-backend")]
        {
            Self {
                feature_extraction: false,
                descriptor_matching: false,
                geometric_verification: false,
                sparse_sfm: false,
                dense_mvs: false,
            }
        }
        #[cfg(not(feature = "opencv-backend"))]
        {
            Self {
                feature_extraction: false,
                descriptor_matching: false,
                geometric_verification: false,
                sparse_sfm: false,
                dense_mvs: false,
            }
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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
/// Data type for OpenCV MVS backend.
pub struct OpenCvMvsBackend;

impl DenseReconstructor for OpenCvMvsBackend {
    fn name(&self) -> &'static str {
        "opencv-mvs-backend"
    }

    fn reconstruct_dense(&mut self, _request: &MvsRequest) -> Result<MvsOutput> {
        Err(unavailable())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_feature_gated_capabilities_without_native_linking() {
        let backend = OpenCvSfmBackend::default();
        let capabilities = backend.capabilities();
        assert!(!capabilities.sparse_sfm);
        assert_eq!(backend.config.detector, "SIFT");
    }
}
