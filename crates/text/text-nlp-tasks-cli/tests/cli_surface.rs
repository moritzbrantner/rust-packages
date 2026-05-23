#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(text_nlp_tasks_cli::LIBRARY_CRATE, "text-nlp-tasks");
    let surface = text_nlp_tasks_cli::package_surface();
    assert_eq!(surface.library, "text-nlp-tasks");
    assert!(!surface.operations.is_empty());
}
