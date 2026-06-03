use num_rational::Rational64;
use video_analysis_core::{FramePosition, MetricsSink, MetricsStore, SceneDetector, ScenePipeline};
use video_analysis_detectors::{ContentDetector, HistogramDetector};
use video_analysis_test_support::{
    assert_metric_present, synthetic_frames, ScenePattern, SyntheticVideoSpec,
};

fn fixture_frames() -> Vec<video_analysis_core::OwnedVideoFrame> {
    synthetic_frames(
        &SyntheticVideoSpec::new(64, 64, 60, Rational64::new(30, 1))
            .pattern(ScenePattern::SolidColor {
                start_frame: 0,
                end_frame: 20,
                rgb: [16, 16, 16],
            })
            .pattern(ScenePattern::SolidColor {
                start_frame: 20,
                end_frame: 40,
                rgb: [240, 32, 32],
            })
            .pattern(ScenePattern::SolidColor {
                start_frame: 40,
                end_frame: 60,
                rgb: [32, 240, 32],
            }),
    )
}

fn run_pipeline<D: SceneDetector + 'static>(
    detector: D,
    start_in_scene: bool,
    frames: &[video_analysis_core::OwnedVideoFrame],
) -> video_analysis_core::DetectionResult {
    let mut pipeline = ScenePipeline::builder()
        .detector(detector)
        .start_in_scene(start_in_scene)
        .build()
        .unwrap();
    for frame in frames {
        pipeline.process_frame(frame.clone()).unwrap();
    }
    pipeline.finish_detection().unwrap()
}

#[test]
fn scene_list_is_sorted_contiguous_and_uses_frame_range_boundaries() {
    let frames = fixture_frames();
    let result = run_pipeline(ContentDetector::new(27.0, 15), true, &frames);

    assert_eq!(result.frames_processed, 60);
    assert_eq!(
        result
            .scenes
            .iter()
            .map(|scene| (scene.start.frame_index, scene.end.frame_index))
            .collect::<Vec<_>>(),
        [(0, 20), (20, 40), (40, 60)]
    );
    for pair in result.scenes.windows(2) {
        assert_eq!(pair[0].end.frame_index, pair[1].start.frame_index);
    }
}

#[test]
fn start_in_scene_false_returns_no_scene_when_no_cut_is_detected() {
    let frames = synthetic_frames(&SyntheticVideoSpec::new(64, 64, 24, Rational64::new(30, 1)));
    let result = run_pipeline(ContentDetector::new(27.0, 15), false, &frames);

    assert!(result.cuts.is_empty());
    assert!(result.scenes.is_empty());
}

#[test]
fn start_in_scene_true_returns_full_span_when_no_cut_is_detected() {
    let frames = synthetic_frames(&SyntheticVideoSpec::new(64, 64, 24, Rational64::new(30, 1)));
    let result = run_pipeline(ContentDetector::new(27.0, 15), true, &frames);

    assert!(result.cuts.is_empty());
    assert_eq!(result.scenes.len(), 1);
    assert_eq!(result.scenes[0].start.frame_index, 0);
    assert_eq!(result.scenes[0].end.frame_index, 24);
}

#[test]
fn multiple_detectors_deduplicate_same_frame_cuts() {
    let frames = fixture_frames();
    let mut pipeline = ScenePipeline::builder()
        .detector(ContentDetector::new(27.0, 15))
        .detector(HistogramDetector::new(0.05, 32, 15))
        .start_in_scene(true)
        .build()
        .unwrap();
    for frame in &frames {
        pipeline.process_frame(frame.clone()).unwrap();
    }
    let result = pipeline.finish_detection().unwrap();

    assert_eq!(
        result
            .cuts
            .iter()
            .map(|cut| cut.position.frame_index)
            .collect::<Vec<_>>(),
        [20, 40]
    );
}

#[test]
fn metrics_remain_available_after_detection() {
    let frames = fixture_frames();
    let result = run_pipeline(ContentDetector::new(27.0, 15), true, &frames);

    assert_metric_present(&result.metrics, 20, "content_val");
    assert!(result.metrics.rows().contains_key(&20));
}

#[test]
fn metric_store_can_be_reused_by_callers() {
    let mut metrics = MetricsStore::default();
    let position = FramePosition::from_frame_index(7, Rational64::new(30, 1));
    metrics.set_metric(position.frame_index, "content_val", 42.0);

    assert_metric_present(&metrics, 7, "content_val");
}
