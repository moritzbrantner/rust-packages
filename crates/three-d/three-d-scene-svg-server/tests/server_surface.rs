#[test]
fn package_endpoint_reports_wrapped_library() {
    let response = three_d_scene_svg_server::response_for("GET", "/api/package", "");
    assert_eq!(response.status_code, 200);
    assert!(response.body.contains("three-d-scene-svg"));
}
