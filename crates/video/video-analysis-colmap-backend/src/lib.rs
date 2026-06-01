#![doc = include_str!("../README.md")]

mod reconstruct_video;
mod scene;
pub mod surface;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use video_analysis_core::{DetectError, Result};
use video_analysis_radiance_fields::CameraViewSet;
use video_analysis_radiance_io::{
    colmap_to_sparse_reconstruction, colmap_to_view_set, read_colmap_text_dir, ColmapDataset,
};
use video_analysis_reconstruction::SparseReconstruction;
use video_analysis_sfm::{
    reconstruction_report, SfmBackend, SfmPipelineOutput, SfmRequest, SfmRunReport,
};

#[cfg(test)]
pub(crate) use reconstruct_video::find_executable;
#[cfg(not(target_family = "wasm"))]
pub use reconstruct_video::validate_reconstruct_video_request;
pub use reconstruct_video::{
    default_reconstruct_video_request, reconstruct_video_surface_operation, ReconstructVideoColmap,
    ReconstructVideoFrames, ReconstructVideoRequest, ReconstructVideoResult, ReconstructVideoVideo,
    COLMAP_TEST_VIDEO_PATH, COLMAP_TEST_VIDEO_URL, DEFAULT_RECONSTRUCT_OUTPUT_DIR,
};
pub use scene::{
    build_colmap_scene, ColmapScene, ColmapSceneBounds, ColmapSceneCamera, ColmapScenePoint,
    ColmapSceneSummary,
};
#[cfg(test)]
pub(crate) use scene::{build_colmap_scene_from_dataset, scene_summary_from_dataset};

