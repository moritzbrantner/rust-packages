#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        video_analysis_radiance_pipeline_cli::LIBRARY_CRATE,
        "video-analysis-radiance-pipeline"
    );
    assert_eq!(video_analysis_radiance_pipeline_cli::SURFACE_KIND, "cli");
}
