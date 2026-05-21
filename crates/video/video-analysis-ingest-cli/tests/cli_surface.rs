#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        video_analysis_ingest_cli::LIBRARY_CRATE,
        "video-analysis-ingest"
    );
    assert_eq!(video_analysis_ingest_cli::SURFACE_KIND, "cli");
}
