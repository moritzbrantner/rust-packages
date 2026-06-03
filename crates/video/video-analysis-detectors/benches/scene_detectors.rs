use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use num_rational::Rational64;
use video_analysis_core::{OwnedVideoFrame, SceneDetector, ScenePipeline};
use video_analysis_detectors::{
    AdaptiveDetector, ContentDetector, ContentScoreAlgorithm, HashDetector, HistogramDetector,
    HistogramScoreAlgorithm, ThresholdDetector, WeightedComponent, WeightedCompositeDetector,
};
use video_analysis_test_support::{synthetic_frames, ScenePattern, SyntheticVideoSpec};

fn bench_scene_detectors(c: &mut Criterion) {
    bench_group(c, 64, 64, 120, "smoke");
    bench_group(c, 640, 360, 600, "640x360");
}

fn bench_group(c: &mut Criterion, width: u32, height: u32, frame_count: u64, suffix: &str) {
    let mut group = c.benchmark_group(format!("scene_detectors_{suffix}"));
    group.sample_size(10);
    group.warm_up_time(Duration::from_millis(300));
    group.measurement_time(Duration::from_secs(1));

    let hard_cut = frames(width, height, frame_count)
        .pattern(ScenePattern::SolidColor {
            start_frame: 0,
            end_frame: frame_count / 3,
            rgb: [16, 16, 16],
        })
        .pattern(ScenePattern::SolidColor {
            start_frame: frame_count / 3,
            end_frame: (frame_count * 2) / 3,
            rgb: [240, 48, 48],
        })
        .pattern(ScenePattern::SolidColor {
            start_frame: (frame_count * 2) / 3,
            end_frame: frame_count,
            rgb: [48, 240, 48],
        });
    let hard_cut = synthetic_frames(&hard_cut);

    let fade = synthetic_frames(&frames(width, height, frame_count).pattern(
        ScenePattern::FadeOutIn {
            start_frame: 0,
            end_frame: frame_count,
            high: 240,
            low: 0,
        },
    ));

    let texture = synthetic_frames(
        &frames(width, height, frame_count)
            .pattern(ScenePattern::TextureShift {
                start_frame: 0,
                end_frame: frame_count / 2,
                offset: 0,
            })
            .pattern(ScenePattern::TextureShift {
                start_frame: frame_count / 2,
                end_frame: frame_count,
                offset: 5,
            }),
    );

    group.bench_function(&format!("content_{suffix}_{frame_count}_frames"), |b| {
        b.iter(|| run_detector(ContentDetector::new(27.0, 15), black_box(&hard_cut)))
    });
    group.bench_function(&format!("adaptive_{suffix}_{frame_count}_frames"), |b| {
        b.iter(|| {
            run_detector(
                AdaptiveDetector::new(3.0, 15, 2, 15.0),
                black_box(&hard_cut),
            )
        })
    });
    group.bench_function(
        &format!("threshold_fade_{suffix}_{frame_count}_frames"),
        |b| b.iter(|| run_detector(ThresholdDetector::new(12.0, 15), black_box(&fade))),
    );
    group.bench_function(&format!("histogram_{suffix}_{frame_count}_frames"), |b| {
        b.iter(|| run_detector(HistogramDetector::new(0.05, 256, 15), black_box(&hard_cut)))
    });
    group.bench_function(&format!("hash_{suffix}_{frame_count}_frames"), |b| {
        b.iter(|| run_detector(HashDetector::new(0.395, 16, 2, 15), black_box(&texture)))
    });
    group.bench_function(
        &format!("weighted_composite_content_histogram_{suffix}_{frame_count}_frames"),
        |b| {
            b.iter(|| {
                let detector = WeightedCompositeDetector::builder()
                    .weighted_component(
                        WeightedComponent::new(ContentScoreAlgorithm::new(27.0), 1.0).unwrap(),
                    )
                    .weighted_component(
                        WeightedComponent::new(HistogramScoreAlgorithm::new(0.05, 256), 1.0)
                            .unwrap(),
                    )
                    .threshold(0.5)
                    .min_scene_len(15)
                    .build()
                    .unwrap();
                run_detector(detector, black_box(&hard_cut))
            })
        },
    );
    group.bench_function(
        &format!("pipeline_content_metrics_{suffix}_{frame_count}_frames"),
        |b| b.iter(|| run_pipeline(ContentDetector::new(27.0, 15), black_box(&hard_cut))),
    );
    group.finish();
}

fn frames(width: u32, height: u32, frame_count: u64) -> SyntheticVideoSpec {
    SyntheticVideoSpec::new(width, height, frame_count, Rational64::new(30, 1))
}

fn run_detector<D: SceneDetector>(mut detector: D, frames: &[OwnedVideoFrame]) -> usize {
    let mut cuts = 0usize;
    for frame in frames {
        cuts += detector
            .process_frame(&frame.as_frame(), None)
            .unwrap()
            .len();
    }
    if let Some(last) = frames.last() {
        cuts += detector.finish(last.position, None).unwrap().len();
    }
    black_box(cuts)
}

fn run_pipeline<D: SceneDetector + 'static>(detector: D, frames: &[OwnedVideoFrame]) -> usize {
    let mut pipeline = ScenePipeline::builder()
        .detector(detector)
        .start_in_scene(true)
        .build()
        .unwrap();
    for frame in frames {
        pipeline.process_frame(frame.clone()).unwrap();
    }
    let result = pipeline.finish_detection().unwrap();
    black_box(result.cuts.len() + result.metrics.rows().len())
}

criterion_group!(benches, bench_scene_detectors);
criterion_main!(benches);
