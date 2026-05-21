#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(comfyui_models_cli::LIBRARY_CRATE, "comfyui-models");
    assert_eq!(comfyui_models_cli::SURFACE_KIND, "cli");
}
