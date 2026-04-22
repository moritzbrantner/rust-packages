use audio_analysis_core::FrameSpec;
use audio_analysis_rhythm::{
    detect_onsets, estimate_tempo, onset_envelope, OnsetDetectorConfig, TempoEstimatorConfig,
};
use audio_analysis_test_support::click_track;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_rhythm(c: &mut Criterion) {
    let samples = click_track(2_000, 120.0, 60.0);
    let frame_spec = FrameSpec::new(80, 20).unwrap();
    c.bench_function("onset_envelope_60s", |b| {
        b.iter(|| onset_envelope(black_box(&samples), 2_000, frame_spec).unwrap())
    });

    let envelope = onset_envelope(&samples, 2_000, frame_spec).unwrap();
    let onset_config = OnsetDetectorConfig {
        strength_threshold: 0.05,
        min_interval_seconds: 0.1,
    };
    c.bench_function("tempo_estimation_60s", |b| {
        b.iter(|| {
            let onsets = detect_onsets(black_box(&envelope), onset_config).unwrap();
            estimate_tempo(&onsets, TempoEstimatorConfig::default()).unwrap()
        })
    });
}

criterion_group!(benches, bench_rhythm);
criterion_main!(benches);
