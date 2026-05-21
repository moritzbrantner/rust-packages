#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        image_analysis_detection_cli::LIBRARY_CRATE,
        "image-analysis-detection"
    );
    assert_eq!(image_analysis_detection_cli::SURFACE_KIND, "cli");
}
