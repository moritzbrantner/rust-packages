#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(finance_data_cli::LIBRARY_CRATE, "finance-data");
    let surface = finance_data_cli::package_surface();
    assert_eq!(surface.library, "moritzbrantner-finance-data");
    assert!(!surface.operations.is_empty());
}

#[test]
fn cli_adapter_runs_primary_operation() {
    let operation = finance_data_cli::package_surface()
        .operations
        .into_iter()
        .find(|operation| operation.id.as_str() == "financeData.bounds")
        .expect("financeData.bounds");
    let response =
        finance_data_cli::run_operation(operation.id.as_str(), operation.example_request)
            .expect("run operation");
    assert_eq!(response.operation.as_str(), "financeData.bounds");
    assert!(response.value["result"]["bounds"].is_object());
}
