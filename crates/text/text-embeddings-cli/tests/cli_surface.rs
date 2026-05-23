#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(text_embeddings_cli::LIBRARY_CRATE, "text-embeddings");
    let surface = text_embeddings_cli::package_surface();
    assert_eq!(surface.library, "text-embeddings");
    assert!(!surface.operations.is_empty());
}
