#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        audio_analysis_core_cli::LIBRARY_CRATE,
        "audio-analysis-core"
    );
    let surface = audio_analysis_core_cli::package_surface();
    assert_eq!(surface.library, "audio-analysis-core");
    assert!(!surface.operations.is_empty());
}
