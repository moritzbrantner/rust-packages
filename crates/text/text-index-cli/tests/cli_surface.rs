#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(text_index_cli::LIBRARY_CRATE, "text-index");
    let surface = text_index_cli::package_surface();
    assert_eq!(surface.library, "moritzbrantner-text-index");
    assert!(!surface.operations.is_empty());
}

#[test]
fn cli_adapter_runs_default_workflow() {
    let response = text_index_cli::run_operation(
        "index.search",
        serde_json::json!({
            "documents": [
                {"id": "doc-1", "body": "Rust durable text indexing"},
                {"id": "doc-2", "body": "Video scene reports"}
            ],
            "query": {"text": "text indexing", "topK": 2}
        }),
    )
    .expect("search");
    assert_eq!(response.value["operation"], "index.search");
    assert!(!response.value["results"].as_array().unwrap().is_empty());
}
