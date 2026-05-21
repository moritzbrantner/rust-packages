#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(audio_analysis_io_cli::LIBRARY_CRATE, "audio-analysis-io");
    assert_eq!(audio_analysis_io_cli::SURFACE_KIND, "cli");
}
