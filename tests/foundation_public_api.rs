use std::fs;

use comfyui_models::{
    ComfyModelKind, ComfyModelRoot, ExtraModelPathSection, ExtraModelPathsConfig,
};
use data_inversion_core::{Generated, InformationFidelity, InversionMethod, InversionTrace};
use dense_data::{BucketGrid, DenseDataset, DensePoint};
use image_analysis_core::{mean_rgb, ImagePixelFormat, OwnedImage};
use image_analysis_processing::grayscale_image;
use image_analysis_synthesis::{solid_image, ImageSynthesisConfig, RgbColor};
use numbers_core::{quartiles, summarize_numbers};
use tempfile::tempdir;
use three_d_processing_core::{centroid, voxel_downsample, Point3};
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

    let scalar_summary = summarize_numbers(&[1.0, 2.0, 3.0, f64::NAN]);
    assert_eq!(scalar_summary.finite_count, 3);
    assert_eq!(scalar_summary.mean, Some(2.0));
    assert_eq!(quartiles(&[1.0, 2.0, 3.0, 4.0])?.median, 2.5);

    let image = OwnedImage::new(2, 1, ImagePixelFormat::Rgb24, vec![255, 0, 0, 0, 255, 0], 6)?;
    let mean = mean_rgb(&image.as_view())?;
    assert!(mean.red > mean.blue);
    let grayscale = grayscale_image(&image.as_view())?;
    assert_eq!(grayscale.pixel_format, ImagePixelFormat::Gray8);

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

    let mesh_path = temp.path().join("triangle.obj");
    write_obj_mesh(&mesh_path, &mesh)?;
    let loaded = read_mesh(&mesh_path)?;
    assert_eq!(loaded.triangles.len(), 1);

    Ok(())
}
