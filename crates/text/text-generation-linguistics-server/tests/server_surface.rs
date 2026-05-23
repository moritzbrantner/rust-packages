#[test]
fn package_endpoint_reports_wrapped_library() {
    let response = text_generation_linguistics_server::response_for("GET", "/api/package", "");
    assert_eq!(response.status_code, 200);
    assert!(response.body.contains("text-generation-linguistics"));
}

#[test]
fn run_endpoint_calls_library_surface() {
    let response = text_generation_linguistics_server::response_for(
        "POST",
        "/api/run",
        r#"{"operation":"describe","input":{"includeOperations":true}}"#,
    );
    assert_eq!(response.status_code, 200);
    assert!(response.body.contains(r#""operation""#));
}
