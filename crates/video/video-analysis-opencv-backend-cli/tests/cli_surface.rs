#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        video_analysis_opencv_backend_cli::LIBRARY_CRATE,
        "video-analysis-opencv-backend"
    );
    assert_eq!(video_analysis_opencv_backend_cli::SURFACE_KIND, "cli");
}
