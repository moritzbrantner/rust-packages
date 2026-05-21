#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(numbers_core_cli::LIBRARY_CRATE, "numbers-core");
    assert_eq!(numbers_core_cli::SURFACE_KIND, "cli");
}
