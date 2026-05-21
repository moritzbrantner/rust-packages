#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        video_analysis_storage_cli::LIBRARY_CRATE,
        "video-analysis-storage"
    );
    assert_eq!(video_analysis_storage_cli::SURFACE_KIND, "cli");
}
