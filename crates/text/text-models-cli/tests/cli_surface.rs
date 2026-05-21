#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(text_models_cli::LIBRARY_CRATE, "text-models");
    assert_eq!(text_models_cli::SURFACE_KIND, "cli");
}
