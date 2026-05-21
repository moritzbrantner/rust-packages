#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        video_analysis_segmentation_cli::LIBRARY_CRATE,
        "video-analysis-segmentation"
    );
    assert_eq!(video_analysis_segmentation_cli::SURFACE_KIND, "cli");
}
