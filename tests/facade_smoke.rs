mod support;

use support::{checkerboard_frame, dataset_with_scene_text_and_feature};
use video_analysis as va;

#[test]
fn root_facade_reexports_core_and_domain_packages() {
    let frame = checkerboard_frame(8, 8, 0);
    let image = va::image_core::ImageView::from_video_frame(&frame.as_frame()).unwrap();
    assert_eq!(image.width, 8);

    let summary =
        va::text_features::summarize_text("Rust tests keep package boundaries honest.", 3);
    assert_eq!(summary.stats.words, 6);

    let linguistic = va::text_linguistics::analyze_text(
        "Alice launched the API in Berlin.",
        &va::text_linguistics::LinguisticAnalysisOptions::default(),
    )
    .unwrap();
    assert_eq!(
        linguistic
            .language
            .primary
            .as_ref()
            .map(|prediction| prediction.language.as_str()),
        Some("en")
    );
    assert!(linguistic
        .events
        .iter()
        .any(|event| event.lemma == "launch"));

    let dataset = dataset_with_scene_text_and_feature();
    let features = va::features::FeaturePipeline::builder()
        .extractor(va::features::SceneStatsExtractor)
        .extractor(va::features::TranscriptStatsExtractor)
        .build()
        .extract(&dataset)
        .unwrap();
    assert!(features
        .iter()
        .any(|feature| feature.name == "scene.duration_seconds"));
    assert!(features
        .iter()
        .any(|feature| feature.name == "transcript.word_count"));
}
