#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        three_d_processing_core_cli::LIBRARY_CRATE,
        "three-d-processing-core"
    );
    assert_eq!(three_d_processing_core_cli::SURFACE_KIND, "cli");
}
