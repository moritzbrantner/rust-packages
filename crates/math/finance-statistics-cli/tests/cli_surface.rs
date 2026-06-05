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
        "finance.performanceRatios",
        serde_json::json!({"returns": [0.1, -0.2, 0.05, 0.3]}),
    )
    .expect("run operation");
    assert!(response.value["drawdownDuration"].is_object());
}
