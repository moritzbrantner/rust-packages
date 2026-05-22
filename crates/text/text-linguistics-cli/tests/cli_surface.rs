#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(text_linguistics_cli::LIBRARY_CRATE, "text-linguistics");
    assert_eq!(text_linguistics_cli::SURFACE_KIND, "cli");
}

#[test]
fn command_schema_reports_bert_ner_default() {
    let schema = text_linguistics_cli::command_schema_json();
    assert!(schema.contains("bert-base-ner"));
    assert!(schema.contains("--entity-recognition <local-model|heuristic>"));
}
