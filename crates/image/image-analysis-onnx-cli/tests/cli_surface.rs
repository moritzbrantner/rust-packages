#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        image_analysis_onnx_cli::LIBRARY_CRATE,
        "image-analysis-onnx"
    );
    assert_eq!(image_analysis_onnx_cli::SURFACE_KIND, "cli");
}
