use text_analysis_corpus::CorpusOptions;
use text_analysis_semantics::{HashedTextEmbedder, SemanticTextIndex, TextEmbeddingConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let embedder = HashedTextEmbedder::new(
        TextEmbeddingConfig {
            dimensions: 128,
            use_idf: true,
        },
        CorpusOptions::default(),
    )?;
    let mut index = SemanticTextIndex::new(embedder);
    index.add_document("scene-1", "opening scene with presenter")?;
    index.add_document("scene-2", "cargo build pipeline status")?;

    for result in index.search("presenter on stage", 2)? {
        println!(
            "{} score={:.3} backend={:?}",
            result.id, result.score, result.metadata.backend
        );
    }

    Ok(())
}
