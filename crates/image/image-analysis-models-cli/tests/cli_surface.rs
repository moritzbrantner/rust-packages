#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        image_analysis_models_cli::LIBRARY_CRATE,
        "image-analysis-models"
    );
    let surface = image_analysis_models_cli::package_surface();
    assert_eq!(surface.library, "image-analysis-models");
    assert!(!surface.operations.is_empty());
}
