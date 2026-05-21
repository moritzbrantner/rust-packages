#[test]
fn package_endpoint_reports_wrapped_library() {
    let response = math_geometry_2d_server::response_for("GET", "/api/package", "");
    assert_eq!(response.status_code, 200);
    assert!(response.body.contains("math-geometry-2d"));
}
