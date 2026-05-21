#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        image_analysis_synthesis_cli::LIBRARY_CRATE,
        "image-analysis-synthesis"
    );
    assert_eq!(image_analysis_synthesis_cli::SURFACE_KIND, "cli");
}
