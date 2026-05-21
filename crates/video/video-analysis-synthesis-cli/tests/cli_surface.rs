#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        video_analysis_synthesis_cli::LIBRARY_CRATE,
        "video-analysis-synthesis"
    );
    assert_eq!(video_analysis_synthesis_cli::SURFACE_KIND, "cli");
}
