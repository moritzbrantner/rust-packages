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

    let corpus = va::text_lexical::TfIdfCorpus::from_texts(
        ["rust cargo crates", "scene audio report"],
        va::text_lexical::CorpusOptions::default(),
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

#[test]
fn expanded_math_apis_interoperate_through_facade() {
    let path = va::maps_kernels::densify_line_flat(&[0.0, 0.0, 3.0, 0.0], 1.0).unwrap();
    let path_summary = va::maps_kernels::path_summary_flat(&path, false).unwrap();
    assert_eq!(path_summary.point_count, 4);

    let left = va::geometry2d::RectF32::new(0.0, 0.0, 2.0, 2.0).unwrap();
    let right = va::geometry2d::RectF32::new(1.0, 1.0, 2.0, 2.0).unwrap();
    assert!(left.iou(right).unwrap() > 0.0);

    let levels = va::signal::signal_levels(&[0.0, 0.5, -1.0]).unwrap();
    assert_eq!(levels.count, 3);

    let sparse = va::sparse::SparseVector::new(3, vec![0, 2], vec![1.0, 2.0]).unwrap();
    assert_eq!(sparse.top_k_by_abs(1).unwrap(), vec![(2, 2.0)]);

    let matrix = va::linear::F32Matrix::from_rows([[1.0, 1.0], [1.0, 2.0], [1.0, 3.0]]).unwrap();
    let qr = matrix.qr_decompose().unwrap();
    assert_eq!(qr.r.shape().rows, 2);

    let regression =
        va::stats::ordinary_least_squares(&matrix.as_view(), &[3.0, 5.0, 7.0]).unwrap();
    assert!((regression.coefficients[1] - 2.0).abs() < 1.0e-4);

    let portfolio =
        va::finance::portfolio_returns(&[vec![0.02, -0.01], vec![0.01, 0.0]], &[0.5, 0.5]).unwrap();
    assert_eq!(portfolio.len(), 2);
}
