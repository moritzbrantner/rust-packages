#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(math_geometry_2d_cli::LIBRARY_CRATE, "math-geometry-2d");
    assert_eq!(math_geometry_2d_cli::SURFACE_KIND, "cli");
}
