#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(maps_kernels_core_cli::LIBRARY_CRATE, "maps-kernels-core");
    let surface = maps_kernels_core_cli::package_surface();
    assert_eq!(surface.library, "moritzbrantner-maps-kernels-core");
    assert!(!surface.operations.is_empty());
}
