use criterion::{black_box, criterion_group, criterion_main, Criterion};
use text_analysis_linguistics::{TextNlpConfig, TextNlpPipeline};

fn bench_pipeline(c: &mut Criterion) {
    let pipeline = TextNlpPipeline::new(TextNlpConfig::rich());
    let text = "Alice presented the roadmap in Berlin. The rollout remained stable, \
        and the migration guide explained the API changes in detail."
        .repeat(32);

    c.bench_function("rich_text_nlp_pipeline", |b| {
        b.iter(|| pipeline.analyze_text(black_box(&text)).unwrap())
    });
}

criterion_group!(benches, bench_pipeline);
criterion_main!(benches);
