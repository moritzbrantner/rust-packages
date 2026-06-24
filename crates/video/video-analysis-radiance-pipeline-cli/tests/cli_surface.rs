#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        video_analysis_radiance_pipeline_cli::LIBRARY_CRATE,
        "video-analysis-radiance-pipeline"
    );
    let surface = video_analysis_radiance_pipeline_cli::package_surface();
    assert_eq!(surface.library, "moenarch-video-analysis-radiance-pipeline");
    assert!(!surface.operations.is_empty());
}
