#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(math_sparse_data_cli::LIBRARY_CRATE, "math-sparse-data");
    let surface = math_sparse_data_cli::package_surface();
    assert_eq!(surface.library, "moritzbrantner-math-sparse-data");
    assert!(!surface.operations.is_empty());
}

#[test]
fn cli_adapter_runs_new_operation() {
    let response = math_sparse_data_cli::run_operation(
        "sparse.vectorOps",
        serde_json::json!({"vector": {"dimensions": 3, "indices": [0, 2], "values": [1.0, -2.0]}, "topK": 1}),
    )
    .expect("run operation");
    assert_eq!(response.value["nnz"], 2);
}
