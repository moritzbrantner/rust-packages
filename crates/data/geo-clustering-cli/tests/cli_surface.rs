#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(geo_clustering_cli::LIBRARY_CRATE, "geo-clustering");
    let surface = geo_clustering_cli::package_surface();
    assert_eq!(surface.library, "moritzbrantner-geo-clustering");
    assert!(!surface.operations.is_empty());
}
