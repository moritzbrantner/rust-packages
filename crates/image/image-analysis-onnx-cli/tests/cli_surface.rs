#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        image_analysis_onnx_cli::LIBRARY_CRATE,
        "image-analysis-onnx"
    );
    let surface = image_analysis_onnx_cli::package_surface();
    assert_eq!(surface.library, "image-analysis-onnx");
    assert!(!surface.operations.is_empty());
}
