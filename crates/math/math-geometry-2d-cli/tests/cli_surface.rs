#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(math_geometry_2d_cli::LIBRARY_CRATE, "math-geometry-2d");
    let surface = math_geometry_2d_cli::package_surface();
    assert_eq!(surface.library, "moritzbrantner-math-geometry-2d");
    assert!(!surface.operations.is_empty());
}
