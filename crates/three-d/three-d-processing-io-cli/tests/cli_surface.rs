#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        three_d_processing_io_cli::LIBRARY_CRATE,
        "three-d-processing-io"
    );
    assert_eq!(three_d_processing_io_cli::SURFACE_KIND, "cli");
}
