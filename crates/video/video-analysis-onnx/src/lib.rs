#![doc = include_str!("../README.md")]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use image_analysis_core::ImageView;
#[cfg(feature = "onnxruntime")]
/// Re-exports the native ONNX runner API.
pub use image_analysis_onnx::NativeOnnxRunner;
/// Re-exports the image analysis ONNX API.
pub use image_analysis_onnx::{
    classification_options_from_bundle, decode_object_detections, image_to_tensor,
    options_from_bundle, preprocessing_from_config, validate_onnx_vision_bundle,
    validate_preprocessing, BoxFormat, ChannelOrder, OnnxImageClassificationOptions,
    OnnxImageClassificationOutput, OnnxImageClassificationRunner, OnnxImageClassifier,
    OnnxImagePreprocessing, OnnxImageTensor, OnnxObjectDetectionOptions, OnnxObjectDetectionOutput,
    OnnxObjectDetectionRunner, OnnxVisionBundleInfo, UnavailableOnnxImageClassificationRunner,
    UnavailableOnnxRunner,
};
use model_runtime::{ModelBundle, ModelTask as RuntimeModelTask};
use video_analysis_core::{DetectError, Result, VideoFrame};
use video_analysis_models::{
    ModelTask as VideoModelTask, PoseLiftModelBackend, PoseModelBackend, RawBoundingBox,
    RawPose2dPrediction, RawPose3dPrediction, RawPrediction, VisionModelBackend,
};
use video_analysis_posture::{KeypointSpace, Skeleton};

#[derive(Debug, Clone)]
/// Data type for ONNX object detector.
pub struct OnnxObjectDetector<R = UnavailableOnnxRunner> {
    inner: image_analysis_onnx::OnnxObjectDetector<R>,
}

#[cfg(not(feature = "onnxruntime"))]
impl OnnxObjectDetector<UnavailableOnnxRunner> {
    /// Builds this value from bundle.
    pub fn from_bundle(bundle: ModelBundle) -> Result<Self> {
        Ok(Self {
            inner: image_analysis_onnx::OnnxObjectDetector::from_bundle(bundle)?,
        })
    }
}

#[cfg(feature = "onnxruntime")]
impl OnnxObjectDetector<NativeOnnxRunner> {
    /// Builds this value from bundle.
    pub fn from_bundle(bundle: ModelBundle) -> Result<Self> {
        Ok(Self {
            inner: image_analysis_onnx::OnnxObjectDetector::from_bundle(bundle)?,
        })
    }
}

impl<R: OnnxObjectDetectionRunner> OnnxObjectDetector<R> {
    /// Builds this value from runner.
    pub fn from_runner(bundle: ModelBundle, runner: R) -> Result<Self> {
        Ok(Self {
            inner: image_analysis_onnx::OnnxObjectDetector::from_runner(bundle, runner)?,
        })
    }

    /// Returns this value with options.
    pub fn with_options(options: OnnxObjectDetectionOptions, runner: R) -> Result<Self> {
        Ok(Self {
            inner: image_analysis_onnx::OnnxObjectDetector::with_options(options, runner)?,
        })
    }

    /// Returns preprocessing.
    pub fn preprocessing(&self) -> &OnnxImagePreprocessing {
        self.inner.preprocessing()
    }

    /// Returns labels.
    pub fn labels(&self) -> &BTreeMap<i64, String> {
        self.inner.labels()
    }

    /// Returns predict frame with original size.
    pub fn predict_frame_with_original_size(
        &mut self,
        frame: &VideoFrame<'_>,
    ) -> Result<Vec<RawPrediction>> {
        let image = ImageView::from_video_frame(frame)?;
        let detections = self.inner.detect_image(&image)?;
        Ok(detections
            .into_iter()
            .map(|detection| RawPrediction {
                kind: Some("object".to_string()),
                label: Some(detection.label),
                text: None,
                score: detection.score,
                region: Some(RawBoundingBox::xywh(
                    detection.region.x as f32,
                    detection.region.y as f32,
                    detection.region.width as f32,
                    detection.region.height as f32,
                )),
                attributes: detection.attributes,
            })
            .collect())
    }
}

