#[test]
fn package_endpoint_reports_wrapped_library() {
    let response = math_statistics_server::response_for("GET", "/api/package", "");
    assert_eq!(response.status_code, 200);
    assert!(response.body.contains("math-statistics"));
}

#[test]
fn run_endpoint_calls_library_surface() {
    let response = math_statistics_server::response_for(
        "POST",
        "/api/run",
        r#"{"operation":"describe","input":{"includeOperations":true}}"#,
    );
    assert_eq!(response.status_code, 200);
    assert!(response.body.contains(r#""operation""#));
}

#[test]
fn run_endpoint_calls_new_operation() {
    let response = math_statistics_server::response_for(
        "POST",
        "/api/run",
        r#"{"operation":"stats.regression.diagnostics","input":{"design":{"rows":4,"cols":2,"values":[1.0,1.0,1.0,2.0,1.0,3.0,1.0,4.0]},"target":[3.0,5.0,7.0,9.0],"tolerance":0.0}}"#,
    );
    assert_eq!(response.status_code, 200);
    assert!(response.body.contains(r#""degreesOfFreedom":2"#));
}
