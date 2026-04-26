use video_analysis as va;

#[test]
fn facade_reexports_sam_defaults_for_detection_and_segmentation() {
    let image_spec = va::image_models::default_sam_model_spec();
    assert_eq!(image_spec.repo_id, "facebook/sam-vit-base");

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

#[test]
fn shared_geometry_and_kernels_flow_through_vision_packages() {
    let rect = va::geometry2d::RectU32::new(1, 1, 2, 2).unwrap();
    let image = va::image_core::OwnedImage::new(
        4,
        4,
        va::image_core::ImagePixelFormat::Rgb24,
        vec![0; 4 * 4 * 3],
        12,
    )
    .unwrap();
    let cropped = va::image_processing::crop_image_rect(&image.as_view(), rect).unwrap();
    let filtered = va::image_processing::convolve_3x3_kernel(
        &cropped.as_view(),
        &va::linear::Kernel2d::identity_3x3(),
        1.0,
        0.0,
    )
    .unwrap();
    assert_eq!(filtered.width, 2);
    let bbox = va::BoundingBox::try_from(rect).unwrap();
    let round_trip = va::geometry2d::RectU32::from(bbox);
    assert_eq!(round_trip, rect);
}
