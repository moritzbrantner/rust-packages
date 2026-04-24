mod support;

use std::io::Cursor;

use num_rational::Rational64;
use support::{
    dataset_with_scene_text_and_feature, frame_position, rgb_frame, scene, text_segment, timestamp,
};
use tempfile::tempdir;
use video_analysis_core::{
    BoundingBox, DetectionResult, FrameTimecode, MetricsSink, MetricsStore, Observation,
    ObservationKind, Scene,
};
use video_analysis_data::{BucketAggregator, BucketConfig, DataRecord};
use video_analysis_dataset::{
    AnalysisDataset, DatasetRecord, FeatureRecord, FeatureValue, SceneRecord,
};
use video_analysis_editing::{crop_frame, grayscale_frame};
use video_analysis_features::{FeaturePipeline, FeatureVectorMeanExtractor, SceneStatsExtractor};
use video_analysis_ingest::{MediaSourceInfo, SourceMode, TextFormat, TextStreamInfo};
use video_analysis_output::{write_detection_result_json, write_scene_list_csv};
use video_analysis_posture::{
    bone_lengths, joint_angle_degrees, normalize_pose3d, Keypoint, Keypoint3d, Pose3dEstimate,
    PoseEstimate, Skeleton,
};
use video_analysis_posture_io::{
    read_coco_keypoints_json, write_coco_keypoints_json, write_stick_figure_gltf,
    write_stick_figure_ply,
};
use video_analysis_recognition::{Embedding, MatchOptions, ReferenceLibrary};
use video_analysis_tracking::{IouTracker, TrackedDetection, TrackingOptions};
use video_analysis_transform::{group_by_scene, record_timestamp_seconds};

#[test]
fn retained_video_crates_support_consumer_smoke_workflows() -> Result<(), Box<dyn std::error::Error>>
{
    let timecode = FrameTimecode::from_seconds(1.5, Rational64::new(30, 1))?;
    assert_eq!(timecode.frame_index, 45);

    let mut metrics = MetricsStore::default();
    metrics.set_metric(0, "content", 0.5);
    let scenes = vec![scene(0, 10)];
    let mut csv = Vec::new();
    write_scene_list_csv(&mut csv, &scenes)?;
    assert!(String::from_utf8(csv)?.contains("Scene Number"));

    let mut aggregator =
        BucketAggregator::new(BucketConfig::record_count(2)?.max_vector_dimensions(8))?;
    let text_segment = text_segment(0, "rust pipelines");
    assert!(aggregator
        .push(DataRecord::text_segment("transcript", &text_segment))?
        .is_empty());
    let buckets = aggregator.push(DataRecord::vector(
        "embedding",
        1,
        Some(timestamp(0)),
        &[1.0, 2.0, 3.0],
    ))?;
    assert_eq!(buckets[0].streams["transcript"].text.segments, 1);

    let mut dataset = AnalysisDataset::empty();
    let clip_scene = Scene {
        start: frame_position(0),
        end: frame_position(2),
    };
    dataset.push(DatasetRecord::Scene(SceneRecord::from_scene(
        0,
        &clip_scene,
    )));
    dataset.push(DatasetRecord::Feature(
        FeatureRecord::new("embedding", FeatureValue::Vector(vec![1.0, 2.0]))
            .scope("global")
            .timestamp(timestamp(0)),
    ));
    assert_eq!(dataset.records.len(), 2);
    assert_eq!(record_timestamp_seconds(&dataset.records[1]), Some(0.0));

    let feature_records = FeaturePipeline::builder()
        .extractor(SceneStatsExtractor)
        .extractor(FeatureVectorMeanExtractor)
        .build()
        .extract(&dataset_with_scene_text_and_feature())?;
    assert!(!feature_records.is_empty());
    assert_eq!(
        group_by_scene(&dataset_with_scene_text_and_feature())[0]
            .scene
            .scene_index,
        0
    );

    let frame = rgb_frame(4, 4, 0, [255, 128, 0]);
    let cropped = crop_frame(&frame.as_frame(), BoundingBox::new(1, 1, 2, 2)?)?;
    let grayscale = grayscale_frame(&cropped.as_frame())?;
    assert_eq!(grayscale.width, 2);

    let mut tracker = IouTracker::new(TrackingOptions::default())?;
    let visible = tracker.update(
        frame_position(0),
        [TrackedDetection::new(BoundingBox::new(0, 0, 8, 8)?).label("person")],
    )?;
    assert_eq!(visible.len(), 1);

    let mut library = ReferenceLibrary::new();
    library.add_reference("alice", "Alice", ObservationKind::Face, [1.0, 0.0])?;
    let matches = library.search(
        &Embedding::new([1.0, 0.0])?,
        Some(&ObservationKind::Face),
        &MatchOptions::default(),
    )?;
    assert_eq!(matches[0].reference_id, "alice");

    let skeleton = Skeleton::coco_17();
    let pose = PoseEstimate::new([
        Keypoint::new("left_shoulder", 0.0, 0.0)?,
        Keypoint::new("left_elbow", 1.0, 0.0)?,
        Keypoint::new("left_wrist", 1.0, 1.0)?,
    ])?;
    assert!(
        joint_angle_degrees(
            pose.keypoint("left_shoulder").unwrap(),
            pose.keypoint("left_elbow").unwrap(),
            pose.keypoint("left_wrist").unwrap(),
        )? > 80.0
    );

    let pose3d = Pose3dEstimate::new([
        Keypoint3d::new(
            "left_hip",
            three_d_processing_core::Point3::new(0.0, 0.0, 0.0),
        )?,
        Keypoint3d::new(
            "left_knee",
            three_d_processing_core::Point3::new(0.0, -1.0, 0.0),
        )?,
        Keypoint3d::new(
            "left_ankle",
            three_d_processing_core::Point3::new(0.0, -2.0, 0.0),
        )?,
    ])?;
    let normalized = normalize_pose3d(&pose3d, "left_hip")?;
    assert!(normalized.keypoint("left_hip").is_some());

    let stick_figure = pose3d.to_stick_figure(skeleton.clone())?;
    let lengths = bone_lengths(&pose3d, &skeleton)?;
    assert!(!lengths.is_empty());

    let temp = tempdir()?;
    let coco_path = temp.path().join("pose.json");
    write_coco_keypoints_json(&coco_path, &[pose])?;
    assert_eq!(read_coco_keypoints_json(&coco_path)?.len(), 1);
    write_stick_figure_ply(temp.path().join("pose.ply"), &stick_figure)?;
    write_stick_figure_gltf(temp.path().join("pose.gltf"), &stick_figure)?;

    let detection_result = DetectionResult {
        scenes,
        cuts: Vec::new(),
        metrics,
        frames_processed: 1,
    };
    let mut json = Cursor::new(Vec::new());
    write_detection_result_json(&mut json, &detection_result)?;
    assert!(String::from_utf8(json.into_inner())?.contains("\"frames_processed\": 1"));

    let source_info = MediaSourceInfo::recorded("transcript.txt").with_text(TextStreamInfo {
        format: TextFormat::Lines,
        language: Some("en".to_string()),
    });
    assert_eq!(source_info.mode, SourceMode::Recorded);

    let observation = Observation::new("fixture", ObservationKind::Text)
        .at_timestamp(timestamp(0))
        .text("hello world")
        .attribute("language", "en");
    assert_eq!(observation.to_text_segment(0).unwrap().text, "hello world");

    Ok(())
}
