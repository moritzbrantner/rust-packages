#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(text_retrieval_cli::LIBRARY_CRATE, "text-retrieval");
    let surface = text_retrieval_cli::package_surface();
    assert_eq!(surface.library, "text-retrieval");
    assert!(!surface.operations.is_empty());
}
