use text_analysis::{
    analyze_text, AnalysisProfile, DocumentAnalysisOptions, EmbeddingDepth, LinguisticDepth,
};

#[test]
fn deterministic_document_analysis_returns_all_default_sections() {
    let report = analyze_text(
        "doc-1",
        "Alice presented the tokenizer roadmap in Berlin. Rust crates analyze text.",
        &DocumentAnalysisOptions::default(),
    )
    .unwrap();

    assert_eq!(report.id, "doc-1");
    assert!(report.core.stats.basic.words > 0);
    assert!(!report.lexical.keywords.is_empty());
    assert!(!report.similarity.token_shingle_counts.is_empty());
    assert!(report.linguistic.is_some());
    assert!(report.embedding.is_some());
}

#[test]
fn empty_text_reports_embedding_diagnostic_without_panic() {
    let report = analyze_text("empty", "   ", &DocumentAnalysisOptions::default()).unwrap();
    assert!(report.embedding.is_none());
    assert!(report
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "embedding_unavailable"));
}

#[test]
fn zero_ngram_size_is_invalid() {
    let options = DocumentAnalysisOptions {
        ngram_sizes: vec![0],
        ..DocumentAnalysisOptions::default()
    };
    let error = analyze_text("doc", "text", &options).unwrap_err();
    assert!(error.to_string().contains("ngram"));
}

#[test]
fn optional_sections_can_be_disabled() {
    let options = DocumentAnalysisOptions {
        linguistic_depth: LinguisticDepth::Off,
        embedding_depth: EmbeddingDepth::Off,
        ..DocumentAnalysisOptions::default()
    };
    let report = analyze_text("doc", "Rust crates analyze text.", &options).unwrap();
    assert!(report.linguistic.is_none());
    assert!(report.embedding.is_none());
}

#[test]
fn model_backed_profile_uses_local_model_options_or_reports_diagnostic() {
    let options = DocumentAnalysisOptions {
        profile: AnalysisProfile::ModelBacked,
        linguistic_depth: LinguisticDepth::LocalModel {
            bundle_dir: std::env::temp_dir().join("text-analysis-missing-ner-bundle"),
            auto_download: false,
            download_progress: false,
        },
        ..DocumentAnalysisOptions::default()
    };
    let report = analyze_text("doc", "Alice works at OpenAI in Berlin.", &options).unwrap();
    assert!(report.linguistic.is_none());
    assert!(report
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "linguistics_unavailable"));
}
