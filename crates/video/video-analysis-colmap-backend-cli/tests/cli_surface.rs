#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        video_analysis_colmap_backend_cli::LIBRARY_CRATE,
        "video-analysis-colmap-backend"
    );
    let surface = video_analysis_colmap_backend_cli::package_surface();
    assert_eq!(surface.library, "video-analysis-colmap-backend");
    assert!(!surface.operations.is_empty());
}
