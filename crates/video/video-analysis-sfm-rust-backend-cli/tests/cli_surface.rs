#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        video_analysis_sfm_rust_backend_cli::LIBRARY_CRATE,
        "video-analysis-sfm-rust-backend"
    );
    assert_eq!(video_analysis_sfm_rust_backend_cli::SURFACE_KIND, "cli");
}
