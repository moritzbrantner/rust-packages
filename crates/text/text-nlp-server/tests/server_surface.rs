#[test]
fn package_endpoint_reports_text_nlp_tasks() {
    let response = text_nlp_server::response_for("GET", "/api/package", "");
    assert_eq!(response.status_code, 200);
    assert!(response.body.contains("text-nlp-tasks"));
    assert!(response.body.contains("text-nlp-server"));
}

#[test]
fn sentiment_endpoint_supports_lexical_fallback() {
    let response = text_nlp_server::response_for(
        "POST",
        "/api/sentiment",
        r#"{"text":"excellent reliable work","model":{"fallbackPolicy":"lexical_fallback"}}"#,
    );
    assert_eq!(response.status_code, 200);
    assert!(response.body.contains("\"operation\":\"sentiment\""));
    assert!(response.body.contains("\"runtime\":\"lexical\""));
}
