#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        video_analysis_posture_cli::LIBRARY_CRATE,
        "video-analysis-posture"
    );
    assert_eq!(video_analysis_posture_cli::SURFACE_KIND, "cli");
}
