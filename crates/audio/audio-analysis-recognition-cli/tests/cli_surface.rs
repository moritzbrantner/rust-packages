#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        audio_analysis_recognition_cli::LIBRARY_CRATE,
        "audio-analysis-recognition"
    );
    assert_eq!(audio_analysis_recognition_cli::SURFACE_KIND, "cli");
}
