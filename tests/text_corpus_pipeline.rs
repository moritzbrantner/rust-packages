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
                rerank: false,
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

#[test]
fn transcript_segment_corpus_export_preserves_retrieval_metadata() {
    let mut segment =
        va::text_core::TextSegmentContract::new(2, "retrieval cites transcript timing");
    segment.stream_id = Some("subs".to_string());
    segment.language = Some("en".to_string());
    segment.timestamp = Some(va::text_core::TimestampContract {
        pts: 3_500,
        timebase: va::text_core::TimebaseContract { num: 1, den: 1_000 },
    });
    segment.duration_seconds = Some(1.75);
    segment
        .attributes
        .insert("speaker".to_string(), "host".to_string());

    let corpus = va::text_lexical::TextCorpus::from_segment_contracts(
        [&segment],
        va::text_lexical::CorpusOptions::default(),
    )
    .unwrap();
    let contracts = corpus.text_document_contracts();
    let search_documents = va::text_retrieval::SearchDocument::from_text_corpus(&corpus);

    assert_eq!(contracts[0].id, "subs:2");
    assert_eq!(contracts[0].timestamp.unwrap().seconds(), 3.5);
    assert_eq!(contracts[0].attributes["speaker"], "host");
    assert_eq!(search_documents[0].metadata["timestamp_seconds"], "3.5");
    assert_eq!(search_documents[0].metadata["duration_seconds"], "1.75");
    assert_eq!(search_documents[0].metadata["language"], "en");
    assert_eq!(
        search_documents[0]
            .source
            .as_ref()
            .unwrap()
            .duration_seconds,
        Some(1.75)
    );
}

#[test]
fn text_corpus_exports_to_primary_text_index_without_losing_metadata() {
    let mut document = va::text_lexical::TextCorpusDocument::new(
        "doc-index",
        "Primary text indexes preserve corpus metadata for durable search.",
    );
    document.language = Some("en".to_string());
    document.timestamp = Some(va::text_core::TimestampContract {
        pts: 42,
        timebase: va::text_core::TimebaseContract { num: 1, den: 1 },
    });
    document.source = Some(va::text_core::TextSourceRef {
        source_id: Some("corpus".to_string()),
        source_kind: Some("fixture".to_string()),
        uri: Some("file:///fixture.txt".to_string()),
        media_timestamp: None,
        duration_seconds: Some(3.0),
    });
    document.provenance.push(va::text_core::TextProvenance {
        crate_name: Some("test".to_string()),
        operation: Some("corpus-export".to_string()),
        model_id: None,
        runtime: Some("deterministic".to_string()),
        confidence: Some(1.0),
    });
    document
        .annotations
        .push(va::text_core::TextAnnotationSpan {
            span: va::text_core::TextSpan {
                byte_start: 0,
                byte_end: 7,
                char_start: 0,
                char_end: 7,
            },
            token_start: Some(0),
            token_end: Some(1),
            source_segment_id: Some("segment-1".to_string()),
        });
    document
        .metadata
        .insert("kind".to_string(), "index".to_string());

    let corpus = va::text_lexical::TextCorpus::from_documents(
        [document],
        va::text_lexical::CorpusOptions::default(),
    )
    .unwrap();
    let index_documents = va::text_index::IndexDocument::from_text_corpus(&corpus);
    let index_document = &index_documents[0];

    assert_eq!(index_document.id, "doc-index");
    assert_eq!(index_document.body, corpus.documents[0].text);
    assert_eq!(index_document.language.as_deref(), Some("en"));
    assert_eq!(index_document.metadata.attributes["kind"], "index");
    assert_eq!(
        index_document
            .metadata
            .source
            .as_ref()
            .and_then(|source| source.media_timestamp)
            .unwrap()
            .seconds(),
        42.0
    );
    assert_eq!(
        index_document.metadata.provenance[0].operation.as_deref(),
        Some("corpus-export")
    );
    assert_eq!(
        index_document.metadata.annotations[0]
            .source_segment_id
            .as_deref(),
        Some("segment-1")
    );
    va::text_index::validate_index_document(index_document).unwrap();

    let embedder = va::text_embeddings::HashedTextEmbedder::new(
        va::text_embeddings::TextEmbeddingConfig {
            dimensions: 32,
            use_idf: false,
        },
        va::text_lexical::CorpusOptions::default(),
    )
    .unwrap();
    let mut index = va::text_index::TextIndex::with_store(
        embedder.clone(),
        va::text_index::MemoryIndexStore::new(),
    );
    index.upsert_documents(&index_documents).unwrap();
    let query = va::text_index::IndexQuery::new("durable corpus metadata", 2);
    let first = index.search(&query).unwrap();
    let second = index.search(&query).unwrap();
    assert_eq!(first, second);
    assert_eq!(first[0].document_id, "doc-index");

    let (migrated, report) = va::text_retrieval::build_memory_text_index(
        embedder,
        &[va::text_retrieval::SearchDocument::from_text_corpus_document(&corpus.documents[0])],
        &va::text_retrieval::IngestionOptions::default(),
    )
    .unwrap();
    assert_eq!(report.documents_received, 1);
    assert_eq!(
        migrated
            .search(&va::text_index::IndexQuery::new(
                "durable corpus metadata",
                1
            ))
            .unwrap()[0]
            .document_id,
        "doc-index"
    );
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
