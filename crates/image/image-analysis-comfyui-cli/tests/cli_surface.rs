#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        image_analysis_comfyui_cli::LIBRARY_CRATE,
        "image-analysis-comfyui"
    );
    assert_eq!(image_analysis_comfyui_cli::SURFACE_KIND, "cli");
}
