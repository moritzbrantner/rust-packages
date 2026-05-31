#[test]
fn cli_surface_delegates_to_library() {
    assert_eq!(geo_io_osm_cli::LIBRARY_CRATE, "geo-io-osm");
    let surface = geo_io_osm_cli::package_surface();
    assert_eq!(surface.library, "moritzbrantner-geo-io-osm");
    assert!(surface
        .operations
        .iter()
        .any(|operation| operation.id.as_str() == "osm.filterPbfBase64"));
}

#[test]
fn cli_run_validates_spec() {
    let response = geo_io_osm_cli::run_operation(
        "osm.validateSpec",
        serde_json::json!({"spec": {"filter": {"types": ["node"]}}}),
    )
    .unwrap();
    assert_eq!(response.value["result"]["valid"], true);
}
