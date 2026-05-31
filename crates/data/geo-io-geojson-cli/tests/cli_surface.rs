#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(geo_io_geojson_cli::LIBRARY_CRATE, "geo-io-geojson");
    let surface = geo_io_geojson_cli::package_surface();
    assert_eq!(surface.library, "moritzbrantner-geo-io-geojson");
    assert!(surface
        .operations
        .iter()
        .any(|operation| operation.id.as_str() == "geoJson.toGeoJson"));

    let response = geo_io_geojson_cli::run_operation(
        "describe",
        serde_json::json!({"includeOperations": true}),
    )
    .expect("describe operation");
    assert_eq!(response.operation.as_str(), "describe");
    assert_eq!(response.value["operation"], "describe");
    assert_eq!(
        response.value["result"]["library"],
        "moritzbrantner-geo-io-geojson"
    );
}
