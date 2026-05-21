#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        audio_analysis_processing_cli::LIBRARY_CRATE,
        "audio-analysis-processing"
    );
    assert_eq!(audio_analysis_processing_cli::SURFACE_KIND, "cli");
}
