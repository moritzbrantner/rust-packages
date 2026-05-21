#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        audio_analysis_separation_cli::LIBRARY_CRATE,
        "audio-analysis-separation"
    );
    assert_eq!(audio_analysis_separation_cli::SURFACE_KIND, "cli");
}
