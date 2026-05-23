#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(math_statistics_cli::LIBRARY_CRATE, "math-statistics");
    let surface = math_statistics_cli::package_surface();
    assert_eq!(surface.library, "math-statistics");
    assert!(!surface.operations.is_empty());
}
