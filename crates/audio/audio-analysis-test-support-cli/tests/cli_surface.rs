#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        audio_analysis_test_support_cli::LIBRARY_CRATE,
        "audio-analysis-test-support"
    );
    let surface = audio_analysis_test_support_cli::package_surface();
    assert_eq!(surface.library, "audio-analysis-test-support");
    assert!(!surface.operations.is_empty());
}
