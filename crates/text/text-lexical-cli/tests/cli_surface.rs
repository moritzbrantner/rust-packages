#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(text_lexical_cli::LIBRARY_CRATE, "text-lexical");
    let surface = text_lexical_cli::package_surface();
    assert_eq!(surface.library, "text-lexical");
    assert!(!surface.operations.is_empty());
}
