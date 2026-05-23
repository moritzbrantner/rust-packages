#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        video_analysis_onnx_cli::LIBRARY_CRATE,
        "video-analysis-onnx"
    );
    let surface = video_analysis_onnx_cli::package_surface();
    assert_eq!(surface.library, "video-analysis-onnx");
    assert!(!surface.operations.is_empty());
}
