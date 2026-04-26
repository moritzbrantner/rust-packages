use video_analysis as va;

#[test]
fn shared_math_layer_connects_geometry_image_audio_text_and_statistics() {
    let region = va::geometry2d::RectU32::new(0, 0, 2, 2).unwrap();
    let image = va::image_core::OwnedImage::new(
        2,
        2,
        va::image_core::ImagePixelFormat::Rgb24,
        vec![255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255],
        6,
    )
    .unwrap();
    let cropped = va::image_processing::crop_image_rect(&image.as_view(), region).unwrap();
    let filtered = va::image_processing::convolve_3x3_kernel(
        &cropped.as_view(),
        &va::linear::Kernel2d::identity_3x3(),
        1.0,
        0.0,
    )
    .unwrap();
    assert_eq!(filtered.width, 2);

    let windowed = va::signal::WindowFunction::Hann.apply(&[1.0, 1.0, 1.0, 1.0]);
    assert!(windowed[1] > 0.7);

    let corpus = va::text_corpus::TfIdfCorpus::from_texts(
        ["rust cargo crates", "scene audio report"],
        va::text_corpus::CorpusOptions::default(),
    )
    .unwrap();
    let sparse = corpus.sparse_term_matrix().unwrap();
    assert_eq!(sparse.matrix.rows(), 2);

    let dataset = va::dense::DenseDataset::from_points([
        va::dense::DensePoint::new([1.0, 0.0]).unwrap(),
        va::dense::DensePoint::new([0.0, 1.0]).unwrap(),
        va::dense::DensePoint::new([1.0, 1.0]).unwrap(),
    ])
    .unwrap();
    let covariance = dataset.covariance_matrix().unwrap();
    assert_eq!(covariance.matrix.shape().rows, 2);
}
