#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(geo_core_cli::LIBRARY_CRATE, "geo-core");
    let surface = geo_core_cli::package_surface();
    assert_eq!(surface.library, "moritzbrantner-geo-core");
    assert!(!surface.operations.is_empty());
}
