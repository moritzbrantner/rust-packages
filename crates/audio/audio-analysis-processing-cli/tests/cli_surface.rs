#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        audio_analysis_processing_cli::LIBRARY_CRATE,
        "audio-analysis-processing"
    );
    let surface = audio_analysis_processing_cli::package_surface();
    assert_eq!(surface.library, "audio-analysis-processing");
    assert!(!surface.operations.is_empty());
}
