#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(dense_data_cli::LIBRARY_CRATE, "dense-data");
    assert_eq!(dense_data_cli::SURFACE_KIND, "cli");
}
