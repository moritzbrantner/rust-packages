#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        audio_analysis_pitch_cli::LIBRARY_CRATE,
        "audio-analysis-pitch"
    );
    let surface = audio_analysis_pitch_cli::package_surface();
    assert_eq!(surface.library, "audio-analysis-pitch");
    assert!(!surface.operations.is_empty());
}
