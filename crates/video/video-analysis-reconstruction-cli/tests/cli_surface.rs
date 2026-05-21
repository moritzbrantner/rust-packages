#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        video_analysis_reconstruction_cli::LIBRARY_CRATE,
        "video-analysis-reconstruction"
    );
    assert_eq!(video_analysis_reconstruction_cli::SURFACE_KIND, "cli");
}
