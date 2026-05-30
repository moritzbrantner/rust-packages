#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(geo_viz_cli::LIBRARY_CRATE, "geo-viz");
    let surface = geo_viz_cli::package_surface();
    assert_eq!(surface.library, "moritzbrantner-geo-viz");
    assert!(!surface.operations.is_empty());
}
