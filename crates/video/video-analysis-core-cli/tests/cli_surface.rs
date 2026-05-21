#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        video_analysis_core_cli::LIBRARY_CRATE,
        "video-analysis-core"
    );
    assert_eq!(video_analysis_core_cli::SURFACE_KIND, "cli");
}
