#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        image_analysis_segmentation_cli::LIBRARY_CRATE,
        "image-analysis-segmentation"
    );
    assert_eq!(image_analysis_segmentation_cli::SURFACE_KIND, "cli");
}
