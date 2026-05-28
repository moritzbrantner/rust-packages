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

#[test]
fn cli_adapter_runs_representative_operation() {
    let response = audio_analysis_test_support_cli::run_operation(
        "audio.fixtures.generate",
        serde_json::json!({"kind": "sine", "frequencyHz": 440.0, "sampleRate": 1000, "seconds": 0.01}),
    )
    .expect("run operation");
    assert_eq!(response.operation.as_str(), "audio.fixtures.generate");
    assert!(response.value["title"].is_string());
    assert!(response.value["summary"].is_object());
}
