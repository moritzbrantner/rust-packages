#[test]
fn package_endpoint_reports_wrapped_library() {
    let response = text_linguistics_server::response_for("GET", "/api/package", "");
    assert_eq!(response.status_code, 200);
    assert!(response.body.contains("text-linguistics"));
}

#[test]
fn run_endpoint_can_use_explicit_heuristic_mode() {
    let response = text_linguistics_server::response_for(
        "POST",
        "/api/run",
        r#"{"operation":"analyze","modelMode":"heuristic","text":"Alice presented the roadmap in Berlin."}"#,
    );
    assert_eq!(response.status_code, 200);
    assert!(response.body.contains("\"operation\":\"analyze\""));
    assert!(response
        .body
        .contains("\"entityRecognition\":\"heuristic\""));
}
