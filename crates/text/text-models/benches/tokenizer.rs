use criterion::{black_box, criterion_group, criterion_main, Criterion};
use text_models::{
    default_backend_priority, select_text_runtime_backend, TextRuntimeBackend, TextRuntimeCatalog,
};

fn bench_runtime_selection(c: &mut Criterion) {
    let priority = default_backend_priority();
    let catalog = TextRuntimeCatalog::default();

    c.bench_function("text_runtime_backend_selection", |b| {
        b.iter(|| {
            let selected = select_text_runtime_backend(black_box(&priority));
            assert_eq!(selected, TextRuntimeBackend::Onnx);
            black_box(&catalog);
        })
    });
}

criterion_group!(benches, bench_runtime_selection);
criterion_main!(benches);
