#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(geo_viz_cli::LIBRARY_CRATE, "geo-viz");
    let surface = geo_viz_cli::package_surface();
    assert_eq!(surface.library, "moritzbrantner-geo-viz");
    assert!(surface
        .operations
        .iter()
        .any(|operation| operation.id.as_str() == "geoViz.aggregateViewport"));

    let response =
        geo_viz_cli::run_operation("describe", serde_json::json!({"includeOperations": true}))
            .expect("describe operation");
    assert_eq!(response.operation.as_str(), "describe");
}
