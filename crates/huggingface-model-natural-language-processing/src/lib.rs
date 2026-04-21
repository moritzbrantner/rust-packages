//! Hugging Face model category metadata for Natural Language Processing.
//!
//! Source category list: <https://huggingface.co/tasks>

/// Human-readable Hugging Face model category name.
pub const NAME: &str = "Natural Language Processing";

/// Stable kebab-case identifier used by this crate.
pub const SLUG: &str = "natural-language-processing";

/// Hugging Face Hub path where this category is listed.
pub const HUB_PATH: &str = "/tasks";

/// Task labels listed under this model category.
pub const TASKS: &[&str] = &[
    "Feature Extraction",
    "Fill-Mask",
    "Question Answering",
    "Sentence Similarity",
    "Summarization",
    "Table Question Answering",
    "Text Classification",
    "Text Generation",
    "Text Ranking",
    "Token Classification",
    "Translation",
    "Zero-Shot Classification",
];

/// Kebab-case task identifiers corresponding to [`TASKS`].
pub const TASK_SLUGS: &[&str] = &[
    "feature-extraction",
    "fill-mask",
    "question-answering",
    "sentence-similarity",
    "summarization",
    "table-question-answering",
    "text-classification",
    "text-generation",
    "text-ranking",
    "token-classification",
    "translation",
    "zero-shot-classification",
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
