#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(comfyui_latents_cli::LIBRARY_CRATE, "comfyui-latents");
    assert_eq!(comfyui_latents_cli::SURFACE_KIND, "cli");
}
