#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(comfyui_data_cli::LIBRARY_CRATE, "comfyui-data");
    assert_eq!(comfyui_data_cli::SURFACE_KIND, "cli");
}
