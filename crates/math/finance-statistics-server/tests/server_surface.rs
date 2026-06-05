#[test]
fn package_endpoint_reports_wrapped_library() {
    let response = finance_statistics_server::response_for("GET", "/api/package", "");
    assert_eq!(response.status_code, 200);
    assert!(response.body.contains("finance-statistics"));
}

#[test]
fn run_endpoint_calls_library_surface() {
    let response = finance_statistics_server::response_for(
        "POST",
        "/api/run",
        r#"{"operation":"describe","input":{"includeOperations":true}}"#,
    );
    assert_eq!(response.status_code, 200);
    assert!(response.body.contains(r#""operation""#));
}

#[test]
fn run_endpoint_calls_new_operation() {
    let response = finance_statistics_server::response_for(
        "POST",
        "/api/run",
        r#"{"operation":"finance.riskContribution","input":{"assetReturns":[[0.02,-0.01,0.03],[0.01,0.0,0.02]],"weights":[0.6,0.4],"periodsPerYear":252.0}}"#,
    );
    assert_eq!(response.status_code, 200);
    assert!(response.body.contains(r#""volatility""#));
}
