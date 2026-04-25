mod support;

use support::{checkerboard_frame, dataset_with_scene_text_and_feature};
use video_analysis as va;

#[test]
fn root_facade_reexports_core_and_domain_packages() {
    let frame = checkerboard_frame(8, 8, 0);
    let image = va::image_core::ImageView::from_video_frame(&frame.as_frame()).unwrap();
    assert_eq!(image.width, 8);
    assert_eq!(
        va::image_models::default_sam_model_spec().repo_id,
        "facebook/sam-vit-base"
    );
    let workflow = va::image_comfyui::build_generation_workflow(
        &va::image_comfyui::ImageGenerationRequest::new("red cube"),
    )
    .unwrap();
    assert!(!workflow.nodes.is_empty());

    let summary =
        va::text_features::summarize_text("Rust tests keep package boundaries honest.", 3);
    assert_eq!(summary.stats.words, 6);

    let numeric_summary = va::numbers::summarize_numbers(&[1.0, 2.0, 3.0, f64::NAN]);
    assert_eq!(numeric_summary.finite_count, 3);
    assert_eq!(
        va::numbers::quartiles(&[1.0, 2.0, 3.0, 4.0])
            .unwrap()
            .median,
        2.5
    );

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
    let dense = va::dense::DenseDataset::from_points([
        va::dense::DensePoint::new([0.0, 0.0]).unwrap(),
        va::dense::DensePoint::new([1.0, 1.0]).unwrap(),
    ])
    .unwrap();
    assert_eq!(dense.summary().unwrap().coordinate_stats[0].mean, Some(0.5));

    let mut graph = va::graph_core::Graph::directed();
    graph.connect("scene-a", "scene-b").unwrap();
    graph.connect("scene-b", "scene-c").unwrap();
    assert!(va::graph_core::is_weakly_connected(&graph));
    assert!(!va::graph_core::is_strongly_connected(&graph));
    assert_eq!(
        va::graph_core::shortest_path(&graph, "scene-a", "scene-c")
            .unwrap()
            .unwrap()
            .nodes,
        vec![
            "scene-a".to_string(),
            "scene-b".to_string(),
            "scene-c".to_string()
        ]
    );

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
