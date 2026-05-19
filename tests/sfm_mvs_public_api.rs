use std::fs;

use tempfile::tempdir;
use video_analysis as va;

#[test]
fn colmap_sfm_and_mvs_surfaces_share_workspace_models() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempdir()?;
    fs::write(
        temp.path().join("cameras.txt"),
        "1 PINHOLE 32 32 30 30 15 15\n",
    )?;
    fs::write(
        temp.path().join("images.txt"),
        "1 1 0 0 0 0 0 0 1 a.png\n15 15 1\n2 1 0 0 0 -1 0 0 1 b.png\n15 15 1\n",
    )?;
    fs::write(
        temp.path().join("points3D.txt"),
        "1 0 0 3 255 255 255 0.25 1 0 2 0\n",
    )?;

    let baseline = va::colmap_backend::load_colmap_text_baseline(temp.path())?;
    assert_eq!(baseline.report.camera_count, 1);
    assert_eq!(baseline.report.registered_image_count, 2);
    assert_eq!(baseline.report.sparse_point_count, 1);
    assert_eq!(baseline.report.track_length_histogram[&2], 1);

    let comparison =
        va::colmap_backend::compare_to_colmap_baseline(&baseline.sparse_reconstruction, &baseline)?;
    assert_eq!(comparison.camera_count_delta, 0);
    assert_eq!(comparison.registered_image_count_delta, 0);
    assert_eq!(comparison.sparse_point_count_delta, 0);

    let mvs_request = va::mvs::MvsRequest::new(baseline.sparse_reconstruction, baseline.view_set)?;
    let mut mvs_pipeline = va::mvs::MvsPipeline::new(va::mvs::SparsePointCloudDenseReconstructor);
    let dense = mvs_pipeline.run(&mvs_request)?;
    assert_eq!(dense.report.point_count, 1);

    let opencv_capabilities = va::opencv_backend::OpenCvBackendCapabilities::current();
    assert!(!opencv_capabilities.sparse_sfm);
    Ok(())
}

#[test]
fn rust_sfm_backend_exposes_shared_pipeline_contract() -> Result<(), Box<dyn std::error::Error>> {
    let intrinsics = va::radiance_fields::CameraIntrinsics::new(32, 32, 30.0, 30.0, 15.0, 15.0)?;
    let left = va::sfm::SfmInputImage::new(
        va::reconstruction::ImageId(1),
        va::reconstruction::CameraId(1),
        "a.png",
        intrinsics,
    )?
    .pose(va::radiance_fields::CameraPose::identity())?
    .features([va::reconstruction::BinaryFeature::new(
        va::reconstruction::Feature2d::new(va::radiance_fields::Vec2::new(15.0, 15.0))?,
        [0_u8],
    )?])?;
    let right = va::sfm::SfmInputImage::new(
        va::reconstruction::ImageId(2),
        va::reconstruction::CameraId(2),
        "b.png",
        intrinsics,
    )?
    .pose(va::radiance_fields::CameraPose::look_at(
        va::radiance_fields::Vec3::new(-1.0, 0.0, 0.0),
        va::radiance_fields::Vec3::new(0.0, 0.0, 3.0),
        va::radiance_fields::Vec3::new(0.0, 1.0, 0.0),
    )?)?
    .features([va::reconstruction::BinaryFeature::new(
        va::reconstruction::Feature2d::new(va::radiance_fields::Vec2::new(15.0, 15.0))?,
        [0_u8],
    )?])?;
    let request = va::sfm::SfmRequest::new([left, right])?;
    let mut pipeline =
        va::sfm::SfmPipeline::new(va::sfm_rust_backend::RustKnownPoseSfmBackend::default());
    let output = pipeline.run(&request)?;
    assert_eq!(output.report.backend, "rust-known-pose-sfm-backend");
    assert_eq!(output.report.registered_image_count, 2);
    Ok(())
}
