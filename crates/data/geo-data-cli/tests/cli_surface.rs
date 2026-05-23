#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(geo_data_cli::LIBRARY_CRATE, "geo-data");
    let surface = geo_data_cli::package_surface();
    assert_eq!(surface.library, "geo-data");
    assert!(!surface.operations.is_empty());
}
