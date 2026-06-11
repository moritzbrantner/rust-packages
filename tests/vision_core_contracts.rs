use std::fs;

use video_analysis as va;

#[test]
fn geometry_no_longer_depends_on_video_core() {
    let manifest = fs::read_to_string("crates/math/math-geometry-2d/Cargo.toml").unwrap();
    assert!(!manifest.contains("video-analysis-core.workspace"));
}

#[test]
fn video_bounding_box_round_trips_with_geometry_rect() -> Result<(), Box<dyn std::error::Error>> {
    let bbox = va::BoundingBox::new(1, 2, 3, 4)?;
    let rect = va::geometry2d::RectU32::from(bbox);
    assert_eq!(rect.width, 3);
    let round_trip = va::BoundingBox::try_from(rect)?;
    assert_eq!(round_trip, bbox);
    Ok(())
}

#[test]
fn vision_core_validates_and_serializes_contracts() -> Result<(), Box<dyn std::error::Error>> {
    let region = va::geometry2d::RectU32::new(2, 3, 16, 20)?;
    let keypoint = va::vision_core::VisualKeypoint::new(va::geometry2d::Point2f::new(6.0, 9.0)?)?
        .name("left_eye")?
        .score(0.95)?;
    let detection = va::vision_core::VisualDetection::face(region)?
        .id("face-1")?
        .score(0.9)?
        .keypoint(keypoint)?;
    detection.validate()?;

    let serialized = serde_json::to_value(&detection)?;
    assert_eq!(serialized["kind"], "face");
    assert!(serialized.get("keypoints").is_some());

    assert!(va::vision_core::VisualEmbedding::new(Vec::<f32>::new()).is_err());
    assert!(va::vision_core::IdentityMatch::new("alice", f32::NAN).is_err());
    Ok(())
}

#[test]
fn image_detection_converts_to_visual_detection() -> Result<(), Box<dyn std::error::Error>> {
    let detection = va::image_detection::ImageDetection {
        label: "person".to_string(),
        score: Some(0.8),
        region: va::BoundingBox::new(4, 5, 6, 7)?,
        attributes: Default::default(),
    };

    let visual = detection.to_visual_detection()?;
    assert_eq!(visual.kind, va::vision_core::VisualDetectionKind::Person);
    assert_eq!(visual.region.width, 6);
    assert_eq!(visual.score, Some(0.8));
    Ok(())
}

#[test]
fn face_detection_converts_to_visual_detection_with_size() -> Result<(), Box<dyn std::error::Error>>
{
    let detection = va::image_detection::FaceDetection::new(
        va::image_detection::FaceBox::new(0.25, 0.25, 0.5, 0.5)?,
        0.91,
    )?
    .landmarks(va::image_detection::FaceLandmarks::new(vec![[0.4, 0.45]])?);

    let visual = detection.to_visual_detection_for_size(100, 80)?;
    assert_eq!(visual.kind, va::vision_core::VisualDetectionKind::Face);
    assert_eq!(visual.region, va::geometry2d::RectU32::new(25, 20, 50, 40)?);
    assert_eq!(visual.keypoints.len(), 1);
    Ok(())
}

#[test]
fn embeddings_and_recognition_convert_to_visual_contracts() -> Result<(), Box<dyn std::error::Error>>
{
    let face_embedding = va::image_embeddings::FaceEmbedding::new(vec![1.0, 0.0])?
        .region(va::BoundingBox::new(1, 2, 3, 4)?);
    let visual_embedding = face_embedding.to_visual_embedding()?;
    assert_eq!(
        visual_embedding.kind,
        Some(va::vision_core::VisualDetectionKind::Face)
    );
    assert_eq!(visual_embedding.region.unwrap().height, 4);

    let candidate =
        va::recognition::RecognitionCandidate::new(va::ObservationKind::Face, [1.0, 0.0])?
            .region(va::BoundingBox::new(1, 2, 3, 4)?)
            .track_id("face-1");
    let candidate_embedding = candidate.to_visual_embedding()?;
    assert_eq!(
        candidate_embedding.source_detection_id.as_deref(),
        Some("face-1")
    );

    let match_summary = va::recognition::RecognitionMatch {
        reference_id: "alice".to_string(),
        label: "Alice".to_string(),
        kind: va::ObservationKind::Face,
        score: 0.99,
        attributes: Default::default(),
    };
    let identity_match = match_summary.to_identity_match()?;
    assert_eq!(identity_match.reference_id, "alice");
    assert_eq!(identity_match.label.as_deref(), Some("Alice"));
    Ok(())
}

#[test]
fn tracked_detection_converts_to_visual_detection() -> Result<(), Box<dyn std::error::Error>> {
    let tracked = va::tracking::TrackedDetection::new(va::BoundingBox::new(0, 0, 10, 12)?)
        .label("person")
        .score(0.82)
        .track_hint("track-1");
    let visual = tracked.to_visual_detection()?;
    assert_eq!(visual.kind, va::vision_core::VisualDetectionKind::Person);
    assert_eq!(visual.id.as_deref(), Some("track-1"));
    Ok(())
}

#[test]
fn deterministic_face_identification_anchor_workflow() -> Result<(), Box<dyn std::error::Error>> {
    let detection = va::image_detection::FaceDetection::new(
        va::image_detection::FaceBox::new(0.1, 0.1, 0.4, 0.4)?,
        0.9,
    )?
    .to_visual_detection_for_size(100, 100)?
    .id("face-1")?;

    let embedding = va::vision_core::VisualEmbedding::new([1.0, 0.0])?
        .id("embedding-1")?
        .source_detection_id(detection.id.clone().unwrap())?
        .kind(va::vision_core::VisualDetectionKind::Face)?
        .region(detection.region)?;

    let identity_match = va::vision_core::IdentityMatch::new("alice", 0.99)?
        .source_detection_id(detection.id.unwrap())?
        .source_embedding_id(embedding.id.unwrap())?
        .label("Alice")?
        .kind(va::vision_core::VisualDetectionKind::Face)?;

    assert_eq!(identity_match.reference_id, "alice");
    assert_eq!(identity_match.score, 0.99);
    Ok(())
}
