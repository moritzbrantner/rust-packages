#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(text_generation_cli::LIBRARY_CRATE, "text-generation");
    let surface = text_generation_cli::package_surface();
    assert_eq!(surface.library, "text-generation");
    assert!(!surface.operations.is_empty());
}
