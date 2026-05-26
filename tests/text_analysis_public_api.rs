use video_analysis as va;

#[test]
fn facade_reexports_unified_text_analysis() {
    let report = va::text_analysis::analyze_text(
        "doc-1",
        "Rust crates analyze text.",
        &va::text_analysis::DocumentAnalysisOptions::default(),
    )
    .unwrap();

    assert_eq!(report.id, "doc-1");
    assert!(report.core.stats.basic.words > 0);
    assert!(!report.lexical.keywords.is_empty());
}

#[test]
fn text_analysis_interops_with_existing_text_crates() {
    let docs = [
        va::text_core::TextDocument::new("doc-1", "rust text analysis"),
        va::text_core::TextDocument::new("doc-2", "video scene analysis"),
    ];
    let corpus = va::text_lexical::TfIdfCorpus::from_documents(
        docs,
        va::text_lexical::CorpusOptions::default(),
    )
    .unwrap();
    let embedder = va::text_embeddings::HashedTextEmbedder::new(
        va::text_embeddings::TextEmbeddingConfig {
            dimensions: 32,
            use_idf: true,
        },
        va::text_lexical::CorpusOptions::default(),
    )
    .unwrap();
    let vector = embedder
        .embed_text_with_corpus("rust text analysis", Some(&corpus))
        .unwrap();

    assert_eq!(vector.dimensions(), 32);
}
