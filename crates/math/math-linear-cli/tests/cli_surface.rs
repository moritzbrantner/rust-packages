#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(math_linear_cli::LIBRARY_CRATE, "math-linear");
    let surface = math_linear_cli::package_surface();
    assert_eq!(surface.library, "math-linear");
    assert!(!surface.operations.is_empty());
}
