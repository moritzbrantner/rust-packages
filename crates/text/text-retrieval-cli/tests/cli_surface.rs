#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(text_retrieval_cli::LIBRARY_CRATE, "text-retrieval");
    assert_eq!(text_retrieval_cli::SURFACE_KIND, "cli");
}
