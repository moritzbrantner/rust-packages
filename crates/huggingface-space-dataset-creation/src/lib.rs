//! Hugging Face Space category metadata for Dataset Creation.
//!
//! Source category list: <https://huggingface.co/spaces>

/// Human-readable Hugging Face Space category name.
pub const NAME: &str = "Dataset Creation";

/// Stable kebab-case identifier used by this crate.
pub const SLUG: &str = "dataset-creation";

/// Hugging Face Hub path where this category is listed.
pub const HUB_PATH: &str = "/spaces";

/// Search label used by the Hugging Face Spaces directory.
pub const SEARCH_QUERY: &str = "Dataset Creation";

/// Returns `true` when `value` matches this category name or slug.
#[must_use]
pub fn matches_name_or_slug(value: &str) -> bool {
    value.eq_ignore_ascii_case(NAME) || value == SLUG
}

#[cfg(test)]
mod tests {
    use super::{matches_name_or_slug, HUB_PATH, NAME, SEARCH_QUERY, SLUG};

    #[test]
    fn space_category_metadata_is_consistent() {
        assert!(!NAME.is_empty());
        assert!(!SLUG.is_empty());
        assert_eq!(HUB_PATH, "/spaces");
        assert_eq!(SEARCH_QUERY, NAME);
        assert!(matches_name_or_slug(NAME));
        assert!(matches_name_or_slug(SLUG));
    }
}
