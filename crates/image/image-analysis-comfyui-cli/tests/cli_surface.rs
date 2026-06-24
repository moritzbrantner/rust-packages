#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        image_analysis_comfyui_cli::LIBRARY_CRATE,
        "image-analysis-comfyui"
    );
    let surface = image_analysis_comfyui_cli::package_surface();
    assert_eq!(surface.library, "moenarch-image-analysis-comfyui");
    assert!(!surface.operations.is_empty());
}
