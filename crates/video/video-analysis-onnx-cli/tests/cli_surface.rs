#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        video_analysis_onnx_cli::LIBRARY_CRATE,
        "video-analysis-onnx"
    );
    assert_eq!(video_analysis_onnx_cli::SURFACE_KIND, "cli");
}
