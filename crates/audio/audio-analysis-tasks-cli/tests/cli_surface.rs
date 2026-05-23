#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        audio_analysis_tasks_cli::LIBRARY_CRATE,
        "audio-analysis-tasks"
    );
    let surface = audio_analysis_tasks_cli::package_surface();
    assert_eq!(surface.library, "audio-analysis-tasks");
    assert!(!surface.operations.is_empty());
}
