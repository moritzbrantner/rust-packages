#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        three_d_processing_mesh_cli::LIBRARY_CRATE,
        "three-d-processing-mesh"
    );
    assert_eq!(three_d_processing_mesh_cli::SURFACE_KIND, "cli");
}
