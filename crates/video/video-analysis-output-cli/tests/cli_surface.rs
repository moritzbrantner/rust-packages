#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        video_analysis_output_cli::LIBRARY_CRATE,
        "video-analysis-output"
    );
    assert_eq!(video_analysis_output_cli::SURFACE_KIND, "cli");
}
