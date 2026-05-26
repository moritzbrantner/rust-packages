use text_core::{detailed_text_stats, TextDocument};
use text_embeddings::{HashedTextEmbedder, TextEmbeddingConfig};
use text_lexical::{
    keywords, token_shingle_similarity, Bm25Corpus, Bm25Options, CorpusOptions, KeywordOptions,
    TfIdfCorpus,
};
use video_analysis_core::Result;

use crate::document::embedding_section;
use crate::{
    invalid_argument, CorpusAnalysisOptions, CorpusAnalysisReport, CorpusDocumentAnalysis,
    DocumentSimilarityPair, EmbeddingDepth, TextAnalysisDiagnostic,
};

pub fn analyze_corpus<'a, I>(
    documents: I,
    options: &CorpusAnalysisOptions,
) -> Result<CorpusAnalysisReport>
where
    I: IntoIterator<Item = TextDocument<'a>>,
{
    if options.top_k == 0 {
        return Err(invalid_argument("top_k must be greater than zero"));
    }
    if options.near_duplicate_shingle_size == 0 {
        return Err(invalid_argument(
            "near duplicate shingle size must be greater than zero",
        ));
    }
    let documents = documents.into_iter().collect::<Vec<_>>();
    let mut diagnostics = Vec::new();
    let tfidf = TfIdfCorpus::from_documents(documents.iter().copied(), CorpusOptions::default())?;
    let bm25 = Bm25Corpus::from_documents(documents.iter().copied(), Bm25Options::default())?;
    let stats = tfidf.stats();
    let term_stats = tfidf.term_stats(options.top_k);
    let mut document_reports = Vec::new();
    for document in &documents {
        let tfidf_terms = tfidf.document_tfidf(document.id, options.tfidf_terms_per_document)?;
        let mut document_options = options.document.clone();
        document_options.embedding_depth = options.document.embedding_depth.clone();
        let embedding_preview = embedding_section(
            document.text,
            &document_options,
            Some(&tfidf),
            &mut diagnostics,
        )
        .map(|embedding| embedding.preview);
        document_reports.push(CorpusDocumentAnalysis {
            id: document.id.to_string(),
            stats: detailed_text_stats(document.text, &options.document.processing),
            tfidf_terms,
            keywords: keywords(
                document.text,
                &KeywordOptions {
                    max_terms: options.document.keyword_limit,
                    ..KeywordOptions::default()
                },
            ),
            embedding_preview,
        });
    }
    let tfidf_search = options
        .query
        .as_deref()
        .map(|query| tfidf.search(query, options.top_k))
        .transpose()?;
    let bm25_search = options
        .query
        .as_deref()
        .map(|query| bm25.search(query, options.top_k))
        .transpose()?;
    let near_duplicates = if options.include_near_duplicates {
        near_duplicate_pairs(&documents, options)?
    } else {
        Vec::new()
    };
    let semantic_neighbors = if options.include_semantic_neighbors {
        semantic_neighbor_pairs(&documents, options, &tfidf, &mut diagnostics)?
    } else {
        Vec::new()
    };

    Ok(CorpusAnalysisReport {
        stats,
        term_stats,
        documents: document_reports,
        tfidf_search,
        bm25_search,
        near_duplicates,
        semantic_neighbors,
        diagnostics,
    })
}

fn near_duplicate_pairs(
    documents: &[TextDocument<'_>],
    options: &CorpusAnalysisOptions,
) -> Result<Vec<DocumentSimilarityPair>> {
    let mut pairs = Vec::new();
    for left_index in 0..documents.len() {
        for right_index in (left_index + 1)..documents.len() {
            let similarity = token_shingle_similarity(
                documents[left_index].text,
                documents[right_index].text,
                options.near_duplicate_shingle_size,
                &options.document.processing,
            )?;
            if similarity.jaccard >= options.near_duplicate_threshold {
                pairs.push(DocumentSimilarityPair {
                    left_id: documents[left_index].id.to_string(),
                    right_id: documents[right_index].id.to_string(),
                    score: similarity.jaccard,
                    metric: "token_shingle_jaccard".to_string(),
                });
            }
        }
    }
    pairs.sort_by(|left, right| {
        right
            .score
            .total_cmp(&left.score)
            .then_with(|| left.left_id.cmp(&right.left_id))
            .then_with(|| left.right_id.cmp(&right.right_id))
    });
    Ok(pairs)
}

fn semantic_neighbor_pairs(
    documents: &[TextDocument<'_>],
    options: &CorpusAnalysisOptions,
    corpus: &TfIdfCorpus,
    diagnostics: &mut Vec<TextAnalysisDiagnostic>,
) -> Result<Vec<DocumentSimilarityPair>> {
    let (dimensions, use_idf) = match options.document.embedding_depth {
        EmbeddingDepth::Hashed {
            dimensions,
            use_idf,
        } => (dimensions.max(1), use_idf),
        EmbeddingDepth::Off => return Ok(Vec::new()),
        _ => {
            diagnostics.push(TextAnalysisDiagnostic::warning(
                "semantic_neighbors_unavailable",
                "semantic corpus neighbors currently use hashed embeddings",
            ));
            return Ok(Vec::new());
        }
    };
    let embedder = HashedTextEmbedder::new(
        TextEmbeddingConfig {
            dimensions,
            use_idf,
        },
        CorpusOptions::default(),
    )?;
    let mut vectors = Vec::new();
    for document in documents {
        match embedder.embed_text_with_corpus(document.text, Some(corpus)) {
            Ok(vector) => vectors.push(Some(vector)),
            Err(error) => {
                diagnostics.push(TextAnalysisDiagnostic::warning(
                    "semantic_neighbor_embedding_unavailable",
                    format!("{}: {error}", document.id),
                ));
                vectors.push(None);
            }
        }
    }
    let mut pairs = Vec::new();
    for left_index in 0..documents.len() {
        for right_index in (left_index + 1)..documents.len() {
            let Some(left) = vectors[left_index].as_ref() else {
                continue;
            };
            let Some(right) = vectors[right_index].as_ref() else {
                continue;
            };
            pairs.push(DocumentSimilarityPair {
                left_id: documents[left_index].id.to_string(),
                right_id: documents[right_index].id.to_string(),
                score: cosine(left.as_slice(), right.as_slice()),
                metric: "hashed_embedding_cosine".to_string(),
            });
        }
    }
    pairs.sort_by(|left, right| {
        right
            .score
            .total_cmp(&left.score)
            .then_with(|| left.left_id.cmp(&right.left_id))
            .then_with(|| left.right_id.cmp(&right.right_id))
    });
    pairs.truncate(options.top_k);
    Ok(pairs)
}

fn cosine(left: &[f32], right: &[f32]) -> f32 {
    let len = left.len().min(right.len());
    if len == 0 {
        return 0.0;
    }
    let mut dot = 0.0;
    let mut left_norm = 0.0;
    let mut right_norm = 0.0;
    for index in 0..len {
        dot += left[index] * right[index];
        left_norm += left[index] * left[index];
        right_norm += right[index] * right[index];
    }
    if left_norm <= f32::EPSILON || right_norm <= f32::EPSILON {
        0.0
    } else {
        dot / (left_norm.sqrt() * right_norm.sqrt())
    }
}
