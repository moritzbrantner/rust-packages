#![doc = include_str!("../README.md")]

pub mod surface;
pub use image_analysis_detection::{
    FaceBox, FaceDetection, FaceDetectionPreset, FaceDetectorBackend, FaceLandmarks,
};
pub use image_analysis_segmentation::{
    default_sam_model_spec, ModelBackedImageSegmentationBackend, SamImagePreset,
};
pub use image_analysis_tasks::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compatibility_reexports_image_task_contracts() {
        assert_eq!(parse_task("classify"), Some(ImageTask::ImageClassification));
        assert_eq!(
            ImageEmbeddingPreset::XenovaClipVitBasePatch32Onnx
                .model_spec()
                .repo_id_value(),
            Some("Xenova/clip-vit-base-patch32")
        );
        assert!(FaceBox::new(0.1, 0.2, 0.3, 0.4).is_ok());
    }
}
