#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(tensor_data_cli::LIBRARY_CRATE, "tensor-data");
    let surface = tensor_data_cli::package_surface();
    assert_eq!(surface.library, "moritzbrantner-tensor-data");
    assert!(!surface.operations.is_empty());
}
