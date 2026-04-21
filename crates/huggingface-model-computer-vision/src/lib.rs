//! Hugging Face model category metadata for Computer Vision.
//!
//! Source category list: <https://huggingface.co/tasks>

/// Human-readable Hugging Face model category name.
pub const NAME: &str = "Computer Vision";

/// Stable kebab-case identifier used by this crate.
pub const SLUG: &str = "computer-vision";

/// Hugging Face Hub path where this category is listed.
pub const HUB_PATH: &str = "/tasks";

/// Task labels listed under this model category.
pub const TASKS: &[&str] = &[
    "Depth Estimation",
    "Image Classification",
    "Image Feature Extraction",
    "Image Segmentation",
    "Image-to-Image",
    "Image-to-Text",
    "Image-to-Video",
    "Keypoint Detection",
    "Mask Generation",
    "Object Detection",
    "Video Classification",
    "Text-to-Image",
    "Text-to-Video",
    "Unconditional Image Generation",
    "Video-to-Video",
    "Zero-Shot Image Classification",
    "Zero-Shot Object Detection",
    "Text-to-3D",
    "Image-to-3D",
];

/// Kebab-case task identifiers corresponding to [`TASKS`].
pub const TASK_SLUGS: &[&str] = &[
    "depth-estimation",
    "image-classification",
    "image-feature-extraction",
    "image-segmentation",
    "image-to-image",
    "image-to-text",
    "image-to-video",
    "keypoint-detection",
    "mask-generation",
    "object-detection",
    "video-classification",
    "text-to-image",
    "text-to-video",
    "unconditional-image-generation",
    "video-to-video",
    "zero-shot-image-classification",
    "zero-shot-object-detection",
    "text-to-3d",
    "image-to-3d",
];

/// Returns `true` when `task_slug` is listed in this category.
#[must_use]
pub fn contains_task_slug(task_slug: &str) -> bool {
    TASK_SLUGS.contains(&task_slug)
}

#[cfg(test)]
mod tests {
    use super::{contains_task_slug, HUB_PATH, NAME, SLUG, TASKS, TASK_SLUGS};

    #[test]
    fn model_category_metadata_is_consistent() {
        assert!(!NAME.is_empty());
        assert!(!SLUG.is_empty());
        assert_eq!(HUB_PATH, "/tasks");
        assert_eq!(TASKS.len(), TASK_SLUGS.len());
        assert!(!TASKS.is_empty());
        assert!(contains_task_slug(TASK_SLUGS[0]));
    }
}
