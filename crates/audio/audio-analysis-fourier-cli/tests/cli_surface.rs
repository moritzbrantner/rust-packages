#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        audio_analysis_fourier_cli::LIBRARY_CRATE,
        "audio-analysis-fourier"
    );
    assert_eq!(audio_analysis_fourier_cli::SURFACE_KIND, "cli");
}
