#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(text_transcripts_cli::LIBRARY_CRATE, "text-transcripts");
    let surface = text_transcripts_cli::package_surface();
    assert_eq!(surface.library, "text-transcripts");
    assert!(!surface.operations.is_empty());
}
