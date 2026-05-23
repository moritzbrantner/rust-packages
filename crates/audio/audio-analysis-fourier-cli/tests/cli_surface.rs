#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        audio_analysis_fourier_cli::LIBRARY_CRATE,
        "audio-analysis-fourier"
    );
    let surface = audio_analysis_fourier_cli::package_surface();
    assert_eq!(surface.library, "audio-analysis-fourier");
    assert!(!surface.operations.is_empty());
}
