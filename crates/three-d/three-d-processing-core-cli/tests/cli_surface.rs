#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        three_d_processing_core_cli::LIBRARY_CRATE,
        "three-d-processing-core"
    );
    let surface = three_d_processing_core_cli::package_surface();
    assert_eq!(surface.library, "three-d-processing-core");
    assert!(!surface.operations.is_empty());
}
