#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(math_sparse_data_cli::LIBRARY_CRATE, "math-sparse-data");
    let surface = math_sparse_data_cli::package_surface();
    assert_eq!(surface.library, "moritzbrantner-math-sparse-data");
    assert!(!surface.operations.is_empty());
}