fn invalid_argument(message: impl Into<String>) -> DetectError {
    DetectError::InvalidArgument(message.into())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Variants describing COLMAP model format.
pub enum ColmapModelFormat {
    /// COLMAP text model with cameras.txt, images.txt, and points3D.txt.
    Text,
    /// COLMAP binary model. Reserved for native parser support.
    Binary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Data type for COLMAP input.
pub struct ColmapInput {
    /// Filesystem path for this input.
    pub path: PathBuf,
    /// Model format.
    pub format: ColmapModelFormat,
}

impl ColmapInput {
    /// Creates a text model input.
    pub fn text_dir(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            format: ColmapModelFormat::Text,
        }
    }

    /// Creates a binary model input.
    pub fn binary_dir(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            format: ColmapModelFormat::Binary,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for COLMAP baseline.
pub struct ColmapBaseline {
    /// Raw COLMAP dataset.
    pub dataset: ColmapDataset,
    /// Converted camera view set.
    pub view_set: CameraViewSet,
    /// Converted sparse reconstruction.
    pub sparse_reconstruction: SparseReconstruction,
    /// Report for the converted sparse reconstruction.
    pub report: SfmRunReport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Data type for COLMAP parity comparison.
pub struct ColmapParityComparison {
    /// Difference between candidate and baseline camera counts.
    pub camera_count_delta: isize,
    /// Difference between candidate and baseline registered image counts.
    pub registered_image_count_delta: isize,
    /// Difference between candidate and baseline sparse point counts.
    pub sparse_point_count_delta: isize,
    /// Difference between candidate and baseline track-length histograms.
    pub track_length_delta: BTreeMap<usize, isize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Data type for COLMAP command specification.
pub struct ColmapCommandSpec {
    /// COLMAP binary path or executable name.
    pub executable: PathBuf,
    /// Command arguments.
    pub args: Vec<String>,
}

impl ColmapCommandSpec {
    /// Creates a new value.
    pub fn new(executable: impl Into<PathBuf>, args: impl Into<Vec<String>>) -> Self {
        Self {
            executable: executable.into(),
            args: args.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Data type for COLMAP text backend.
pub struct ColmapTextBackend {
    input: ColmapInput,
}

impl ColmapTextBackend {
    /// Creates a new text backend.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            input: ColmapInput::text_dir(path),
        }
    }

    /// Loads the baseline.
    pub fn load(&self) -> Result<ColmapBaseline> {
        load_colmap_baseline(&self.input)
    }

    /// Returns input.
    pub fn input(&self) -> &ColmapInput {
        &self.input
    }
}

impl SfmBackend for ColmapTextBackend {
    fn name(&self) -> &'static str {
        "colmap-text-backend"
    }

    fn reconstruct(&mut self, _request: &SfmRequest) -> Result<SfmPipelineOutput> {
        let baseline = self.load()?;
        Ok(SfmPipelineOutput {
            reconstruction: baseline.sparse_reconstruction,
            report: baseline.report,
            verified_pairs: Vec::new(),
            registered_images: Vec::new(),
        })
    }
}

/// Loads a COLMAP baseline.
pub fn load_colmap_baseline(input: &ColmapInput) -> Result<ColmapBaseline> {
    match input.format {
        ColmapModelFormat::Text => load_colmap_text_baseline(&input.path),
        ColmapModelFormat::Binary => Err(invalid_argument(
            "COLMAP binary models are reserved for native parser support; use text export today",
        )),
    }
}

/// Loads a COLMAP text baseline.
pub fn load_colmap_text_baseline(path: impl AsRef<Path>) -> Result<ColmapBaseline> {
    let dataset = read_colmap_text_dir(path).map_err(|err| invalid_argument(err.to_string()))?;
    let view_set = colmap_to_view_set(&dataset).map_err(|err| invalid_argument(err.to_string()))?;
    let sparse_reconstruction = colmap_to_sparse_reconstruction(&dataset)
        .map_err(|err| invalid_argument(err.to_string()))?;
    let report = reconstruction_report(
        "colmap-text-baseline",
        dataset.images.len(),
        &sparse_reconstruction,
    )?;
    Ok(ColmapBaseline {
        dataset,
        view_set,
        sparse_reconstruction,
        report,
    })
}

/// Compares a candidate sparse reconstruction to a COLMAP baseline.
pub fn compare_to_colmap_baseline(
    candidate: &SparseReconstruction,
    baseline: &ColmapBaseline,
) -> Result<ColmapParityComparison> {
    let candidate_report = reconstruction_report("candidate", candidate.images().len(), candidate)?;
    Ok(compare_reports(&candidate_report, &baseline.report))
}

/// Compares two SfM reports.
pub fn compare_reports(
    candidate: &SfmRunReport,
    baseline: &SfmRunReport,
) -> ColmapParityComparison {
    let mut track_length_delta = BTreeMap::new();
    for key in candidate
        .track_length_histogram
        .keys()
        .chain(baseline.track_length_histogram.keys())
    {
        let candidate_value = *candidate.track_length_histogram.get(key).unwrap_or(&0) as isize;
        let baseline_value = *baseline.track_length_histogram.get(key).unwrap_or(&0) as isize;
        track_length_delta.insert(*key, candidate_value - baseline_value);
    }
    ColmapParityComparison {
        camera_count_delta: candidate.camera_count as isize - baseline.camera_count as isize,
        registered_image_count_delta: candidate.registered_image_count as isize
            - baseline.registered_image_count as isize,
        sparse_point_count_delta: candidate.sparse_point_count as isize
            - baseline.sparse_point_count as isize,
        track_length_delta,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use runtime_core::DiagnosticSeverity;
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn loads_colmap_text_baseline_and_reports_counts() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("cameras.txt"),
            "1 PINHOLE 32 32 30 30 15 15\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("images.txt"),
            "1 1 0 0 0 0 0 0 1 a.png\n15 15 1\n2 1 0 0 0 -1 0 0 1 b.png\n15 15 1\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("points3D.txt"),
            "1 0 0 3 255 255 255 0.25 1 0 2 0\n",
        )
        .unwrap();

        let baseline = load_colmap_text_baseline(dir.path()).unwrap();
        assert_eq!(baseline.dataset.cameras.len(), 1);
        assert_eq!(baseline.report.registered_image_count, 2);
        assert_eq!(baseline.report.sparse_point_count, 1);
        assert_eq!(baseline.report.track_length_histogram[&2], 1);
    }

    #[test]
    fn reports_binary_models_as_explicitly_unsupported() {
        let error = load_colmap_baseline(&ColmapInput::binary_dir("sparse/0")).unwrap_err();
        assert!(error.to_string().contains("binary"));
    }

    #[cfg(not(target_family = "wasm"))]
    #[test]
    fn reconstruct_video_validation_rejects_missing_and_out_of_scope_video_path() {
        let mut request = default_reconstruct_video_request();
        request.video_path = PathBuf::from(
            "prototypes/web/video-analysis-web/public/samples/video/missing-test-video.mp4",
        );
        request.video_url = None;
        let missing = validate_reconstruct_video_request(request).unwrap_err();
        assert!(missing.contains("not readable"));

        let dir = tempdir().unwrap();
        let outside = dir.path().join("outside.mp4");
        fs::write(&outside, b"not a real video").unwrap();
        let mut request = default_reconstruct_video_request();
        request.video_path = outside;
        request.video_url = None;
        let out_of_scope = validate_reconstruct_video_request(request).unwrap_err();
        assert!(out_of_scope.contains("inside workspace"));
    }

    #[test]
    fn scene_extraction_from_colmap_text_fixture_returns_path_and_points() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("cameras.txt"),
            "1 PINHOLE 32 32 30 30 15 15\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("images.txt"),
            "1 1 0 0 0 0 0 0 1 frame_00001.jpg\n15 15 1\n2 1 0 0 0 -1 0 0 1 frame_00002.jpg\n15 15 1\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("points3D.txt"),
            "1 0 0 3 255 128 0 0.25 1 0 2 0\n",
        )
        .unwrap();

        let baseline = load_colmap_text_baseline(dir.path()).unwrap();
        let scene = build_colmap_scene(&baseline).unwrap();

        assert_eq!(scene.cameras.len(), 2);
        assert_eq!(scene.camera_path.len(), 2);
        assert_eq!(scene.points.len(), 1);
        assert_eq!(scene.points[0].position, [0.0, 0.0, 3.0]);
        assert_eq!(scene.points[0].color, [255, 128, 0]);
    }

    #[test]
    fn scene_summary_and_scene_support_simple_radial_colmap_text() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("cameras.txt"),
            "1 SIMPLE_RADIAL 32 32 30 15 15 0.1\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("images.txt"),
            "1 1 0 0 0 0 0 0 1 frame_00001.jpg\n15 15 1\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("points3D.txt"),
            "1 0 0 3 255 255 255 0.25 1 0\n",
        )
        .unwrap();

        assert!(load_colmap_text_baseline(dir.path()).is_err());
        let dataset = read_colmap_text_dir(dir.path()).unwrap();
        let summary = scene_summary_from_dataset(&dataset);
        let scene = build_colmap_scene_from_dataset(&dataset).unwrap();

        assert_eq!(summary.camera_count, 1);
        assert_eq!(summary.registered_image_count, 1);
        assert_eq!(summary.sparse_point_count, 1);
        assert_eq!(summary.track_length_histogram[&1], 1);
        assert_eq!(scene.cameras.len(), 1);
        assert_eq!(scene.points.len(), 1);
    }

    #[cfg(not(target_family = "wasm"))]
    #[test]
    #[ignore = "requires local test-video.mp4, ffmpeg, and COLMAP"]
    fn real_colmap_video_smoke_test_when_local_tools_are_available() {
        let video_path = PathBuf::from(COLMAP_TEST_VIDEO_PATH);
        if !video_path.exists()
            || find_executable(&["ffmpeg", "/usr/bin/ffmpeg"]).is_none()
            || find_executable(&["colmap", "/usr/local/bin/colmap"]).is_none()
        {
            eprintln!("skipping real COLMAP smoke test because video or tools are unavailable");
            return;
        }

        let response = reconstruct_video_surface_operation(serde_json::json!({
            "videoPath": video_path,
            "videoUrl": COLMAP_TEST_VIDEO_URL,
            "outputDir": DEFAULT_RECONSTRUCT_OUTPUT_DIR,
            "frameFps": 2,
            "maxFrames": 80,
            "imageWidth": 1280,
            "clean": false
        }))
        .unwrap();

        assert!(
            response
                .diagnostics
                .iter()
                .all(|diagnostic| diagnostic.severity != DiagnosticSeverity::Error),
            "unexpected diagnostics: {:?}",
            response.diagnostics
        );
        assert!(response.value["scene"]["points"].as_array().is_some());
    }
}
