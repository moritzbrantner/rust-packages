#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(runtime_contracts_cli::LIBRARY_CRATE, "runtime-contracts");
    let surface = runtime_contracts_cli::package_surface();
    assert_eq!(surface.library, "runtime-contracts");
    assert!(!surface.operations.is_empty());
}
