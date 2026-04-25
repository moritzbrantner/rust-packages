use std::fs::File;
use std::io::Write;

use tempfile::tempdir;
use video_analysis_gaussian_splatting::{
    GaussianSplat3d, GaussianSplatScene, Quaternion, SphericalHarmonicsRgb,
};
use video_analysis_radiance_fields::{CameraModel, ColorRgb, Vec2, Vec3};
use video_analysis_radiance_io::{
    write_colmap_text_dir, write_gaussian_splat_ply, write_nerfstudio_transforms, ColmapCamera,
    ColmapDataset, ColmapImage, ColmapPoint2d, ColmapPoint3d, ColmapTrackElement, NerfstudioFrame,
    NerfstudioTransforms,
};
use video_analysis_radiance_pipeline::{
    RadiancePipelineError, RadianceProject, RadianceProjectPaths, RadianceViewSource,
};

fn minimal_colmap_dataset(raw_model: &str, model: CameraModel) -> ColmapDataset {
    ColmapDataset {
        cameras: vec![ColmapCamera {
            id: 1,
            model,
            raw_model: raw_model.to_string(),
            width: 64,
            height: 48,
            params: match raw_model {
                "PINHOLE" => vec![50.0, 50.0, 32.0, 24.0],
                "SIMPLE_PINHOLE" => vec![50.0, 32.0, 24.0],
                "SIMPLE_RADIAL" => vec![50.0, 32.0, 24.0, 0.01],
                _ => vec![50.0, 50.0, 32.0, 24.0],
            },
        }],
        images: vec![ColmapImage {
            id: 1,
            qw: 1.0,
            qx: 0.0,
            qy: 0.0,
            qz: 0.0,
            tx: 0.0,
            ty: 0.0,
            tz: 0.0,
            camera_id: 1,
            name: "frame_0001.png".to_string(),
            points2d: vec![ColmapPoint2d {
                xy: Vec2::new(32.0, 24.0),
                point3d_id: Some(1),
            }],
        }],
        points: vec![ColmapPoint3d {
            id: 1,
            xyz: Vec3::new(0.0, 0.0, 1.0),
            color: ColorRgb::new(1.0, 0.0, 0.0),
            error: 0.1,
            track: vec![ColmapTrackElement {
                image_id: 1,
                point2d_index: 0,
            }],
        }],
    }
}

fn minimal_nerfstudio_transforms() -> NerfstudioTransforms {
    NerfstudioTransforms {
        camera_model: Some("PINHOLE".to_string()),
        fl_x: Some(50.0),
        fl_y: Some(50.0),
        cx: Some(32.0),
        cy: Some(24.0),
        w: Some(64),
        h: Some(48),
        frames: vec![NerfstudioFrame {
            file_path: "images/frame_0001.png".to_string(),
            transform_matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, -1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            fl_x: None,
            fl_y: None,
            cx: None,
            cy: None,
            w: None,
            h: None,
        }],
    }
}

fn minimal_splats() -> GaussianSplatScene {
    GaussianSplatScene {
        splats: vec![GaussianSplat3d {
            mean: Vec3::new(0.0, 0.0, 1.0),
            scale_log: Vec3::new(-1.5, -1.5, -1.5),
            rotation: Quaternion::IDENTITY,
            opacity_logit: 4.0,
            sh: SphericalHarmonicsRgb::dc(ColorRgb::WHITE),
        }],
    }
}

#[test]
fn loads_minimal_pinhole_colmap_dataset() {
    let dir = tempdir().unwrap();
    let colmap_dir = dir.path().join("colmap");
    write_colmap_text_dir(
        &colmap_dir,
        &minimal_colmap_dataset("PINHOLE", CameraModel::Pinhole),
    )
    .unwrap();

    let project = RadianceProject::from_paths(&RadianceProjectPaths {
        colmap_text_dir: Some(colmap_dir),
        nerfstudio_transforms_json: None,
        gaussian_splat_ply: None,
    })
    .unwrap();
    let summary = project.summary().unwrap();

    assert_eq!(summary.colmap_camera_count, 1);
    assert_eq!(summary.colmap_point_count, 1);
    assert_eq!(
        summary.available_view_sources,
        vec![RadianceViewSource::Colmap]
    );
}

