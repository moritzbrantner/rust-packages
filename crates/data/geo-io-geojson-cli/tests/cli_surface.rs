#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(geo_io_geojson_cli::LIBRARY_CRATE, "geo-io-geojson");
    let surface = geo_io_geojson_cli::package_surface();
    assert_eq!(surface.library, "moritzbrantner-geo-io-geojson");
    assert!(!surface.operations.is_empty());
}
