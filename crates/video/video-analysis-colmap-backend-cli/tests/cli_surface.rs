#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        video_analysis_colmap_backend_cli::LIBRARY_CRATE,
        "video-analysis-colmap-backend"
    );
    assert_eq!(video_analysis_colmap_backend_cli::SURFACE_KIND, "cli");
}