#[test]
fn loads_minimal_nerfstudio_transforms() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("transforms.json");
    write_nerfstudio_transforms(&path, &minimal_nerfstudio_transforms()).unwrap();

    let project = RadianceProject::from_paths(&RadianceProjectPaths {
        colmap_text_dir: None,
        nerfstudio_transforms_json: Some(path),
        gaussian_splat_ply: None,
    })
    .unwrap();
    let summary = project.summary().unwrap();

    assert_eq!(summary.nerfstudio_frame_count, 1);
    assert_eq!(
        summary.available_view_sources,
        vec![RadianceViewSource::Nerfstudio]
    );
}

#[test]
fn loads_colmap_and_nerfstudio_together() {
    let dir = tempdir().unwrap();
    let colmap_dir = dir.path().join("colmap");
    let transforms_path = dir.path().join("transforms.json");

    write_colmap_text_dir(
        &colmap_dir,
        &minimal_colmap_dataset("PINHOLE", CameraModel::Pinhole),
    )
    .unwrap();
    write_nerfstudio_transforms(&transforms_path, &minimal_nerfstudio_transforms()).unwrap();

    let project = RadianceProject::from_paths(&RadianceProjectPaths {
        colmap_text_dir: Some(colmap_dir),
        nerfstudio_transforms_json: Some(transforms_path),
        gaussian_splat_ply: None,
    })
    .unwrap();
    let summary = project.summary().unwrap();

    assert_eq!(
        summary.available_view_sources,
        vec![RadianceViewSource::Colmap, RadianceViewSource::Nerfstudio]
    );
}

#[test]
fn colmap_with_simple_radial_is_rejected() {
    let dir = tempdir().unwrap();
    let colmap_dir = dir.path().join("colmap");
    write_colmap_text_dir(
        &colmap_dir,
        &minimal_colmap_dataset("SIMPLE_RADIAL", CameraModel::SimpleRadial),
    )
    .unwrap();

    let error = RadianceProject::from_paths(&RadianceProjectPaths {
        colmap_text_dir: Some(colmap_dir),
        nerfstudio_transforms_json: None,
        gaussian_splat_ply: None,
    })
    .unwrap_err();

    match error {
        RadiancePipelineError::UnsupportedColmapCameraModels(support) => {
            assert_eq!(support.len(), 1);
            assert_eq!(support[0].camera_id, 1);
            assert_eq!(support[0].raw_model, "SIMPLE_RADIAL");
        }
        other => panic!("expected UnsupportedColmapCameraModels, got {other:?}"),
    }
}

#[test]
fn malformed_or_missing_files_preserve_radiance_io_error() {
    let dir = tempdir().unwrap();
    let transforms_path = dir.path().join("bad.json");
    let mut file = File::create(&transforms_path).unwrap();
    file.write_all(b"{ bad json").unwrap();

    let error = RadianceProject::from_paths(&RadianceProjectPaths {
        colmap_text_dir: None,
        nerfstudio_transforms_json: Some(transforms_path),
        gaussian_splat_ply: None,
    })
    .unwrap_err();

    assert!(matches!(error, RadiancePipelineError::RadianceIo(_)));
}

#[test]
fn gaussian_only_project_summarizes_loaded_splats() {
    let dir = tempdir().unwrap();
    let splat_path = dir.path().join("splats.ply");
    write_gaussian_splat_ply(&splat_path, &minimal_splats()).unwrap();

    let project = RadianceProject::from_paths(&RadianceProjectPaths {
        colmap_text_dir: None,
        nerfstudio_transforms_json: None,
        gaussian_splat_ply: Some(splat_path),
    })
    .unwrap();
    let summary = project.summary().unwrap();

    assert_eq!(summary.gaussian_splat_count, 1);
    assert!(summary.available_view_sources.is_empty());
    assert!(summary.gaussian_bounds.is_some());
}
