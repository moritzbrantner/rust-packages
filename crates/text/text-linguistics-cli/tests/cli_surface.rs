#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(text_linguistics_cli::LIBRARY_CRATE, "text-linguistics");
    assert_eq!(text_linguistics_cli::SURFACE_KIND, "cli");
}
