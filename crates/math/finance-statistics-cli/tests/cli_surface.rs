#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(finance_statistics_cli::LIBRARY_CRATE, "finance-statistics");
    let surface = finance_statistics_cli::package_surface();
    assert_eq!(surface.library, "moritzbrantner-finance-statistics");
    assert!(!surface.operations.is_empty());
}
