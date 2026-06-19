#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        audio_generation_tts_cli::LIBRARY_CRATE,
        "audio-generation-tts"
    );
    let surface = audio_generation_tts_cli::package_surface();
    assert_eq!(surface.library, "moritzbrantner-audio-generation-tts");
    assert!(surface
        .operations
        .iter()
        .any(|operation| operation.id.as_str() == "audio.tts.synthesize"));
}

#[test]
fn cli_adapter_runs_representative_operation() {
    let response = audio_generation_tts_cli::run_operation(
        "audio.tts.synthesize",
        serde_json::json!({"text":"Hello from the CLI adapter."}),
    )
    .expect("run operation");
    assert_eq!(response.operation.as_str(), "audio.tts.synthesize");
    assert_eq!(response.value["result"]["audioGenerated"], false);
    assert!(response.value["title"].is_string());
    assert!(response.value["summary"].is_object());
}
