#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(math_signal_core_cli::LIBRARY_CRATE, "math-signal-core");
    let surface = math_signal_core_cli::package_surface();
    assert_eq!(surface.library, "moritzbrantner-math-signal-core");
    assert!(!surface.operations.is_empty());
}
