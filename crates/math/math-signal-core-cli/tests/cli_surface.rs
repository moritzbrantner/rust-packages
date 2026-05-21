#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(math_signal_core_cli::LIBRARY_CRATE, "math-signal-core");
    assert_eq!(math_signal_core_cli::SURFACE_KIND, "cli");
}
