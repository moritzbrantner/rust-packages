#[test]
fn selected_text_cli_surfaces_passthrough_library_operations() {
    let core = text_core_cli::run_operation(
        "text.statistics",
        serde_json::json!({"text": "Hello text."}),
    )
    .expect("text core cli");
    assert_eq!(core.value["value"]["wordCount"], 2);

    let embeddings = text_embeddings_cli::run_operation(
        "embeddings.similarity",
        serde_json::json!({"left": "rust text", "right": "rust language"}),
    )
    .expect("text embeddings cli");
    assert!(embeddings.value["similarity"].as_f64().unwrap().is_finite());

    let retrieval = text_retrieval_cli::run_operation(
        "retrieval.chunk",
        serde_json::json!({"documents": [{"id": "doc-1", "body": "Rust text retrieval."}]}),
    )
    .expect("text retrieval cli");
    assert!(!retrieval.value["chunks"].as_array().unwrap().is_empty());

    let transcripts = text_transcripts_cli::run_operation(
        "transcripts.formatSrt",
        serde_json::json!({"segments": [{"index": 0, "startSeconds": 0.0, "endSeconds": 1.0, "text": "Hello", "isFinal": true}]}),
    )
    .expect("text transcripts cli");
    assert!(transcripts.value["srt"].as_str().unwrap().contains("Hello"));
}

#[test]
fn selected_text_server_surfaces_passthrough_library_operations() {
    let core = text_core_server::response_for(
        "POST",
        "/api/run",
        r#"{"operation":"text.normalize","input":{"text":" Hello  TEXT ","lowercase":true}}"#,
    );
    assert_eq!(core.status_code, 200);
    assert!(core.body.contains("hello text"));

    let embeddings = text_embeddings_server::response_for(
        "POST",
        "/api/run",
        r#"{"operation":"embeddings.embed","input":{"texts":["rust"],"dimensions":8}}"#,
    );
    assert_eq!(embeddings.status_code, 200);
    assert!(embeddings.body.contains("embedding"));

    let retrieval = text_retrieval_server::response_for(
        "POST",
        "/api/run",
        r#"{"operation":"retrieval.search","input":{"documents":[{"id":"doc-1","body":"rust retrieval"}],"query":"rust","mode":"full_text"}}"#,
    );
    assert_eq!(retrieval.status_code, 200);
    assert!(retrieval.body.contains("doc-1"));

    let transcripts = text_transcripts_server::response_for(
        "POST",
        "/api/run",
        r#"{"operation":"transcripts.normalize","input":{"segments":[{"index":0,"startSeconds":0.0,"endSeconds":1.0,"text":" Hello ","isFinal":true}]}}"#,
    );
    assert_eq!(transcripts.status_code, 200);
    assert!(transcripts.body.contains("Hello"));
}
