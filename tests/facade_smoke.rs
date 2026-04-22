use video_analysis as va;

#[test]
fn root_facade_reexports_core_and_domain_packages() {
    let frame = video_analysis_test_support::checkerboard_frame(8, 8, 0);
    let image = va::image_core::ImageView::from_video_frame(&frame.as_frame()).unwrap();
    assert_eq!(image.width, 8);

    let summary =
        va::text_features::summarize_text("Rust tests keep package boundaries honest.", 3);
    assert_eq!(summary.stats.words, 6);

    let dataset = video_analysis_test_support::dataset_with_scene_text_and_feature();
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
