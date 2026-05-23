#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        video_analysis_models_cli::LIBRARY_CRATE,
        "video-analysis-models"
    );
    let surface = video_analysis_models_cli::package_surface();
    assert_eq!(surface.library, "video-analysis-models");
    assert!(!surface.operations.is_empty());
}
