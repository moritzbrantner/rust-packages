#[test]
fn server_package_endpoint_mentions_osm() {
    let response = geo_io_osm_server::response_for("GET", "/api/package", "");
    assert_eq!(response.status_code, 200);
    assert!(response.body.contains("geo-io-osm"));
}

#[test]
fn server_operations_include_filter() {
    let response = geo_io_osm_server::response_for("GET", "/api/operations", "");
    assert_eq!(response.status_code, 200);
    assert!(response.body.contains("osm.filterPbfBase64"));
}

#[test]
fn server_run_validates_spec() {
    let response = geo_io_osm_server::response_for(
        "POST",
        "/api/run",
        r#"{"operation":"osm.validateSpec","input":{"spec":{"filter":{"types":["node"]}}}}"#,
    );
    assert_eq!(response.status_code, 200);
    assert!(response.body.contains("\"valid\":true"));
    let body: serde_json::Value = serde_json::from_str(&response.body).expect("response JSON");
    assert_eq!(body["operation"], "osm.validateSpec");
}
