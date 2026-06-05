#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(maps_kernels_core_cli::LIBRARY_CRATE, "maps-kernels-core");
    let surface = maps_kernels_core_cli::package_surface();
    assert_eq!(surface.library, "moritzbrantner-maps-kernels-core");
    assert!(!surface.operations.is_empty());
}

#[test]
fn cli_adapter_runs_new_operation() {
    let response = maps_kernels_core_cli::run_operation(
        "maps.pathSummary",
        serde_json::json!({"coordinates": [0.0, 0.0, 3.0, 4.0]}),
    )
    .expect("run operation");
    assert_eq!(response.value["pointCount"], 2);
}
