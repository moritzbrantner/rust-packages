#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(text_linguistics_cli::LIBRARY_CRATE, "text-linguistics");
    let surface = text_linguistics_cli::package_surface();
    assert_eq!(surface.library, "text-linguistics");
    assert!(!surface.operations.is_empty());
}
