#[test]
fn package_endpoint_reports_wrapped_library() {
    let response = image_analysis_core_server::response_for("GET", "/api/package", "");
    assert_eq!(response.status_code, 200);
    assert!(response.body.contains("image-analysis-core"));
}
