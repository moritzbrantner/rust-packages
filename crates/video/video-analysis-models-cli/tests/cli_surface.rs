#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        video_analysis_models_cli::LIBRARY_CRATE,
        "video-analysis-models"
    );
    assert_eq!(video_analysis_models_cli::SURFACE_KIND, "cli");
}
