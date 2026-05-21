#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        video_analysis_posture_io_cli::LIBRARY_CRATE,
        "video-analysis-posture-io"
    );
    assert_eq!(video_analysis_posture_io_cli::SURFACE_KIND, "cli");
}
