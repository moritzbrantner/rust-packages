#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(math_statistics_cli::LIBRARY_CRATE, "math-statistics");
    let surface = math_statistics_cli::package_surface();
    assert_eq!(surface.library, "moritzbrantner-math-statistics");
    assert!(!surface.operations.is_empty());
}

#[test]
fn cli_adapter_runs_new_operation() {
    let response = math_statistics_cli::run_operation(
        "stats.regression.linear",
        serde_json::json!({"x": [1.0, 2.0, 3.0], "y": [3.0, 5.0, 7.0]}),
    )
    .expect("run operation");
    assert_eq!(response.value["slope"], 2.0);
}
