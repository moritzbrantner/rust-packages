#![doc = include_str!("../README.md")]

pub mod surface;
pub use text_nlp_models::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reexports_text_nlp_model_catalog() {
        assert!(!model_catalog(None).is_empty());
        assert_eq!(parse_task("classify"), Some(NlpTask::TextClassification));
    }
}
