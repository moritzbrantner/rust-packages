#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(image_analysis_io_cli::LIBRARY_CRATE, "image-analysis-io");
    assert_eq!(image_analysis_io_cli::SURFACE_KIND, "cli");
}
