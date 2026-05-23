#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(comfyui_data_cli::LIBRARY_CRATE, "comfyui-data");
    let surface = comfyui_data_cli::package_surface();
    assert_eq!(surface.library, "comfyui-data");
    assert!(!surface.operations.is_empty());
}
