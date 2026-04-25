use tempfile::tempdir;
use video_analysis_gaussian_splatting::{
    GaussianSplat3d, GaussianSplatScene, ProjectionConfig, Quaternion, SphericalHarmonicsRgb,
    SplatRenderConfig,
};
use video_analysis_radiance_fields::{ColorRgb, Vec3};
use video_analysis_radiance_io::{
    write_gaussian_splat_ply, write_nerfstudio_transforms, NerfstudioFrame, NerfstudioTransforms,
};
use video_analysis_radiance_pipeline::{
    GaussianPreviewRequest, RadianceProject, RadianceProjectPaths, RadianceViewSource,
};

fn preview_request(source: RadianceViewSource) -> GaussianPreviewRequest {
    GaussianPreviewRequest {
        source,
        view_index: 0,
        projection: ProjectionConfig::default(),
        render: SplatRenderConfig::new(32, 24).unwrap(),
        min_opacity: None,
        downsample_stride: None,
    }
}

fn minimal_nerfstudio_transforms() -> NerfstudioTransforms {
    NerfstudioTransforms {
        camera_model: Some("PINHOLE".to_string()),
        fl_x: Some(50.0),
        fl_y: Some(50.0),
        cx: Some(16.0),
        cy: Some(12.0),
        w: Some(32),
        h: Some(24),
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

fn sample_scene() -> GaussianSplatScene {
    GaussianSplatScene {
        splats: vec![
            GaussianSplat3d {
                mean: Vec3::new(0.0, 0.0, 1.0),
                scale_log: Vec3::new(-1.2, -1.2, -1.2),
                rotation: Quaternion::IDENTITY,
                opacity_logit: 5.0,
                sh: SphericalHarmonicsRgb::dc(ColorRgb::WHITE),
            },
            GaussianSplat3d {
                mean: Vec3::new(0.2, 0.1, 1.1),
                scale_log: Vec3::new(-1.2, -1.2, -1.2),
                rotation: Quaternion::IDENTITY,
                opacity_logit: 5.0,
                sh: SphericalHarmonicsRgb::dc(ColorRgb::new(1.0, 0.0, 0.0)),
            },
        ],
    }
}

#[test]
fn renders_gaussian_preview_against_nerfstudio_view() {
    let dir = tempdir().unwrap();
    let transforms_path = dir.path().join("transforms.json");
    let splat_path = dir.path().join("splats.ply");
    write_nerfstudio_transforms(&transforms_path, &minimal_nerfstudio_transforms()).unwrap();
    write_gaussian_splat_ply(&splat_path, &sample_scene()).unwrap();

    let project = RadianceProject::from_paths(&RadianceProjectPaths {
        colmap_text_dir: None,
        nerfstudio_transforms_json: Some(transforms_path),
        gaussian_splat_ply: Some(splat_path),
    })
    .unwrap();

    let preview = project
        .render_gaussian_preview(&preview_request(RadianceViewSource::Nerfstudio))
        .unwrap();

    assert_eq!(
        preview.pixels.len(),
        (preview.width * preview.height) as usize
    );
    assert!(preview.pixels.iter().any(|pixel| pixel.alpha > 0.0));
}
