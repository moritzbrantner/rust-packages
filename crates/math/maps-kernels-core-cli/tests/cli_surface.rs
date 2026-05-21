#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(maps_kernels_core_cli::LIBRARY_CRATE, "maps-kernels-core");
    assert_eq!(maps_kernels_core_cli::SURFACE_KIND, "cli");
}
