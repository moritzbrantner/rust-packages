#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(finance_statistics_cli::LIBRARY_CRATE, "finance-statistics");
    assert_eq!(finance_statistics_cli::SURFACE_KIND, "cli");
}
