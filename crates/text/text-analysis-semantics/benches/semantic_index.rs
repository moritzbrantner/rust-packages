use criterion::{black_box, criterion_group, criterion_main, Criterion};
use text_analysis_corpus::CorpusOptions;
use text_analysis_semantics::{HashedTextEmbedder, SemanticTextIndex, TextEmbeddingConfig};

fn bench_semantic_index(c: &mut Criterion) {
    let mut index = SemanticTextIndex::new(
        HashedTextEmbedder::new(
            TextEmbeddingConfig {
                dimensions: 256,
                use_idf: true,
            },
            CorpusOptions::default(),
        )
        .unwrap(),
    );

    for i in 0..256 {
        index
            .add_document(
                format!("doc-{i}"),
                format!("rust cargo pipeline benchmark document {i}"),
            )
            .unwrap();
    }

    c.bench_function("semantic_text_index_search", |b| {
        b.iter(|| {
            index
                .search(black_box("cargo pipeline status"), 10)
                .unwrap()
        })
    });
}

criterion_group!(benches, bench_semantic_index);
criterion_main!(benches);
