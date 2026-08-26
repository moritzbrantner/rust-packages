#![cfg(feature = "three-d")]

use media_core::AnnotationSelector;
use three_d_processing_core::annotations::{
    CoordinateFrameRef, CoordinateUnit, SpatialBinding, SpatialEntityRef, SpatialSelector,
};
use three_d_processing_core::{CameraPose3d, PinholeIntrinsicsd, Point3d};

fn round_trip(binding: &SpatialBinding) -> SpatialBinding {
    let encoded = serde_json::to_vec(binding).expect("spatial binding should serialize");
    let decoded: SpatialBinding =
        serde_json::from_slice(&encoded).expect("spatial binding should deserialize");
    decoded
        .validate()
        .expect("round-tripped spatial binding should remain valid");
    decoded
}

#[test]
fn media_frame_round_trips_with_colmap_camera_pose() {
    let source_selector = AnnotationSelector::Frame { frame_index: 42 };
    source_selector
        .validate()
        .expect("media frame selector should be valid");

    let scene_frame = CoordinateFrameRef::local("colmap-world")
        .expect("scene frame should be valid")
        .unit(CoordinateUnit::Arbitrary);
    let pose = CameraPose3d::from_colmap_world_to_camera(
        1.0, 0.0, 0.0, 0.0, 0.25, -0.5, 1.5,
    )
    .expect("COLMAP pose should convert to the canonical camera pose");
    let intrinsics = PinholeIntrinsicsd::new(1920, 1080, 1200.0, 1200.0, 960.0, 540.0)
        .expect("pinhole intrinsics should be valid");
    let calibration_ref = SpatialEntityRef::new("colmap", "camera", "7")
        .expect("calibration reference should be valid");

    let binding = SpatialBinding::new(SpatialSelector::CameraPose {
        frame: scene_frame,
        pose,
        intrinsics: Some(intrinsics),
        calibration_ref: Some(calibration_ref),
        uncertainty: None,
    })
    .expect("camera selector should be valid")
    .with_source_selector(source_selector.clone())
    .expect("media selector should bind to the camera pose");

    let decoded = round_trip(&binding);

    assert_eq!(decoded, binding);
    assert_eq!(
        decoded
            .source_selector_as::<AnnotationSelector>()
            .expect("media selector should deserialize"),
        Some(source_selector)
    );
}

#[test]
fn media_region_round_trips_with_scene_point() {
    let source_selector = AnnotationSelector::Region2d {
        x: 0.25,
        y: 0.20,
        width: 0.50,
        height: 0.40,
        coordinate_space: Some("normalized".to_string()),
    };
    source_selector
        .validate()
        .expect("media region selector should be valid");

    let scene_frame = CoordinateFrameRef::local("colmap-world")
        .expect("scene frame should be valid")
        .unit(CoordinateUnit::Arbitrary);
    let binding = SpatialBinding::new(SpatialSelector::Point3 {
        frame: scene_frame,
        point: Point3d::new(1.25, -0.5, 2.0),
        uncertainty: None,
    })
    .expect("3D point selector should be valid")
    .with_source_selector(source_selector.clone())
    .expect("media region should bind to the scene point");

    let decoded = round_trip(&binding);

    assert_eq!(decoded, binding);
    assert_eq!(
        decoded
            .source_selector_as::<AnnotationSelector>()
            .expect("media selector should deserialize"),
        Some(source_selector)
    );
}
