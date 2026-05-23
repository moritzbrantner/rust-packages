#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        audio_analysis_recognition_cli::LIBRARY_CRATE,
        "audio-analysis-recognition"
    );
    let surface = audio_analysis_recognition_cli::package_surface();
    assert_eq!(surface.library, "audio-analysis-recognition");
    assert!(!surface.operations.is_empty());
}
