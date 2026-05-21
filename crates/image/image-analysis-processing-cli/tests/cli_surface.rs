#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        image_analysis_processing_cli::LIBRARY_CRATE,
        "image-analysis-processing"
    );
    assert_eq!(image_analysis_processing_cli::SURFACE_KIND, "cli");
}
