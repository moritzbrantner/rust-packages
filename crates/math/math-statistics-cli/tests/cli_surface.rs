#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(math_statistics_cli::LIBRARY_CRATE, "math-statistics");
    assert_eq!(math_statistics_cli::SURFACE_KIND, "cli");
}
