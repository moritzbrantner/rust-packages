#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        three_d_processing_mesh_cli::LIBRARY_CRATE,
        "three-d-processing-mesh"
    );
    let surface = three_d_processing_mesh_cli::package_surface();
    assert_eq!(surface.library, "moenarch-three-d-processing-mesh");
    assert!(!surface.operations.is_empty());
}
