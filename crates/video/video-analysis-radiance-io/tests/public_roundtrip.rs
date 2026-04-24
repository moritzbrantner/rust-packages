use video_analysis_radiance_fields::{CameraModel, ColorRgb, Vec2, Vec3};
use video_analysis_radiance_io::{
    colmap_to_view_set, read_colmap_text_dir, read_nerfstudio_transforms, transforms_to_view_set,
    write_colmap_text_dir, write_nerfstudio_transforms, ColmapCamera, ColmapDataset, ColmapImage,
    ColmapPoint2d, ColmapPoint3d, ColmapTrackElement, NerfstudioFrame, NerfstudioTransforms,
};

fn minimal_colmap_dataset() -> ColmapDataset {
    ColmapDataset {
        cameras: vec![ColmapCamera {
            id: 1,
            model: CameraModel::Pinhole,
            raw_model: "PINHOLE".to_string(),
            width: 64,
            height: 48,
            params: vec![50.0, 50.0, 32.0, 24.0],
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
                [0.0, 0.0, 1.0, 0.0],
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

#[test]
fn colmap_and_nerfstudio_public_round_trips_convert_to_views() {
    let dir = tempfile::tempdir().unwrap();
    let colmap_dir = dir.path().join("colmap");
    let dataset = minimal_colmap_dataset();

    write_colmap_text_dir(&colmap_dir, &dataset).unwrap();
    let loaded = read_colmap_text_dir(&colmap_dir).unwrap();
    assert_eq!(loaded.cameras, dataset.cameras);
    assert_eq!(loaded.images.len(), 1);
    assert_eq!(loaded.points.len(), 1);
    assert_eq!(colmap_to_view_set(&loaded).unwrap().views.len(), 1);

    let transforms = minimal_nerfstudio_transforms();
    let transforms_path = dir.path().join("transforms.json");
    write_nerfstudio_transforms(&transforms_path, &transforms).unwrap();
    let loaded_transforms = read_nerfstudio_transforms(&transforms_path).unwrap();
    assert_eq!(loaded_transforms, transforms);
    assert_eq!(
        transforms_to_view_set(&loaded_transforms)
            .unwrap()
            .views
            .len(),
        1
    );
}
