use text_core::AsTextSegmentContract;
use video_analysis as va;

struct ReverseReranker;

impl va::text_model_runtime::TextReranker for ReverseReranker {
    fn rerank(
        &mut self,
        _query: &str,
        documents: &[String],
    ) -> video_analysis_core::Result<Vec<f32>> {
        Ok((0..documents.len())
            .map(|index| (documents.len() - index) as f32)
            .collect())
    }

    fn runtime_backend(&self) -> va::text_model_runtime::TextRuntimeBackend {
        va::text_model_runtime::TextRuntimeBackend::External
    }
}

#[test]
fn sophisticated_text_workspace_flow_preserves_metadata_and_citations() {
    let transcript = va::text_transcripts::parse_srt(
        "1\n00:00:03,000 --> 00:00:05,000\nRust retrieval cites timed transcript chunks.\n",
    )
    .unwrap();
    let mut segment =
        va::text_transcripts::TranscriptSegmentContract::from(transcript.segments[0].clone());
    segment.language = Some("en".to_string());
    let mut text_segment = segment.as_text_segment_contract();
    text_segment.stream_id = Some("subs".to_string());
    let document_id = text_segment.document_id().unwrap();

    let corpus = va::text_lexical::TextCorpus::from_segment_contracts(
        [&text_segment],
        va::text_lexical::CorpusOptions::default(),
    )
    .unwrap();
    let search_documents = va::text_retrieval::SearchDocument::from_text_corpus(&corpus);
    assert_eq!(search_documents[0].metadata["timestamp_seconds"], "3");
    assert_eq!(search_documents[0].metadata["duration_seconds"], "2");

    let document_analysis = va::text_analysis::DocumentAnalysisOptions {
        classification_depth: va::text_analysis::ClassificationDepth::LexicalFallback,
        ..va::text_analysis::DocumentAnalysisOptions::default()
    };
    let corpus_analysis = va::text_analysis::CorpusAnalysisOptions {
        document: va::text_analysis::DocumentAnalysisOptions {
            classification_depth: va::text_analysis::ClassificationDepth::LexicalFallback,
            ..va::text_analysis::DocumentAnalysisOptions::default()
        },
        ..va::text_analysis::CorpusAnalysisOptions::default()
    };
    let options = va::text_analysis::TextWorkspaceOptions {
        ingestion: va::text_retrieval::IngestionOptions {
            chunk_tokens: 8,
            chunk_overlap_tokens: 0,
            store_raw_text: true,
        },
        document_analysis,
        corpus_analysis,
        ..va::text_analysis::TextWorkspaceOptions::default()
    };

    let mut workspace = va::text_analysis::TextWorkspace::new(options);
    workspace
        .ingest_documents([va::text_analysis::WorkspaceDocument::SegmentContract(
            text_segment.clone(),
        )])
        .unwrap();
    let document_report = workspace.analyze_document(&document_id).unwrap();
    assert!(document_report.classification.is_some());
    let corpus_report = workspace.analyze_corpus().unwrap();
    assert!(corpus_report.classification.is_some());

    workspace.build_index().unwrap();
    let index_search = workspace
        .search_index(va::text_index::IndexQuery::new(
            "timed transcript citations",
            2,
        ))
        .unwrap();
    assert_eq!(index_search.results[0].document_id, document_id);
    assert_eq!(
        index_search.results[0]
            .chunk
            .source
            .as_ref()
            .and_then(|source| source.duration_seconds),
        Some(2.0)
    );
    assert!(workspace.inspect_index().unwrap().chunk_count > 0);

    workspace.build_retrieval_index().unwrap();
    let search = workspace
        .search(va::text_retrieval::SearchQuery::hybrid(
            "timed transcript citations",
            2,
            va::text_retrieval::HybridConfig {
                rerank_window: 4,
                ..va::text_retrieval::HybridConfig::default()
            },
        ))
        .unwrap();
    assert_eq!(search.results[0].document_id, document_id);
    assert_eq!(search.results[0].metadata["duration_seconds"], "2");

    let mut reranker = ReverseReranker;
    let mut rerank_context = va::text_retrieval::RerankExecutionContext {
        reranker: Some(&mut reranker),
        model_id: Some("reverse".to_string()),
    };
    let reranked = va::text_retrieval::rerank_documents_with_context(
        va::text_retrieval::RerankRequest {
            query: "timed transcript".to_string(),
            documents: search
                .results
                .iter()
                .map(|result| result.snippet.clone())
                .collect(),
            top_k: 1,
            imported_scores: Vec::new(),
        },
        &mut rerank_context,
    )
    .unwrap();
    assert_eq!(reranked.runtime.as_deref(), Some("external"));

    let qa = va::text_question_answering::answer_question_with_retrieval(
        va::text_question_answering::RetrievalQuestionAnsweringRequest {
            question: "What cites timed transcript chunks?".to_string(),
            documents: search_documents,
            top_k_chunks: 1,
            top_k_answers: 1,
            imported_predictions: vec![va::text_question_answering::ImportedAnswerPrediction {
                kind: None,
                label: None,
                text: Some("Rust retrieval".to_string()),
                score: 0.9,
                attributes: Default::default(),
            }],
            model: Default::default(),
            local_model: None,
            fallback_policy: None,
        },
    )
    .unwrap();
    assert_eq!(qa.answers[0].citations[0].document_id, document_id);
    assert!(qa.answers[0].citations[0]
        .snippet
        .contains("Rust retrieval"));

    let snapshot = workspace.snapshot();
    assert_eq!(snapshot.documents[0].timestamp.unwrap().seconds(), 3.0);
    assert_eq!(
        snapshot.documents[0]
            .source
            .as_ref()
            .and_then(|source| source.duration_seconds),
        Some(2.0)
    );
}
