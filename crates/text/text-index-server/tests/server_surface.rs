#[test]
fn package_endpoint_reports_wrapped_library() {
    let response = text_index_server::response_for("GET", "/api/package", "");
    assert_eq!(response.status_code, 200);
    assert!(response.body.contains("text-index"));
}

#[test]
fn run_endpoint_calls_library_surface() {
    let response = text_index_server::response_for(
        "POST",
        "/api/run",
        r#"{"operation":"index.search","input":{"documents":[{"id":"doc-1","body":"Rust durable text indexing"},{"id":"doc-2","body":"Video scene reports"}],"query":{"text":"text indexing","topK":2}}}"#,
    );
    assert_eq!(response.status_code, 200);
    assert!(response.body.contains("index.search"));
}
