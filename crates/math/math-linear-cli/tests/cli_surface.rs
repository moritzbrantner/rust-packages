#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(math_linear_cli::LIBRARY_CRATE, "math-linear");
    assert_eq!(math_linear_cli::SURFACE_KIND, "cli");
}
