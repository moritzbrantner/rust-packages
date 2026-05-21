#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        video_analysis_detectors_cli::LIBRARY_CRATE,
        "video-analysis-detectors"
    );
    assert_eq!(video_analysis_detectors_cli::SURFACE_KIND, "cli");
}
