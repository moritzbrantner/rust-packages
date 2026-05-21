#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        vector_analysis_index_cli::LIBRARY_CRATE,
        "vector-analysis-index"
    );
    assert_eq!(vector_analysis_index_cli::SURFACE_KIND, "cli");
}
