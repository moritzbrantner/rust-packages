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
    let f64_matrix = va::linear::F64Matrix::try_from(&matrix).unwrap();
    let svd = f64_matrix
        .svd(va::linear::SvdOptions {
            compute_factors: true,
            ..va::linear::SvdOptions::default()
        })
        .unwrap();
    assert_eq!(svd.rank, 2);
    assert!(svd.reconstruction.relative_residual < 1.0e-10);

    let regression =
        va::stats::ordinary_least_squares(&matrix.as_view(), &[3.0, 5.0, 7.0]).unwrap();
    assert!((regression.coefficients[1] - 2.0).abs() < 1.0e-4);

    let changes =
        va::stats::changes(&[100.0, 102.0, 99.0], va::stats::ChangeMethod::Relative).unwrap();
    assert_eq!(changes.len(), 2);
    assert!(va::stats::tail_risk(&changes, 0.8)
        .unwrap()
        .conditional_value_at_risk
        .is_finite());
}

#[test]
fn analytical_math_crates_connect_sparse_linear_statistics_and_risk_metrics() {
    let coo = va::sparse::CooMatrix::new(
        4,
        2,
        vec![
            (0, 0, 1.0),
            (0, 1, 1.0),
            (1, 0, 1.0),
            (1, 1, 2.0),
            (2, 0, 1.0),
            (2, 1, 3.0),
            (3, 0, 1.0),
            (3, 1, 4.0),
        ],
    )
    .unwrap();
    let csr = coo.to_csr().unwrap();
    let summary = csr.summary().unwrap();
    assert_eq!(summary.rows, 4);
    assert_eq!(summary.cols, 2);
    assert_eq!(summary.nnz, 8);

    let dense = csr.to_dense_matrix().unwrap();
    let target = [3.0, 5.0, 7.0, 9.0];
    let least_squares = dense.least_squares(&target, 0.0).unwrap();
    assert_eq!(least_squares.coefficients.len(), 2);
    assert!(least_squares.residual_sum_squares < 1.0e-6);

    let diagnostics =
        va::stats::ordinary_least_squares_diagnostics(&dense.as_view(), &target, 0.0).unwrap();
    assert_eq!(diagnostics.degrees_of_freedom, 2);
    assert!(diagnostics.root_mean_squared_error.is_finite());

    let matrix =
        va::linear::F32Matrix::from_rows([[0.02, 0.01], [-0.01, 0.0], [0.03, 0.02], [0.01, -0.01]])
            .unwrap();
    let covariance =
        va::stats::covariance_matrix_from_rows(&matrix.as_view(), va::stats::VarianceMode::Sample)
            .unwrap();
    assert_eq!(covariance.shape().rows, 2);
    assert_eq!(covariance.shape().cols, 2);

    let drawdown = va::stats::max_drawdown(&[0.02, -0.03, 0.01, 0.04]).unwrap();
    assert!(drawdown.depth > 0.0);
}
