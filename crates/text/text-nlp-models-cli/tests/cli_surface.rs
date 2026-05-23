#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(text_nlp_models_cli::LIBRARY_CRATE, "text-nlp-models");
    let surface = text_nlp_models_cli::package_surface();
    assert_eq!(surface.library, "text-nlp-models");
    assert!(!surface.operations.is_empty());
}
