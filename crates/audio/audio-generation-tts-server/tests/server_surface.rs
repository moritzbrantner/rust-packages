#[test]
fn package_endpoint_reports_wrapped_library() {
    let response = audio_generation_tts_server::response_for("GET", "/api/package", "");
    assert_eq!(response.status_code, 200);
    assert!(response.body.contains("audio-generation-tts"));
}

#[test]
fn run_endpoint_calls_library_surface() {
    let response = audio_generation_tts_server::response_for(
        "POST",
        "/api/run",
        r#"{"operation":"describe","input":{"includeOperations":true}}"#,
    );
    assert_eq!(response.status_code, 200);
    assert!(response.body.contains(r#""operation""#));
}

#[test]
fn run_endpoint_calls_representative_operation() {
    let response = audio_generation_tts_server::response_for(
        "POST",
        "/api/run",
        r#"{"operation":"audio.tts.synthesize","input":{"text":"Hello from server adapter."}}"#,
    );
    assert_eq!(response.status_code, 200);
    assert!(response.body.contains("audio.tts.synthesize"));
    assert!(response.body.contains("\"title\""));
    assert!(response.body.contains("\"summary\""));
}
