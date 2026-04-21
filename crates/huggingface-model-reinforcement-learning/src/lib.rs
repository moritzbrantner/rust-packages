//! Hugging Face model category metadata for Reinforcement Learning.
//!
//! Source category list: <https://huggingface.co/tasks>

/// Human-readable Hugging Face model category name.
pub const NAME: &str = "Reinforcement Learning";

/// Stable kebab-case identifier used by this crate.
pub const SLUG: &str = "reinforcement-learning";

/// Hugging Face Hub path where this category is listed.
pub const HUB_PATH: &str = "/tasks";

/// Task labels listed under this model category.
pub const TASKS: &[&str] = &["Reinforcement Learning"];

/// Kebab-case task identifiers corresponding to [`TASKS`].
pub const TASK_SLUGS: &[&str] = &["reinforcement-learning"];

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
