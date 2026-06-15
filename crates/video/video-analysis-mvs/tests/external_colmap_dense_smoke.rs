use std::path::PathBuf;

use video_analysis_mvs::{ColmapMvsBackend, ColmapMvsConfig, DenseReconstructor, MvsRequest};
use video_analysis_radiance_io::{
    colmap_to_sparse_reconstruction, colmap_to_view_set, read_colmap_text_dir,
};

#[test]
#[ignore = "requires COLMAP sparse reconstruction and dense MVS runtime"]
fn colmap_dense_mvs_smoke_when_configured() {
    let strict = std::env::var_os("STRICT_EXTERNAL_RUNTIME_CHECKS").is_some();
    let Some(text_dir) = required_path(
        "COLMAP_SPARSE_TEXT_DIR",
        ".external-test-tools/colmap-runs/test-video/sparse_txt",
        strict,
    ) else {
        eprintln!("skipping COLMAP dense MVS smoke because COLMAP_SPARSE_TEXT_DIR is missing");
        return;
    };
    let config = ColmapMvsConfig {
        image_dir: path_env(
            "COLMAP_MVS_IMAGE_DIR",
            ".external-test-tools/colmap-runs/test-video/frames",
        ),
        sparse_model_dir: path_env(
            "COLMAP_MVS_SPARSE_DIR",
            ".external-test-tools/colmap-runs/test-video/sparse/0",
        ),
        workspace_dir: path_env(
            "COLMAP_MVS_WORKSPACE_DIR",
            ".external-test-tools/colmap-runs/test-video/dense",
        ),
        fused_ply_path: path_env(
            "COLMAP_MVS_FUSED_PLY",
            ".external-test-tools/colmap-runs/test-video/dense/fused.ply",
        ),
        use_gpu: std::env::var("COLMAP_MVS_USE_GPU")
            .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
            .unwrap_or(false),
        ..ColmapMvsConfig::default()
    };

    let dataset = read_colmap_text_dir(text_dir).expect("read sparse COLMAP text model");
    let sparse_reconstruction =
        colmap_to_sparse_reconstruction(&dataset).expect("convert sparse reconstruction");
    let views = colmap_to_view_set(&dataset).expect("convert COLMAP views");
    let request = MvsRequest::new(sparse_reconstruction, views).expect("build MVS request");
    let mut backend = ColmapMvsBackend::new(config).expect("build COLMAP MVS backend");
    let output = backend
        .reconstruct_dense(&request)
        .expect("run COLMAP dense MVS");
    let artifact = backend
        .artifact_report()
        .expect("read COLMAP MVS artifacts");

    assert!(artifact.fused_ply_path.is_file());
    assert!(artifact.fused_ply_size_bytes > 0);
    assert!(artifact.fused_vertex_count > 0);
    assert!(artifact.loaded_point_count > 0);
    assert_eq!(output.report.backend, "colmap-mvs-backend");
    assert!(output
        .dense
        .point_cloud
        .points()
        .iter()
        .all(|point| point.is_finite()));
}

fn path_env(key: &str, default: &str) -> PathBuf {
    std::env::var_os(key)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(default))
}

fn required_path(key: &str, default: &str, strict: bool) -> Option<PathBuf> {
    let path = path_env(key, default);
    if path.exists() {
        return Some(path);
    }
    assert!(!strict, "{key} does not exist: {}", path.display());
    None
}
