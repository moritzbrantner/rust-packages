#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        audio_analysis_separation_cli::LIBRARY_CRATE,
        "audio-analysis-separation"
    );
    let surface = audio_analysis_separation_cli::package_surface();
    assert_eq!(surface.library, "audio-analysis-separation");
    assert!(!surface.operations.is_empty());
}
