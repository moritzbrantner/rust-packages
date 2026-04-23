use video_analysis as va;

#[test]
fn facade_reexports_sam_defaults_for_detection_and_segmentation() {
    let image_spec = va::image_segmentation::default_sam_model_spec();
    assert_eq!(image_spec.repo_id, "facebook/sam-vit-base");

    let detection_spec = va::image_detection::default_detection_model_spec();
    assert_eq!(detection_spec.repo_id, "facebook/sam-vit-base");

    let video_spec = va::video_segmentation::default_sam2_model_spec();
    assert_eq!(video_spec.repo_id, "facebook/sam2.1-hiera-large");
}

#[test]
fn segmentation_masks_can_be_converted_into_detection_boxes() {
    let segment = va::image_segmentation::ImageSegment::new(
        va::image_segmentation::BinaryMask::filled_rect(
            10,
            8,
            va::BoundingBox::new(3, 2, 4, 3).unwrap(),
        )
        .unwrap(),
    )
    .unwrap()
    .label("vehicle")
    .score(0.7);

    let detections = va::image_detection::segments_to_detections(&[segment], 1, "object");
    assert_eq!(detections.len(), 1);
    assert_eq!(detections[0].label, "vehicle");
    assert_eq!(
        detections[0].region,
        va::BoundingBox::new(3, 2, 4, 3).unwrap()
    );
}
