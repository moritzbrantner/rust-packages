#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        vector_analysis_core_cli::LIBRARY_CRATE,
        "vector-analysis-core"
    );
    assert_eq!(vector_analysis_core_cli::SURFACE_KIND, "cli");
}
