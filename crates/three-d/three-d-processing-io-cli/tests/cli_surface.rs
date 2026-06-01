#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        three_d_processing_io_cli::LIBRARY_CRATE,
        "three-d-processing-io"
    );
    let surface = three_d_processing_io_cli::package_surface();
    assert_eq!(surface.library, "moritzbrantner-three-d-processing-io");
    assert!(!surface.operations.is_empty());
}
