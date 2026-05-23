#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(comfyui_latents_cli::LIBRARY_CRATE, "comfyui-latents");
    let surface = comfyui_latents_cli::package_surface();
    assert_eq!(surface.library, "comfyui-latents");
    assert!(!surface.operations.is_empty());
}
