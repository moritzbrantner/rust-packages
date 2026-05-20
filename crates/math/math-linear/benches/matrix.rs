use criterion::{black_box, criterion_group, criterion_main, Criterion};
use math_linear::{F32Matrix, MatrixShape};

fn matrix(rows: usize, cols: usize, seed: f32) -> F32Matrix {
    F32Matrix::new(
        MatrixShape::new(rows, cols).unwrap(),
        (0..rows * cols)
            .map(|index| {
                let value = index as f32 * 0.017 + seed;
                value.sin() * 0.5 + value.cos() * 0.25
            })
            .collect(),
    )
    .unwrap()
}

fn bench_matrix(c: &mut Criterion) {
    let left = matrix(128, 128, 0.1);
    let right = matrix(128, 128, 1.3);
    let tall = matrix(512, 128, 2.1);
    let query = matrix(256, 128, 0.7);
    let vector = (0..128)
        .map(|index| (index as f32 * 0.023).sin())
        .collect::<Vec<_>>();

    c.bench_function("matrix_matmul_128x128", |b| {
        b.iter(|| left.matmul(black_box(&right.as_view())).unwrap())
    });

    c.bench_function("matrix_matvec_512x128", |b| {
        b.iter(|| tall.matvec(black_box(&vector)).unwrap())
    });

    c.bench_function("matrix_l2_normalize_rows_512x128", |b| {
        b.iter(|| tall.l2_normalize_rows().unwrap())
    });

    c.bench_function("matrix_pairwise_row_cosine_512x256x128", |b| {
        b.iter(|| {
            tall.pairwise_row_cosine(black_box(&query.as_view()))
                .unwrap()
        })
    });
}

criterion_group!(benches, bench_matrix);
criterion_main!(benches);
