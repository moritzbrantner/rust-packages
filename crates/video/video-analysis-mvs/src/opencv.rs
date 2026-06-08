use video_analysis_core::{DetectError, Result};

use crate::{DenseReconstructor, MvsOutput, MvsRequest};

fn unavailable() -> DetectError {
    DetectError::InvalidArgument(
        "OpenCV MVS execution is intentionally optional and unavailable in this build; use video-analysis-mvs provider planning or a concrete dense reconstruction backend."
            .to_string(),
    )
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
    use video_analysis_radiance_fields::{CameraIntrinsics, CameraPose, CameraView, CameraViewSet};
    use video_analysis_reconstruction::SparseReconstruction;

    use super::*;

    #[test]
    fn opencv_mvs_backend_returns_unavailable_error() {
        let intrinsics = CameraIntrinsics::pinhole(16, 16, 1.0).unwrap();
        let views = CameraViewSet {
            views: vec![CameraView {
                id: 1,
                name: "a.png".to_string(),
                intrinsics,
                distortion: None,
                pose: CameraPose::identity(),
            }],
        };
        let request = MvsRequest::new(SparseReconstruction::new(), views).unwrap();
        let mut backend = OpenCvMvsBackend;
        let error = backend.reconstruct_dense(&request).unwrap_err();
        assert!(error.to_string().contains("OpenCV MVS execution"));
    }
}
