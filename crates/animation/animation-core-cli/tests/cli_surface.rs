#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(animation_core_cli::LIBRARY_CRATE, "animation-core");
    assert_eq!(animation_core_cli::SURFACE_KIND, "cli");
}
