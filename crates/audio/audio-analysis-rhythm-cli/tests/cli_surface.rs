#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        audio_analysis_rhythm_cli::LIBRARY_CRATE,
        "audio-analysis-rhythm"
    );
    assert_eq!(audio_analysis_rhythm_cli::SURFACE_KIND, "cli");
}
