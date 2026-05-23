#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(audio_analysis_io_cli::LIBRARY_CRATE, "audio-analysis-io");
    let surface = audio_analysis_io_cli::package_surface();
    assert_eq!(surface.library, "audio-analysis-io");
    assert!(!surface.operations.is_empty());
}
