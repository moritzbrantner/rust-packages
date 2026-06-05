use criterion::{black_box, criterion_group, criterion_main, Criterion};
use maps_kernels_core::{densify_line_flat, path_summary_flat, simplify_line_flat};

fn line(point_count: usize) -> Vec<f64> {
    let mut coordinates = Vec::with_capacity(point_count * 2);
    for index in 0..point_count {
        let x = index as f64;
        let y = (index as f64 * 0.017).sin();
        coordinates.push(x);
        coordinates.push(y);
    }
    coordinates
}

fn bench_paths(c: &mut Criterion) {
    let path = line(10_000);
    c.bench_function("path_summary_10k", |b| {
        b.iter(|| path_summary_flat(black_box(&path), false).unwrap())
    });
    c.bench_function("simplify_line_10k", |b| {
        b.iter(|| simplify_line_flat(black_box(&path), black_box(0.01)).unwrap())
    });
    c.bench_function("densify_line_10k", |b| {
        b.iter(|| densify_line_flat(black_box(&path), black_box(0.5)).unwrap())
    });
}

criterion_group!(benches, bench_paths);
criterion_main!(benches);
