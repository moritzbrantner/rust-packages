#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        video_analysis_recognition_cli::LIBRARY_CRATE,
        "video-analysis-recognition"
    );
    assert_eq!(video_analysis_recognition_cli::SURFACE_KIND, "cli");
}
