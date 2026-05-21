#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        audio_analysis_pitch_cli::LIBRARY_CRATE,
        "audio-analysis-pitch"
    );
    assert_eq!(audio_analysis_pitch_cli::SURFACE_KIND, "cli");
}
