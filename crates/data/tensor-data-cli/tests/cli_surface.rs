#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(tensor_data_cli::LIBRARY_CRATE, "tensor-data");
    assert_eq!(tensor_data_cli::SURFACE_KIND, "cli");
}
