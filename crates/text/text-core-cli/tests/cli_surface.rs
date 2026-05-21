#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(text_core_cli::LIBRARY_CRATE, "text-core");
    assert_eq!(text_core_cli::SURFACE_KIND, "cli");
}
