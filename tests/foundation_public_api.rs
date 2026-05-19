use std::fs;

use comfyui_models::{
    ComfyModelKind, ComfyModelRoot, ExtraModelPathSection, ExtraModelPathsConfig,
};
use data_inversion_core::{Generated, InformationFidelity, InversionMethod, InversionTrace};
use dense_data::{BucketGrid, DenseDataset, DensePoint};
use finance_statistics::{historical_value_at_risk, max_drawdown, sharpe_ratio, simple_returns};
use graph_analysis_core::{minimum_spanning_tree, shortest_path, Graph};
use image_analysis_core::{mean_rgb, ImagePixelFormat, OwnedImage};
use image_analysis_processing::grayscale_image;
use image_analysis_synthesis::{solid_image, ImageSynthesisConfig, RgbColor};
use math_geometry_2d::{
    broad_phase_pairs_2d, BroadPhase2Config, BroadPhase2Strategy, NormalizedPoint2, RectU32, Size2u,
};
use math_linear::{F32Matrix, Kernel2d};
use math_signal_core::{BiquadDesign, SampleRate, WindowFunction};
use math_sparse_data::SparseVector;
use math_statistics::{PrincipalComponents, RunningCovariance};
use numbers_core::{quartiles, summarize_numbers};
use tempfile::tempdir;
use three_d_processing_core::{
    broad_phase_pairs_3d, centroid, voxel_downsample, Bounds3, BroadPhase3Config,
    BroadPhase3Strategy, Point3, SpatialCellSize3,
};
use three_d_processing_io::{read_mesh, write_obj_mesh};
use three_d_processing_mesh::{Mesh, Triangle};
use vector_analysis_core::{cosine_similarity, DenseVector, VectorMetric};
use vector_analysis_index::{
    assign_nearest_centroids, SearchConfig, VectorRecord, VectorSearchIndex,
};

