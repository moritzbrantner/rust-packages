#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(math_linear_cli::LIBRARY_CRATE, "math-linear");
    let surface = math_linear_cli::package_surface();
    assert_eq!(surface.library, "moritzbrantner-math-linear");
    assert!(!surface.operations.is_empty());
}

#[test]
fn cli_adapter_runs_new_operation() {
    let response = math_linear_cli::run_operation(
        "linear.cholesky",
        serde_json::json!({"matrix": {"rows": 2, "cols": 2, "values": [4.0, 2.0, 2.0, 3.0]}}),
    )
    .expect("run operation");
    assert_eq!(response.value["method"], "cholesky");
}
