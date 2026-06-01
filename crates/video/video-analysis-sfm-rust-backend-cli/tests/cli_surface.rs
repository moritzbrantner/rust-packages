#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        video_analysis_sfm_rust_backend_cli::LIBRARY_CRATE,
        "video-analysis-sfm-rust-backend"
    );
    let surface = video_analysis_sfm_rust_backend_cli::package_surface();
    assert_eq!(
        surface.library,
        "moritzbrantner-video-analysis-sfm-rust-backend"
    );
    assert!(!surface.operations.is_empty());
}
