#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        audio_generation_midi_cli::LIBRARY_CRATE,
        "audio-generation-midi"
    );
    assert_eq!(audio_generation_midi_cli::SURFACE_KIND, "cli");
}
