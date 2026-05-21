#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        video_analysis_editing_cli::LIBRARY_CRATE,
        "video-analysis-editing"
    );
    assert_eq!(video_analysis_editing_cli::SURFACE_KIND, "cli");
}
