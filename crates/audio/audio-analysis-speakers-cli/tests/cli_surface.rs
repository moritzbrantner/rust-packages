#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        audio_analysis_speakers_cli::LIBRARY_CRATE,
        "audio-analysis-speakers"
    );
    let surface = audio_analysis_speakers_cli::package_surface();
    assert_eq!(surface.library, "audio-analysis-speakers");
    assert!(!surface.operations.is_empty());
}