#[test]
fn foundation_crates_support_basic_consumer_workflows() -> Result<(), Box<dyn std::error::Error>> {
    let trace = InversionTrace::new(
        "dense_summary",
        "preview_image",
        InformationFidelity::Heuristic,
    )
    .confidence(0.4)?
    .assumption("preview colors are synthetic")
    .note("palette", InversionMethod::Template, "solid fill preview");
    let generated = Generated::new(41_u32, trace).map(|value| value + 1);
    assert_eq!(generated.value, 42);

    let temp = tempdir()?;
    let checkpoint_dir = temp.path().join("models/checkpoints");
    fs::create_dir_all(&checkpoint_dir)?;
    fs::write(checkpoint_dir.join("demo.safetensors"), b"weights")?;

    let assets = ComfyModelRoot::new(temp.path()).scan()?;
    assert!(assets
        .iter()
        .any(|asset| asset.kind == ComfyModelKind::Checkpoint));

    let yaml = ExtraModelPathsConfig::new()
        .insert_section(
            "shared",
            ExtraModelPathSection::default_comfyui("/models").default_first(true),
        )
        .to_yaml_string();
    assert!(yaml.contains("checkpoints"));

    let dataset = DenseDataset::from_points([
        DensePoint::new([0.0, 0.0])?.named("a"),
        DensePoint::new([1.0, 1.0])?.named("b").valued(2.0)?,
    ])?;
    assert_eq!(dataset.len(), 2);
    assert_eq!(dataset.averages()?.count, 2);
    assert_eq!(dataset.summary()?.coordinate_stats[0].mean, Some(0.5));
    assert!(!dataset.buckets(&BucketGrid::uniform(2, 1.0)?)?.is_empty());

    let mut graph = Graph::undirected();
    graph.connect_weighted("a", "b", 2.0)?;
    graph.connect_weighted("b", "c", 1.0)?;
    graph.connect_weighted("a", "c", 5.0)?;
    assert_eq!(
        shortest_path(&graph, "a", "c")?.unwrap().nodes,
        vec!["a".to_string(), "b".to_string(), "c".to_string()]
    );
    assert_eq!(minimum_spanning_tree(&graph)?.total_weight, 3.0);

    let scalar_summary = summarize_numbers(&[1.0, 2.0, 3.0, f64::NAN]);
    assert_eq!(scalar_summary.finite_count, 3);
    assert_eq!(scalar_summary.mean, Some(2.0));
    assert_eq!(quartiles(&[1.0, 2.0, 3.0, 4.0])?.median, 2.5);

    let returns = simple_returns(&[100.0, 102.0, 99.0, 105.0])?;
    assert!(sharpe_ratio(&returns, 0.0, 252.0)?.is_finite());
    assert!(max_drawdown(&returns)?.depth >= 0.0);
    assert_eq!(historical_value_at_risk(&returns, 0.95)?.observations, 3);

    let rect = RectU32::new(2, 3, 4, 5)?;
    assert_eq!(rect.area()?, 20);
    let center = rect.center_f32().to_normalized(Size2u::new(12, 16)?)?;
    assert!(center.x > 0.0 && center.y > 0.0);
    assert_eq!(
        broad_phase_pairs_2d(
            &[rect, RectU32::new(3, 4, 4, 5)?],
            BroadPhase2Config {
                strategy: BroadPhase2Strategy::SweepAndPrune,
                ..BroadPhase2Config::default()
            }
        )?
        .len(),
        1
    );
    assert_eq!(
        NormalizedPoint2::new(0.5, 0.5)?
            .to_pixel_point(Size2u::new(10, 8)?)
            .x,
        5
    );

    let image = OwnedImage::new(2, 1, ImagePixelFormat::Rgb24, vec![255, 0, 0, 0, 255, 0], 6)?;
    let mean = mean_rgb(&image.as_view())?;
    assert!(mean.red > mean.blue);
    let grayscale = grayscale_image(&image.as_view())?;
    assert_eq!(grayscale.pixel_format, ImagePixelFormat::Gray8);
    assert_eq!(Kernel2d::sharpen_3x3().as_array_3x3()?[4], 5.0);

    let synthesized = solid_image(
        RgbColor::WHITE,
        ImageSynthesisConfig::new(2, 2, ImagePixelFormat::Rgb24)?,
    )?;
    assert_eq!(synthesized.value.width, 2);

    let vector_a = DenseVector::new([1.0, 0.0])?;
    let vector_b = DenseVector::new([0.0, 1.0])?;
    assert!(cosine_similarity(vector_a.as_slice(), vector_b.as_slice())?.abs() < 1.0e-6);

    let mut index = VectorSearchIndex::new();
    index.add(VectorRecord::new("x", vector_a.clone()))?;
    index.add(VectorRecord::new("y", vector_b.clone()))?;
    let results = index.search(
        &DenseVector::new([0.9, 0.1])?,
        SearchConfig {
            metric: VectorMetric::Cosine,
            ..SearchConfig::default()
        },
    )?;
    assert_eq!(results[0].id, "x");
    assert_eq!(
        assign_nearest_centroids(
            &[DenseVector::new([0.8, 0.2])?],
            &[vector_a, vector_b],
            VectorMetric::Euclidean,
        )?,
        vec![0]
    );

    let matrix = F32Matrix::from_rows([[1.0, 0.0], [0.0, 1.0]])?;
    assert_eq!(matrix.matmul(&matrix.as_view())?.shape().rows, 2);
    let covariance = RunningCovariance::from_matrix(&matrix.as_view())?.covariance_matrix()?;
    assert_eq!(covariance.matrix.shape().rows, 2);
    assert_eq!(
        PrincipalComponents::fit(&matrix.as_view(), 1)?
            .components()
            .shape()
            .rows,
        1
    );

    let windowed = WindowFunction::Hann.apply(&[1.0, 1.0, 1.0, 1.0]);
    assert!(windowed[0].abs() < 1.0e-6);
    BiquadDesign::LowPass.design(SampleRate::new(48_000)?, 1_000.0, 0.707)?;

    let sparse = SparseVector::from_dense(&[1.0, 0.0, 2.0])?.canonicalized()?;
    assert_eq!(sparse.to_dense(), vec![1.0, 0.0, 2.0]);

    let mesh = Mesh::new(
        [
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ],
        [Triangle::new(0, 1, 2)],
    )?;
    assert!(mesh.surface_area()? > 0.0);
    assert!(centroid(&mesh.vertices)?.is_some());
    assert_eq!(voxel_downsample(&mesh.vertices, 1.0)?.len(), 3);
    assert_eq!(
        broad_phase_pairs_3d(
            &[
                Bounds3::new(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 1.0, 1.0))?,
                Bounds3::new(Point3::new(0.5, 0.5, 0.5), Point3::new(2.0, 2.0, 2.0))?,
            ],
            BroadPhase3Config {
                strategy: BroadPhase3Strategy::SpatialHashGrid,
                cell_size: SpatialCellSize3::Fixed { size: 1.0 },
                ..BroadPhase3Config::default()
            }
        )?
        .len(),
        1
    );

    let mesh_path = temp.path().join("triangle.obj");
    write_obj_mesh(&mesh_path, &mesh)?;
    let loaded = read_mesh(&mesh_path)?;
    assert_eq!(loaded.triangles.len(), 1);

    Ok(())
}
