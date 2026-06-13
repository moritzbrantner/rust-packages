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

    let embedding_backends = text_embeddings_cli::run_operation(
        "embeddings.backends",
        serde_json::json!({"dimensions": 16}),
    )
    .expect("text embeddings backend cli");
    assert_eq!(embedding_backends.value["defaultBackend"], "hashed");
    assert_eq!(
        embedding_backends.value["backends"][0]["model"]["dimensions"],
        16
    );

    let classification_schema =
        text_classification_cli::run_operation("classification.schema", serde_json::json!({}))
            .expect("text classification schema cli");
    assert!(!classification_schema.value["tasks"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(!classification_schema.value["models"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(!classification_schema.value["registeredPresets"]
        .as_array()
        .unwrap()
        .is_empty());

    let generation_perplexity = text_generation_cli::run_operation(
        "generation.perplexity",
        serde_json::json!({
            "trainingTexts": ["rust text analysis rust text analysis"],
            "text": "rust text analysis",
            "order": 2
        }),
    )
    .expect("text generation perplexity cli");
    assert!(generation_perplexity.value["perplexity"]
        .as_f64()
        .unwrap()
        .is_finite());

    let lexical_corpus_stats = text_lexical_cli::run_operation(
        "lexical.corpusStats",
        serde_json::json!({
            "documents": [
                {"id": "doc-1", "text": "rust text analysis"},
                {"id": "doc-2", "text": "video scene analysis"}
            ],
            "documentId": "doc-1",
            "limit": 8
        }),
    )
    .expect("text lexical corpus stats cli");
    assert_eq!(lexical_corpus_stats.value["stats"]["documents"], 2);
    assert!(!lexical_corpus_stats.value["terms"]
        .as_array()
        .unwrap()
        .is_empty());

    let retrieval = text_retrieval_cli::run_operation(
        "retrieval.chunk",
        serde_json::json!({"documents": [{"id": "doc-1", "body": "Rust text retrieval."}]}),
    )
    .expect("text retrieval cli");
    assert!(!retrieval.value["chunks"].as_array().unwrap().is_empty());

    let index = text_index_cli::run_operation(
        "index.search",
        serde_json::json!({
            "documents": [{"id": "doc-1", "body": "Rust text index search."}],
            "query": {"text": "text index", "topK": 2}
        }),
    )
    .expect("text index cli");
    assert_eq!(index.value["operation"], "index.search");
    assert_eq!(index.value["summary"]["status"], "ok");
    assert!(!index.value["result"]["results"]
        .as_array()
        .unwrap()
        .is_empty());

    let analysis = text_analysis_cli::run_operation(
        "analysis.document",
        serde_json::json!({"id": "doc-1", "text": "Rust text analysis is deterministic."}),
    )
    .expect("text analysis cli");
    assert_eq!(analysis.value["operation"], "analysis.document");
    assert!(
        analysis.value["result"]["core"]["stats"]["basic"]["words"]
            .as_u64()
            .unwrap()
            > 0
    );

    let linguistics = text_linguistics_cli::run_operation(
        "linguistics.language",
        serde_json::json!({"text": "This is a simple English sentence.", "sentenceLevel": true}),
    )
    .expect("text linguistics cli");
    assert_eq!(linguistics.value["operation"], "linguistics.language");
    assert_eq!(linguistics.value["result"]["primary"]["language"], "en");

    let generation_linguistics = text_generation_linguistics_cli::run_operation(
        "generationLinguistics.analysisTerms",
        serde_json::json!({"text": "Alice presented the tokenizer roadmap in Berlin."}),
    )
    .expect("text generation linguistics cli");
    assert_eq!(
        generation_linguistics.value["operation"],
        "generationLinguistics.analysisTerms"
    );
    assert!(!generation_linguistics.value["result"]["terms"]
        .as_array()
        .unwrap()
        .is_empty());

    let runtime = text_model_runtime_cli::run_operation(
        "runtime.softmax",
        serde_json::json!({"logits": [0.0, 1.0]}),
    )
    .expect("text model runtime cli");
    assert_eq!(runtime.value["operation"], "runtime.softmax");
    assert_eq!(
        runtime.value["result"]["probabilities"]
            .as_array()
            .unwrap()
            .len(),
        2
    );

    let qa = text_question_answering_cli::run_operation(
        "qa.answer",
        serde_json::json!({
            "question": "What is reliable?",
            "context": "Rust is reliable.",
            "importedPredictions": [{"text": "Rust", "score": 0.9}]
        }),
    )
    .expect("text question answering cli");
    assert_eq!(qa.value["operation"], "qa.answer");
    assert!(!qa.value["result"]["answers"].as_array().unwrap().is_empty());

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

    let embedding_backends = text_embeddings_server::response_for(
        "POST",
        "/api/run",
        r#"{"operation":"embeddings.backends","input":{"dimensions":16}}"#,
    );
    assert_eq!(embedding_backends.status_code, 200);
    let embedding_backends = response_json(&embedding_backends);
    assert_eq!(embedding_backends["value"]["defaultBackend"], "hashed");
    assert_eq!(
        embedding_backends["value"]["summary"]["backendCount"],
        serde_json::json!(3)
    );

    let classification_schema = text_classification_server::response_for(
        "POST",
        "/api/run",
        r#"{"operation":"classification.schema","input":{}}"#,
    );
    assert_eq!(classification_schema.status_code, 200);
    let classification_schema = response_json(&classification_schema);
    assert!(!classification_schema["value"]["tasks"]
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(
        classification_schema["value"]["summary"]["taskCount"],
        serde_json::json!(3)
    );

    let generation_perplexity = text_generation_server::response_for(
        "POST",
        "/api/run",
        r#"{"operation":"generation.perplexity","input":{"trainingTexts":["rust text analysis rust text analysis"],"text":"rust text analysis","order":2}}"#,
    );
    assert_eq!(generation_perplexity.status_code, 200);
    let generation_perplexity = response_json(&generation_perplexity);
    assert!(generation_perplexity["value"]["perplexity"]
        .as_f64()
        .unwrap()
        .is_finite());
    assert_eq!(
        generation_perplexity["value"]["summary"]["isInfinite"],
        serde_json::json!(false)
    );

    let lexical_corpus_stats = text_lexical_server::response_for(
        "POST",
        "/api/run",
        r#"{"operation":"lexical.corpusStats","input":{"documents":[{"id":"doc-1","text":"rust text analysis"},{"id":"doc-2","text":"video scene analysis"}],"documentId":"doc-1","limit":8}}"#,
    );
    assert_eq!(lexical_corpus_stats.status_code, 200);
    let lexical_corpus_stats = response_json(&lexical_corpus_stats);
    assert_eq!(
        lexical_corpus_stats["value"]["stats"]["documents"],
        serde_json::json!(2)
    );
    assert!(!lexical_corpus_stats["value"]["documentTfidf"]
        .as_array()
        .unwrap()
        .is_empty());

    let retrieval = text_retrieval_server::response_for(
        "POST",
        "/api/run",
        r#"{"operation":"retrieval.search","input":{"documents":[{"id":"doc-1","body":"rust retrieval"}],"query":"rust","mode":"full_text"}}"#,
    );
    assert_eq!(retrieval.status_code, 200);
    assert!(retrieval.body.contains("doc-1"));

    let index = text_index_server::response_for(
        "POST",
        "/api/run",
        r#"{"operation":"index.search","input":{"documents":[{"id":"doc-1","body":"rust text index search"}],"query":{"text":"text index","topK":2}}}"#,
    );
    assert_eq!(index.status_code, 200);
    let index = response_json(&index);
    assert_eq!(index["value"]["operation"], "index.search");
    assert_eq!(index["value"]["summary"]["status"], serde_json::json!("ok"));
    assert!(!index["value"]["result"]["results"]
        .as_array()
        .unwrap()
        .is_empty());

    let analysis = text_analysis_server::response_for(
        "POST",
        "/api/run",
        r#"{"operation":"analysis.document","input":{"id":"doc-1","text":"Rust text analysis is deterministic."}}"#,
    );
    assert_eq!(analysis.status_code, 200);
    let analysis = response_json(&analysis);
    assert_eq!(analysis["value"]["operation"], "analysis.document");
    assert!(
        analysis["value"]["result"]["core"]["stats"]["basic"]["words"]
            .as_u64()
            .unwrap()
            > 0
    );

    let linguistics = text_linguistics_server::response_for(
        "POST",
        "/api/run",
        r#"{"operation":"linguistics.language","input":{"text":"This is a simple English sentence.","sentenceLevel":true}}"#,
    );
    assert_eq!(linguistics.status_code, 200);
    let linguistics = response_json(&linguistics);
    assert_eq!(linguistics["value"]["operation"], "linguistics.language");
    assert_eq!(linguistics["value"]["result"]["primary"]["language"], "en");

    let generation_linguistics = text_generation_linguistics_server::response_for(
        "POST",
        "/api/run",
        r#"{"operation":"generationLinguistics.analysisTerms","input":{"text":"Alice presented the tokenizer roadmap in Berlin."}}"#,
    );
    assert_eq!(generation_linguistics.status_code, 200);
    let generation_linguistics = response_json(&generation_linguistics);
    assert_eq!(
        generation_linguistics["value"]["operation"],
        "generationLinguistics.analysisTerms"
    );
    assert!(!generation_linguistics["value"]["result"]["terms"]
        .as_array()
        .unwrap()
        .is_empty());

    let runtime = text_model_runtime_server::response_for(
        "POST",
        "/api/run",
        r#"{"operation":"runtime.softmax","input":{"logits":[0.0,1.0]}}"#,
    );
    assert_eq!(runtime.status_code, 200);
    let runtime = response_json(&runtime);
    assert_eq!(runtime["value"]["operation"], "runtime.softmax");
    assert_eq!(
        runtime["value"]["result"]["probabilities"]
            .as_array()
            .unwrap()
            .len(),
        2
    );

    let qa = text_question_answering_server::response_for(
        "POST",
        "/api/run",
        r#"{"operation":"qa.answer","input":{"question":"What is reliable?","context":"Rust is reliable.","importedPredictions":[{"text":"Rust","score":0.9}]}}"#,
    );
    assert_eq!(qa.status_code, 200);
    let qa = response_json(&qa);
    assert_eq!(qa["value"]["operation"], "qa.answer");
    assert!(!qa["value"]["result"]["answers"]
        .as_array()
        .unwrap()
        .is_empty());

    let transcripts = text_transcripts_server::response_for(
        "POST",
        "/api/run",
        r#"{"operation":"transcripts.normalize","input":{"segments":[{"index":0,"startSeconds":0.0,"endSeconds":1.0,"text":" Hello ","isFinal":true}]}}"#,
    );
    assert_eq!(transcripts.status_code, 200);
    assert!(transcripts.body.contains("Hello"));
}

fn response_json(response: &runtime_core::server::HttpResponse) -> serde_json::Value {
    serde_json::from_str(&response.body).expect("server response JSON")
}
