#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        text_question_answering_cli::LIBRARY_CRATE,
        "text-question-answering"
    );
    let surface = text_question_answering_cli::package_surface();
    assert_eq!(surface.library, "text-question-answering");
    assert!(!surface.operations.is_empty());
}
