#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        video_analysis_split_cli::LIBRARY_CRATE,
        "video-analysis-split"
    );
    assert_eq!(video_analysis_split_cli::SURFACE_KIND, "cli");
}
