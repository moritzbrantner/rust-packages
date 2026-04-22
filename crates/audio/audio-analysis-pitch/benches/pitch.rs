use audio_analysis_pitch::AutocorrelationPitchDetector;
use audio_analysis_test_support::sine_len;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_pitch(c: &mut Criterion) {
    let detector = AutocorrelationPitchDetector::default();
    for len in [2048, 4096, 8192] {
        let samples = sine_len(440.0, 48_000, len);
        c.bench_function(&format!("autocorrelation_pitch_{len}"), |b| {
            b.iter(|| {
                detector
                    .estimate_samples(black_box(&samples), 48_000)
                    .unwrap()
            })
        });
    }
}

criterion_group!(benches, bench_pitch);
criterion_main!(benches);
