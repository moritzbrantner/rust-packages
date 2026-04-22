use video_analysis_radiance_io::{
    colmap_to_view_set, read_colmap_text_dir, read_nerfstudio_transforms, transforms_to_view_set,
    write_colmap_text_dir, write_nerfstudio_transforms,
};

#[test]
fn colmap_and_nerfstudio_public_round_trips_convert_to_views() {
    let dir = tempfile::tempdir().unwrap();
    let colmap_dir = dir.path().join("colmap");
    let dataset = video_analysis_test_support::minimal_colmap_dataset();

    write_colmap_text_dir(&colmap_dir, &dataset).unwrap();
    let loaded = read_colmap_text_dir(&colmap_dir).unwrap();
    assert_eq!(loaded.cameras, dataset.cameras);
    assert_eq!(loaded.images.len(), 1);
    assert_eq!(loaded.points.len(), 1);
    assert_eq!(colmap_to_view_set(&loaded).unwrap().views.len(), 1);

    let transforms = video_analysis_test_support::minimal_nerfstudio_transforms();
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
