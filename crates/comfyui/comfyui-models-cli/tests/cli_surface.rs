#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(comfyui_models_cli::LIBRARY_CRATE, "comfyui-models");
    let surface = comfyui_models_cli::package_surface();
    assert_eq!(surface.library, "moritzbrantner-comfyui-models");
    assert!(!surface.operations.is_empty());
}
