#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        video_analysis_posture_io_cli::LIBRARY_CRATE,
        "video-analysis-posture-io"
    );
    let surface = video_analysis_posture_io_cli::package_surface();
    assert_eq!(surface.library, "moritzbrantner-video-analysis-posture-io");
    assert!(!surface.operations.is_empty());
}
