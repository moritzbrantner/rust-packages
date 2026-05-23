#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(text_model_runtime_cli::LIBRARY_CRATE, "text-model-runtime");
    let surface = text_model_runtime_cli::package_surface();
    assert_eq!(surface.library, "text-model-runtime");
    assert!(!surface.operations.is_empty());
}