impl<R: OnnxObjectDetectionRunner> VisionModelBackend for OnnxObjectDetector<R> {
    fn task(&self) -> VideoModelTask {
        VideoModelTask::ObjectDetection
    }

    fn predict_frame(&mut self, frame: &VideoFrame<'_>) -> Result<Vec<RawPrediction>> {
        self.predict_frame_with_original_size(frame)
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for ONNX pose2d options.
pub struct OnnxPose2dOptions {
    /// The preprocessing value.
    pub preprocessing: OnnxImagePreprocessing,
    /// The skeleton value.
    pub skeleton: Skeleton,
    /// The output space value.
    pub output_space: KeypointSpace,
    /// The min pose score value.
    pub min_pose_score: f32,
    /// The min keypoint score value.
    pub min_keypoint_score: f32,
}

impl Default for OnnxPose2dOptions {
    fn default() -> Self {
        Self {
            preprocessing: OnnxImagePreprocessing::default(),
            skeleton: Skeleton::coco_17(),
            output_space: KeypointSpace::Normalized,
            min_pose_score: 0.0,
            min_keypoint_score: 0.0,
        }
    }
}

#[derive(Debug, Clone, Default)]
/// Data type for unavailable pose2d runner.
pub struct UnavailablePose2dRunner;

/// Trait for ONNX pose2d runner implementations.
pub trait OnnxPose2dRunner {
    /// Runs pose 2d.
    fn run_pose_2d(&mut self, input: &OnnxImageTensor) -> Result<Vec<RawPose2dPrediction>>;
}

impl OnnxPose2dRunner for UnavailablePose2dRunner {
    fn run_pose_2d(&mut self, _input: &OnnxImageTensor) -> Result<Vec<RawPose2dPrediction>> {
        Err(DetectError::Source(
            "native ONNX 2D pose execution is unavailable; construct with a runner or enable an executor"
                .to_string(),
        ))
    }
}

#[derive(Debug, Clone)]
/// Data type for ONNX pose2d estimator.
pub struct OnnxPose2dEstimator<R = UnavailablePose2dRunner> {
    options: OnnxPose2dOptions,
    runner: R,
}

impl OnnxPose2dEstimator<UnavailablePose2dRunner> {
    /// Builds this value from bundle.
    pub fn from_bundle(bundle: ModelBundle) -> Result<Self> {
        Ok(Self {
            options: pose_2d_options_from_bundle(&bundle)?,
            runner: UnavailablePose2dRunner,
        })
    }
}

impl<R: OnnxPose2dRunner> OnnxPose2dEstimator<R> {
    /// Builds this value from runner.
    pub fn from_runner(bundle: ModelBundle, runner: R) -> Result<Self> {
        Ok(Self {
            options: pose_2d_options_from_bundle(&bundle)?,
            runner,
        })
    }

    /// Returns this value with options.
    pub fn with_options(options: OnnxPose2dOptions, runner: R) -> Result<Self> {
        validate_preprocessing(&options.preprocessing)?;
        validate_threshold(options.min_pose_score)?;
        validate_threshold(options.min_keypoint_score)?;
        Ok(Self { options, runner })
    }

    /// Returns options.
    pub fn options(&self) -> &OnnxPose2dOptions {
        &self.options
    }
}

impl<R: OnnxPose2dRunner> PoseModelBackend for OnnxPose2dEstimator<R> {
    fn predict_frame(&mut self, frame: &VideoFrame<'_>) -> Result<Vec<RawPose2dPrediction>> {
        let input = preprocess_frame(frame, &self.options.preprocessing)?;
        let predictions = self.runner.run_pose_2d(&input)?;
        predictions
            .into_iter()
            .filter(|prediction| prediction.score.unwrap_or(1.0) >= self.options.min_pose_score)
            .map(|prediction| {
                convert_pose_2d_output(
                    prediction,
                    (frame.width, frame.height),
                    self.options.output_space,
                    self.options.min_keypoint_score,
                )
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for ONNX pose lifter options.
pub struct OnnxPoseLifterOptions {
    /// The min pose score value.
    pub min_pose_score: f32,
}

impl Default for OnnxPoseLifterOptions {
    fn default() -> Self {
        Self {
            min_pose_score: 0.0,
        }
    }
}

#[derive(Debug, Clone, Default)]
/// Data type for unavailable pose lift runner.
pub struct UnavailablePoseLiftRunner;

/// Trait for ONNX pose lift runner implementations.
pub trait OnnxPoseLiftRunner {
    /// Runs pose lift.
    fn run_pose_lift(
        &mut self,
        sequence: &[RawPose2dPrediction],
    ) -> Result<Vec<RawPose3dPrediction>>;
}

impl OnnxPoseLiftRunner for UnavailablePoseLiftRunner {
    fn run_pose_lift(
        &mut self,
        _sequence: &[RawPose2dPrediction],
    ) -> Result<Vec<RawPose3dPrediction>> {
        Err(DetectError::Source(
            "native ONNX pose lifting execution is unavailable; construct with a runner or enable an executor"
                .to_string(),
        ))
    }
}

#[derive(Debug, Clone)]
/// Data type for ONNX pose lifter.
pub struct OnnxPoseLifter<R = UnavailablePoseLiftRunner> {
    options: OnnxPoseLifterOptions,
    runner: R,
}

impl OnnxPoseLifter<UnavailablePoseLiftRunner> {
    /// Builds this value from bundle.
    pub fn from_bundle(bundle: ModelBundle) -> Result<Self> {
        validate_onnx_pose_bundle(&bundle, RuntimeModelTask::PoseLifting3d)?;
        Ok(Self {
            options: OnnxPoseLifterOptions::default(),
            runner: UnavailablePoseLiftRunner,
        })
    }
}

impl<R: OnnxPoseLiftRunner> OnnxPoseLifter<R> {
    /// Builds this value from runner.
    pub fn from_runner(bundle: ModelBundle, runner: R) -> Result<Self> {
        validate_onnx_pose_bundle(&bundle, RuntimeModelTask::PoseLifting3d)?;
        Ok(Self {
            options: OnnxPoseLifterOptions::default(),
            runner,
        })
    }

    /// Returns this value with options.
    pub fn with_options(options: OnnxPoseLifterOptions, runner: R) -> Result<Self> {
        validate_threshold(options.min_pose_score)?;
        Ok(Self { options, runner })
    }
}

impl<R: OnnxPoseLiftRunner> PoseLiftModelBackend for OnnxPoseLifter<R> {
    fn lift_poses(&mut self, sequence: &[RawPose2dPrediction]) -> Result<Vec<RawPose3dPrediction>> {
        let poses = self.runner.run_pose_lift(sequence)?;
        Ok(poses
            .into_iter()
            .filter(|pose| pose.score.unwrap_or(1.0) >= self.options.min_pose_score)
            .collect())
    }
}

/// Returns preprocess frame.
pub fn preprocess_frame(
    frame: &VideoFrame<'_>,
    options: &OnnxImagePreprocessing,
) -> Result<OnnxImageTensor> {
    let image = ImageView::from_video_frame(frame)?;
    image_analysis_onnx::preprocess_image(&image, options)
}

/// Validates ONNX pose bundle.
pub fn validate_onnx_pose_bundle(
    bundle: &ModelBundle,
    task: RuntimeModelTask,
) -> Result<OnnxVisionBundleInfo> {
    if bundle.manifest.task != task {
        return Err(DetectError::InvalidArgument(format!(
            "ONNX pose bundle task must be {:?}, got {:?}",
            task, bundle.manifest.task
        )));
    }
    let config_path = required_bundle_file(bundle, "config.json")?;
    let preprocessor_config_path = bundle.file_path("preprocessor_config.json");
    let onnx_files = bundle_files_with_extension(bundle, "onnx");
    let model_path = match onnx_files.as_slice() {
        [path] => path.clone(),
        [] => {
            return Err(DetectError::InvalidArgument(
                "ONNX pose bundle must contain exactly one `.onnx` model file".to_string(),
            ))
        }
        files => {
            return Err(DetectError::InvalidArgument(format!(
                "ONNX pose bundle must contain exactly one `.onnx` model file, found {}",
                files.len()
            )))
        }
    };
    let config = read_json(&config_path)?;
    Ok(OnnxVisionBundleInfo {
        config_path,
        preprocessor_config_path,
        model_path,
        labels: label_map_from_config(&config)?,
    })
}

/// Returns pose 2d options from bundle.
pub fn pose_2d_options_from_bundle(bundle: &ModelBundle) -> Result<OnnxPose2dOptions> {
    let info = validate_onnx_pose_bundle(bundle, RuntimeModelTask::PoseEstimation2d)?;
    let config = read_json(&info.config_path)?;
    let preprocessing = if let Some(path) = &info.preprocessor_config_path {
        preprocessing_from_config(&read_json(path)?)?
    } else {
        OnnxImagePreprocessing::default()
    };
    validate_preprocessing(&preprocessing)?;
    Ok(OnnxPose2dOptions {
        preprocessing,
        skeleton: skeleton_from_config(&config),
        output_space: KeypointSpace::Normalized,
        min_pose_score: 0.0,
        min_keypoint_score: 0.0,
    })
}

fn skeleton_from_config(config: &serde_json::Value) -> Skeleton {
    if let Some(names) = config
        .get("video_analysis")
        .and_then(|value| value.get("keypoint_names"))
        .or_else(|| config.get("keypoint_names"))
        .and_then(serde_json::Value::as_array)
    {
        let keypoints = names
            .iter()
            .filter_map(serde_json::Value::as_str)
            .map(str::to_string)
            .collect::<Vec<_>>();
        if !keypoints.is_empty() {
            return Skeleton::new(keypoints);
        }
    }
    Skeleton::coco_17()
}

fn convert_pose_2d_output(
    mut prediction: RawPose2dPrediction,
    dimensions: (u32, u32),
    output_space: KeypointSpace,
    min_keypoint_score: f32,
) -> Result<RawPose2dPrediction> {
    prediction
        .keypoints
        .retain(|keypoint| keypoint.score.unwrap_or(1.0) >= min_keypoint_score);
    if output_space == KeypointSpace::Pixel {
        for keypoint in &mut prediction.keypoints {
            keypoint.x *= dimensions.0 as f32;
            keypoint.y *= dimensions.1 as f32;
        }
        if let Some(region) = prediction.region.as_mut() {
            if region.normalized {
                if let (Some(xmin), Some(ymin), Some(xmax), Some(ymax)) =
                    (region.xmin, region.ymin, region.xmax, region.ymax)
                {
                    region.xmin = Some(xmin * dimensions.0 as f32);
                    region.ymin = Some(ymin * dimensions.1 as f32);
                    region.xmax = Some(xmax * dimensions.0 as f32);
                    region.ymax = Some(ymax * dimensions.1 as f32);
                    region.normalized = false;
                }
            }
        }
    }
    Ok(prediction)
}

fn validate_threshold(value: f32) -> Result<()> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(DetectError::InvalidArgument(
            "ONNX score threshold must be finite and in the range 0..=1".to_string(),
        ));
    }
    Ok(())
}

fn required_bundle_file(bundle: &ModelBundle, remote_path: &str) -> Result<PathBuf> {
    bundle.file_path(remote_path).ok_or_else(|| {
        DetectError::InvalidArgument(format!(
            "model bundle `{}` is missing required file `{remote_path}`",
            bundle.manifest.name
        ))
    })
}

fn bundle_files_with_extension(bundle: &ModelBundle, extension: &str) -> Vec<PathBuf> {
    bundle
        .manifest
        .files
        .values()
        .filter(|file| {
            Path::new(&file.remote_path)
                .extension()
                .and_then(|value| value.to_str())
                == Some(extension)
        })
        .map(|file| bundle.root.join(&file.local_path))
        .collect()
}

fn read_json(path: &Path) -> Result<serde_json::Value> {
    let data = std::fs::read(path)?;
    serde_json::from_slice(&data).map_err(|err| {
        DetectError::Source(format!("failed to parse JSON `{}`: {err}", path.display()))
    })
}

fn label_map_from_config(config: &serde_json::Value) -> Result<BTreeMap<i64, String>> {
    let Some(id2label) = config
        .get("id2label")
        .and_then(serde_json::Value::as_object)
    else {
        return Ok(BTreeMap::new());
    };
    let mut labels = BTreeMap::new();
    for (key, value) in id2label {
        let id = key.parse::<i64>().map_err(|err| {
            DetectError::InvalidArgument(format!("invalid id2label key `{key}`: {err}"))
        })?;
        let label = value.as_str().ok_or_else(|| {
            DetectError::InvalidArgument(format!("id2label value for `{key}` must be a string"))
        })?;
        labels.insert(id, label.to_string());
    }
    Ok(labels)
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::BTreeMap;

    use image_analysis_onnx::OnnxObjectDetectionOutput;
    use model_runtime::{ModelBundleFile, ModelBundleManifest};
    use num_rational::Rational64;
    use video_analysis_core::{FramePosition, PixelFormat};

    #[derive(Debug, Clone)]
    struct FakeDetectionRunner {
        output: OnnxObjectDetectionOutput,
    }

    impl OnnxObjectDetectionRunner for FakeDetectionRunner {
        fn run_object_detection(
            &mut self,
            _input: &OnnxImageTensor,
        ) -> Result<OnnxObjectDetectionOutput> {
            Ok(self.output.clone())
        }
    }

    #[derive(Debug, Clone)]
    struct FakePose2dRunner {
        output: Vec<RawPose2dPrediction>,
    }

    impl OnnxPose2dRunner for FakePose2dRunner {
        fn run_pose_2d(&mut self, _input: &OnnxImageTensor) -> Result<Vec<RawPose2dPrediction>> {
            Ok(self.output.clone())
        }
    }

    fn fake_bundle_with_task(
        root: &Path,
        task: RuntimeModelTask,
        files: &[(&str, &str)],
    ) -> ModelBundle {
        let files_root = root.join("files");
        std::fs::create_dir_all(&files_root).unwrap();
        let mut manifest_files = BTreeMap::new();
        for (remote_path, contents) in files {
            let path = files_root.join(remote_path);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(&path, contents).unwrap();
            let local_path = Path::new("files").join(remote_path);
            manifest_files.insert(
                (*remote_path).to_string(),
                ModelBundleFile {
                    remote_path: (*remote_path).to_string(),
                    local_path: local_path.to_string_lossy().replace('\\', "/"),
                    size_bytes: contents.len() as u64,
                },
            );
        }
        ModelBundle {
            root: root.to_path_buf(),
            manifest: ModelBundleManifest {
                schema_version: 1,
                name: "fake-vision".to_string(),
                repo_id: "fake/vision".to_string(),
                revision: "main".to_string(),
                task,
                files: manifest_files,
            },
        }
    }

    fn frame() -> VideoFrame<'static> {
        static DATA: [u8; 12] = [255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255];
        VideoFrame::packed(
            FramePosition::from_frame_index(0, Rational64::new(1, 1)),
            2,
            2,
            PixelFormat::Rgb24,
            &DATA,
            6,
        )
        .unwrap()
    }

    #[test]
    fn video_wrapper_predicts_frame_detections() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = fake_bundle_with_task(
            dir.path(),
            RuntimeModelTask::ObjectDetection,
            &[
                ("config.json", r#"{"id2label":{"0":"person"}}"#),
                ("onnx/model.onnx", "fake"),
            ],
        );
        let mut backend = OnnxObjectDetector::from_runner(
            bundle,
            FakeDetectionRunner {
                output: OnnxObjectDetectionOutput {
                    boxes: vec![[0.5, 0.5, 1.0, 1.0]],
                    class_ids: vec![0],
                    scores: vec![0.8],
                    box_format: BoxFormat::CxcywhNormalized,
                },
            },
        )
        .unwrap();
        let predictions = backend.predict_frame(&frame()).unwrap();
        assert_eq!(predictions[0].label.as_deref(), Some("person"));
    }

    #[test]
    fn pose_estimator_uses_shared_preprocessing() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = fake_bundle_with_task(
            dir.path(),
            RuntimeModelTask::PoseEstimation2d,
            &[
                ("config.json", r#"{"keypoint_names":["nose"]}"#),
                ("onnx/model.onnx", "fake"),
            ],
        );
        let mut estimator = OnnxPose2dEstimator::from_runner(
            bundle,
            FakePose2dRunner {
                output: vec![RawPose2dPrediction::default()],
            },
        )
        .unwrap();
        let poses = estimator.predict_frame(&frame()).unwrap();
        assert_eq!(poses.len(), 1);
    }
}
