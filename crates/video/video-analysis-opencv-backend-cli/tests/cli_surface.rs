#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        video_analysis_opencv_backend_cli::LIBRARY_CRATE,
        "video-analysis-opencv-backend"
    );
    let surface = video_analysis_opencv_backend_cli::package_surface();
    assert_eq!(surface.library, "video-analysis-opencv-backend");
    assert!(!surface.operations.is_empty());
}
