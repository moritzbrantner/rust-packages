#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(text_core_cli::LIBRARY_CRATE, "text-core");
    let surface = text_core_cli::package_surface();
    assert_eq!(surface.library, "text-core");
    assert!(!surface.operations.is_empty());
}
