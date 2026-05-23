#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        audio_analysis_synthesis_cli::LIBRARY_CRATE,
        "audio-analysis-synthesis"
    );
    let surface = audio_analysis_synthesis_cli::package_surface();
    assert_eq!(surface.library, "audio-analysis-synthesis");
    assert!(!surface.operations.is_empty());
}
