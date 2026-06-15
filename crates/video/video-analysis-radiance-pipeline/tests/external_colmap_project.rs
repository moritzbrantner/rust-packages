use std::path::PathBuf;

use video_analysis_radiance_pipeline::{RadianceProject, RadianceProjectPaths, RadianceViewSource};

#[test]
#[ignore = "requires external COLMAP sparse text output"]
fn radiance_pipeline_loads_external_colmap_project() {
    let Some(path) = colmap_sparse_text_dir() else {
        if std::env::var_os("STRICT_EXTERNAL_RUNTIME_CHECKS").is_some() {
            panic!("radiance-pipeline external smoke setup is incomplete: COLMAP_SPARSE_TEXT_DIR is missing");
        }
        eprintln!(
            "skipping radiance-pipeline external smoke because COLMAP_SPARSE_TEXT_DIR is missing"
        );
        return;
    };
    let project = RadianceProject::from_paths(&RadianceProjectPaths {
        colmap_text_dir: Some(path),
        ..RadianceProjectPaths::default()
    })
    .expect("load radiance project from COLMAP text output");
    let summary = project.summary().expect("summarize radiance project");
    assert!(summary
        .available_view_sources
        .contains(&RadianceViewSource::Colmap));
    assert!(summary.colmap_camera_count > 0);
    assert!(summary.colmap_image_count > 0);
    assert!(summary.colmap_point_count > 0);
}

fn colmap_sparse_text_dir() -> Option<PathBuf> {
    let path = std::env::var_os("COLMAP_SPARSE_TEXT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".external-test-tools/colmap-runs/test-video/sparse_txt"));
    path.join("cameras.txt").is_file().then_some(path)
}
