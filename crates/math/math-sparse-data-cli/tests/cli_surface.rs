#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(math_sparse_data_cli::LIBRARY_CRATE, "math-sparse-data");
    assert_eq!(math_sparse_data_cli::SURFACE_KIND, "cli");
}
