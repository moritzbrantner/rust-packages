//! Hugging Face model category metadata for Multimodal.
//!
//! Source category list: <https://huggingface.co/tasks>

/// Human-readable Hugging Face model category name.
pub const NAME: &str = "Multimodal";

/// Stable kebab-case identifier used by this crate.
pub const SLUG: &str = "multimodal";

/// Hugging Face Hub path where this category is listed.
pub const HUB_PATH: &str = "/tasks";

/// Task labels listed under this model category.
pub const TASKS: &[&str] = &[
    "Any-to-Any",
    "Audio-Text-to-Text",
    "Document Question Answering",
    "Visual Document Retrieval",
    "Image-Text-to-Text",
    "Image-Text-to-Image",
    "Image-Text-to-Video",
    "Video-Text-to-Text",
    "Visual Question Answering",
];

/// Kebab-case task identifiers corresponding to [`TASKS`].
pub const TASK_SLUGS: &[&str] = &[
    "any-to-any",
    "audio-text-to-text",
    "document-question-answering",
    "visual-document-retrieval",
    "image-text-to-text",
    "image-text-to-image",
    "image-text-to-video",
    "video-text-to-text",
    "visual-question-answering",
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
