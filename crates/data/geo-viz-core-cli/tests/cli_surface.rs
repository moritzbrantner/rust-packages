#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(geo_viz_core_cli::LIBRARY_CRATE, "geo-viz-core");
    let surface = geo_viz_core_cli::package_surface();
    assert_eq!(surface.library, "geo-viz-core");
    assert!(!surface.operations.is_empty());
}
