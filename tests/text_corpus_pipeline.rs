use std::collections::BTreeMap;

use video_analysis as va;

#[test]
fn text_corpus_flows_through_lexical_semantic_retrieval_and_analysis() {
    let documents = vec![
        corpus_document(
            "doc-lexical",
            "Lexical scoring ranks TF-IDF terms and BM25 matches for local text corpora.",
            "en",
            [("source", "guide"), ("kind", "lexical")],
        ),
        corpus_document(
            "doc-semantic",
            "Semantic search compares deterministic hashed embeddings for related text.",
            "en",
            [("source", "guide"), ("kind", "semantic")],
        ),
        corpus_document(
            "doc-retrieval",
            "Retrieval indexes chunk documents and combine lexical scores with semantic vectors.",
            "en",
            [("source", "guide"), ("kind", "retrieval")],
        ),
        corpus_document(
            "doc-analysis",
            "Corpus analysis reports keywords, summaries, neighbors, and diagnostics.",
            "en",
            [("source", "guide"), ("kind", "analysis")],
        ),
    ];
    let corpus = va::text_lexical::TextCorpus::from_documents(
        documents.clone(),
        va::text_lexical::CorpusOptions::default(),
    )
    .unwrap();

    assert_eq!(corpus.len(), 4);
    assert_eq!(
        corpus.document("doc-retrieval").unwrap().metadata["kind"],
        "retrieval"
    );

    let snapshot = corpus.snapshot().unwrap();
    let snapshot_retrieval = snapshot
        .documents
        .iter()
        .find(|document| document.id == "doc-retrieval")
        .unwrap();
    assert_eq!(snapshot_retrieval.language.as_deref(), Some("en"));
    assert_eq!(snapshot_retrieval.metadata["kind"], "retrieval");

    let tfidf = corpus.to_tfidf_corpus().unwrap();
    assert_eq!(tfidf.stats().documents, 4);
    assert!(tfidf.document("doc-lexical").is_some());
    assert_eq!(
        tfidf.search("chunk retrieval vectors", 1).unwrap()[0].id,
        "doc-retrieval"
    );

    let embedder = va::text_embeddings::HashedTextEmbedder::new(
        va::text_embeddings::TextEmbeddingConfig {
            dimensions: 64,
            use_idf: true,
        },
        va::text_lexical::CorpusOptions::default(),
    )
    .unwrap();
    let lexical_vector = embedder
        .embed_text_with_corpus("TF-IDF lexical terms", Some(&tfidf))
        .unwrap();
    assert_eq!(lexical_vector.dimensions(), 64);

    let mut semantic_index = va::text_embeddings::SemanticTextIndex::new(embedder.clone());
    semantic_index
        .add_documents(corpus.as_text_documents())
        .unwrap();
    let semantic_matches = semantic_index.search("hashed semantic text", 2).unwrap();
    assert!(semantic_matches
        .iter()
        .any(|result| result.id == "doc-semantic"));

    let search_documents = corpus
        .documents
        .iter()
        .map(|document| {
            let mut search_document =
                va::text_retrieval::SearchDocument::new(&document.id, &document.text);
            search_document.metadata = document.metadata.clone();
            if let Some(language) = &document.language {
                search_document
                    .metadata
                    .insert("language".to_string(), language.clone());
            }
            search_document
        })
        .collect::<Vec<_>>();

    let mut retrieval = va::text_retrieval::RetrievalIndex::new(embedder);
    let ingest = retrieval
        .ingest_documents(
            &search_documents,
            &va::text_retrieval::IngestionOptions {
                chunk_tokens: 10,
                chunk_overlap_tokens: 2,
                store_raw_text: true,
            },
        )
        .unwrap();
    assert_eq!(ingest.documents_received, 4);
    assert!(retrieval
        .chunks_iter()
        .any(|chunk| chunk.document_id == "doc-retrieval"
            && chunk
                .metadata
                .get("kind")
                .is_some_and(|kind| kind == "retrieval")
            && chunk
                .metadata
                .get("language")
                .is_some_and(|language| language == "en")));

    let hybrid_results = retrieval
        .search(&va::text_retrieval::SearchQuery::hybrid(
            "retrieval lexical semantic vectors",
            3,
            va::text_retrieval::HybridConfig {
                semantic_weight: 0.6,
                lexical_weight: 0.4,
                rerank_window: 8,
            },
        ))
        .unwrap();
    assert!(!hybrid_results.is_empty());
    assert!(hybrid_results
        .iter()
        .any(|result| result.document_id == "doc-retrieval"));

    let analysis_report = va::text_analysis::analyze_corpus(
        corpus.as_text_documents(),
        &va::text_analysis::CorpusAnalysisOptions {
            query: Some("lexical semantic retrieval".to_string()),
            top_k: 4,
            tfidf_terms_per_document: 4,
            ..va::text_analysis::CorpusAnalysisOptions::default()
        },
    )
    .unwrap();
    assert_eq!(analysis_report.stats.documents, 4);
    assert!(analysis_report
        .documents
        .iter()
        .any(|document| document.id == "doc-retrieval"));
    assert!(analysis_report
        .tfidf_search
        .as_ref()
        .unwrap()
        .iter()
        .any(|result| result.id == "doc-retrieval" || result.id == "doc-lexical"));
}

fn corpus_document(
    id: &str,
    text: &str,
    language: &str,
    metadata: impl IntoIterator<Item = (&'static str, &'static str)>,
) -> va::text_lexical::TextCorpusDocument {
    let mut document = va::text_lexical::TextCorpusDocument::new(id, text);
    document.language = Some(language.to_string());
    document.metadata = metadata
        .into_iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect::<BTreeMap<_, _>>();
    document
}
