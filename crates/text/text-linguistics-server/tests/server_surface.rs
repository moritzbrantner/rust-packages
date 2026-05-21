#[test]
fn package_endpoint_reports_wrapped_library() {
    let response = text_linguistics_server::response_for("GET", "/api/package", "");
    assert_eq!(response.status_code, 200);
    assert!(response.body.contains("text-linguistics"));
}
