#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        image_analysis_models_cli::LIBRARY_CRATE,
        "image-analysis-models"
    );
    assert_eq!(image_analysis_models_cli::SURFACE_KIND, "cli");
}
