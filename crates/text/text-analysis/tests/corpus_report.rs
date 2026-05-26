use text_analysis::{analyze_corpus, CorpusAnalysisOptions};
use text_core::TextDocument;

#[test]
fn corpus_report_includes_tfidf_bm25_duplicates_and_semantic_neighbors() {
    let documents = [
        TextDocument::new("doc-1", "rust cargo crates text analysis"),
        TextDocument::new("doc-2", "rust cargo crates text analysis pipeline"),
        TextDocument::new("doc-3", "video scene reports and camera motion"),
    ];
    let options = CorpusAnalysisOptions {
        query: Some("rust text analysis".to_string()),
        near_duplicate_threshold: 0.4,
        ..CorpusAnalysisOptions::default()
    };

    let report = analyze_corpus(documents, &options).unwrap();

    assert_eq!(report.stats.documents, 3);
    assert!(report.documents[0]
        .tfidf_terms
        .iter()
        .any(|term| term.term == "rust"));
    assert_eq!(report.tfidf_search.as_ref().unwrap()[0].id, "doc-1");
    assert_eq!(report.bm25_search.as_ref().unwrap()[0].id, "doc-1");
    assert!(!report.near_duplicates.is_empty());
    assert!(!report.semantic_neighbors.is_empty());
}
