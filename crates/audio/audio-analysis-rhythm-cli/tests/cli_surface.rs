#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        audio_analysis_rhythm_cli::LIBRARY_CRATE,
        "audio-analysis-rhythm"
    );
    let surface = audio_analysis_rhythm_cli::package_surface();
    assert_eq!(surface.library, "audio-analysis-rhythm");
    assert!(!surface.operations.is_empty());
}
