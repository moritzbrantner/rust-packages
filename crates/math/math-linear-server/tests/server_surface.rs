#[test]
fn package_endpoint_reports_wrapped_library() {
    let response = math_linear_server::response_for("GET", "/api/package", "");
    assert_eq!(response.status_code, 200);
    assert!(response.body.contains("math-linear"));
}

#[test]
fn run_endpoint_calls_library_surface() {
    let response = math_linear_server::response_for(
        "POST",
        "/api/run",
        r#"{"operation":"describe","input":{"includeOperations":true}}"#,
    );
    assert_eq!(response.status_code, 200);
    assert!(response.body.contains(r#""operation""#));
}

#[test]
fn run_endpoint_calls_new_operation() {
    let response = math_linear_server::response_for(
        "POST",
        "/api/run",
        r#"{"operation":"linear.cholesky","input":{"matrix":{"rows":2,"cols":2,"values":[4.0,2.0,2.0,3.0]}}}"#,
    );
    assert_eq!(response.status_code, 200);
    assert!(response.body.contains(r#""method":"cholesky""#));
}
