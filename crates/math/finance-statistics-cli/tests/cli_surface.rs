#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(finance_statistics_cli::LIBRARY_CRATE, "finance-statistics");
    let surface = finance_statistics_cli::package_surface();
    assert_eq!(surface.library, "moritzbrantner-finance-statistics");
    assert!(!surface.operations.is_empty());
}

#[test]
fn cli_adapter_runs_new_operation() {
    let response = finance_statistics_cli::run_operation(
        "finance.riskContribution",
        serde_json::json!({
            "assetReturns": [[0.02, -0.01, 0.03], [0.01, 0.0, 0.02]],
            "weights": [0.6, 0.4],
            "periodsPerYear": 252.0
        }),
    )
    .expect("run operation");
    assert!(response.value["volatility"].as_f64().unwrap() > 0.0);
}
