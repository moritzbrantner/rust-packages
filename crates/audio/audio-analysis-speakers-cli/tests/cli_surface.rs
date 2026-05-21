#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        audio_analysis_speakers_cli::LIBRARY_CRATE,
        "audio-analysis-speakers"
    );
    assert_eq!(audio_analysis_speakers_cli::SURFACE_KIND, "cli");
}
