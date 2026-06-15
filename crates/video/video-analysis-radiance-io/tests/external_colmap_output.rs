use std::path::PathBuf;

use video_analysis_radiance_io::{colmap_to_sparse_reconstruction, read_colmap_text_dir};

#[test]
#[ignore = "requires external COLMAP sparse text output"]
fn radiance_io_reads_external_colmap_output() {
    let Some(path) = colmap_sparse_text_dir() else {
        skip_or_panic(
            "radiance-io external smoke",
            "COLMAP_SPARSE_TEXT_DIR is missing",
        );
        return;
    };
    let dataset = read_colmap_text_dir(&path).expect("read COLMAP text output");
    assert!(!dataset.cameras.is_empty());
    assert!(!dataset.images.is_empty());
    assert!(!dataset.points.is_empty());
}

#[test]
#[ignore = "requires external COLMAP sparse text output"]
fn reconstruction_accepts_external_sparse_colmap_output() {
    let Some(path) = colmap_sparse_text_dir() else {
        skip_or_panic(
            "reconstruction external smoke",
            "COLMAP_SPARSE_TEXT_DIR is missing",
        );
        return;
    };
    let dataset = read_colmap_text_dir(&path).expect("read COLMAP text output");
    let reconstruction =
        colmap_to_sparse_reconstruction(&dataset).expect("convert COLMAP to sparse reconstruction");
    assert!(!reconstruction.cameras().is_empty());
    assert!(!reconstruction.images().is_empty());
    assert!(!reconstruction.points().is_empty());
}

fn colmap_sparse_text_dir() -> Option<PathBuf> {
    let path = std::env::var_os("COLMAP_SPARSE_TEXT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".external-test-tools/colmap-runs/test-video/sparse_txt"));
    path.join("cameras.txt").is_file().then_some(path)
}

fn skip_or_panic(test_name: &str, reason: &str) {
    if std::env::var_os("STRICT_EXTERNAL_RUNTIME_CHECKS").is_some() {
        panic!("{test_name} setup is incomplete: {reason}");
    }
    eprintln!("skipping {test_name} because {reason}");
}
