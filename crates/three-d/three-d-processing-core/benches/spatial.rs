use criterion::{black_box, criterion_group, criterion_main, Criterion};
use three_d_processing_core::{
    broad_phase_pairs_3d, Bounds3, BroadPhase3Config, BroadPhase3Strategy, Point3, SpatialCellSize3,
};

fn sparse_bounds(count: usize) -> Vec<Bounds3> {
    (0..count)
        .map(|index| {
            let x = index as f32 * 10.0;
            let y = index as f32 * 7.0;
            Bounds3::new(Point3::new(x, y, 0.0), Point3::new(x + 2.0, y + 2.0, 2.0)).unwrap()
        })
        .collect()
}

fn clustered_bounds(count: usize) -> Vec<Bounds3> {
    (0..count)
        .map(|index| {
            let x = ((index % 128) as f32) * 3.0;
            let y = (((index / 128) % 128) as f32) * 3.0;
            let z = (((index / (128 * 128)) % 32) as f32) * 3.0;
            Bounds3::new(Point3::new(x, y, z), Point3::new(x + 8.0, y + 8.0, z + 8.0)).unwrap()
        })
        .collect()
}

fn overlapping_bounds(count: usize) -> Vec<Bounds3> {
    (0..count)
        .map(|index| {
            let x = (index % 512) as f32;
            Bounds3::new(Point3::new(x, 0.0, 0.0), Point3::new(x + 4.0, 16.0, 16.0)).unwrap()
        })
        .collect()
}

fn config(strategy: BroadPhase3Strategy) -> BroadPhase3Config {
    BroadPhase3Config {
        strategy,
        cell_size: SpatialCellSize3::Auto,
        ..BroadPhase3Config::default()
    }
}

fn bench_spatial(c: &mut Criterion) {
    for (name, make_bounds) in [
        ("sparse", sparse_bounds as fn(usize) -> Vec<Bounds3>),
        ("clustered", clustered_bounds),
        ("many_overlap", overlapping_bounds),
    ] {
        for count in [1_000, 10_000, 50_000] {
            let bounds = make_bounds(count);
            c.bench_function(&format!("3d_{name}_auto_{count}"), |b| {
                b.iter(|| {
                    broad_phase_pairs_3d(
                        black_box(&bounds),
                        black_box(config(BroadPhase3Strategy::Auto)),
                    )
                    .unwrap()
                })
            });
            c.bench_function(&format!("3d_{name}_sweep_{count}"), |b| {
                b.iter(|| {
                    broad_phase_pairs_3d(
                        black_box(&bounds),
                        black_box(config(BroadPhase3Strategy::SweepAndPrune)),
                    )
                    .unwrap()
                })
            });
            c.bench_function(&format!("3d_{name}_grid_{count}"), |b| {
                b.iter(|| {
                    broad_phase_pairs_3d(
                        black_box(&bounds),
                        black_box(config(BroadPhase3Strategy::SpatialHashGrid)),
                    )
                    .unwrap()
                })
            });
            if count == 1_000 {
                c.bench_function(&format!("3d_{name}_brute_{count}"), |b| {
                    b.iter(|| {
                        broad_phase_pairs_3d(
                            black_box(&bounds),
                            black_box(config(BroadPhase3Strategy::BruteForce)),
                        )
                        .unwrap()
                    })
                });
            }
        }
    }
}

criterion_group!(benches, bench_spatial);
criterion_main!(benches);
