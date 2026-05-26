#[test]
fn package_endpoint_reports_wrapped_library() {
    let response = audio_analysis_test_support_server::response_for("GET", "/api/package", "");
    assert_eq!(response.status_code, 200);
    assert!(response.body.contains("audio-analysis-test-support"));
}

#[test]
fn run_endpoint_calls_library_surface() {
    let response = audio_analysis_test_support_server::response_for(
        "POST",
        "/api/run",
        r#"{"operation":"describe","input":{"includeOperations":true}}"#,
    );
    assert_eq!(response.status_code, 200);
    assert!(response.body.contains(r#""operation""#));
}

#[test]
fn run_endpoint_calls_representative_operation() {
    let response = audio_analysis_test_support_server::response_for(
        "POST",
        "/api/run",
        r#"{"operation":"audio.fixtures.generate","input":{"kind":"sine","frequencyHz":440.0,"sampleRate":1000,"seconds":0.01}}"#,
    );
    assert_eq!(response.status_code, 200);
    assert!(response.body.contains("audio.fixtures.generate"));
}
