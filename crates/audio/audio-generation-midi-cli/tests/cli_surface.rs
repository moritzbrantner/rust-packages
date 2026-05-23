#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        audio_generation_midi_cli::LIBRARY_CRATE,
        "audio-generation-midi"
    );
    let surface = audio_generation_midi_cli::package_surface();
    assert_eq!(surface.library, "audio-generation-midi");
    assert!(!surface.operations.is_empty());
}
