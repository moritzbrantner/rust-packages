#![doc = include_str!("../README.md")]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
#[cfg(feature = "onnxruntime")]
use std::sync::Mutex;

use image_analysis_core::{ImageView, OwnedImage};
use image_analysis_processing::resize_nearest;
use serde_json::Value;
use video_analysis_core::{DetectError, Result, VideoFrame};
use video_analysis_models::{
    ModelBundle, ModelTask, PoseLiftModelBackend, PoseModelBackend, RawBoundingBox,
    RawPose2dPrediction, RawPose3dPrediction, RawPrediction, VisionModelBackend,
};
use video_analysis_posture::{KeypointSpace, Skeleton};

#[derive(Debug, Clone, PartialEq)]
pub struct OnnxVisionBundleInfo {
    pub config_path: PathBuf,
    pub preprocessor_config_path: Option<PathBuf>,
    pub model_path: PathBuf,
    pub labels: BTreeMap<i64, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelOrder {
    Rgb,
    Bgr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OnnxImagePreprocessing {
    pub input_width: u32,
    pub input_height: u32,
    pub rescale_factor: f32,
    pub mean: [f32; 3],
    pub std: [f32; 3],
    pub channel_order: ChannelOrder,
}

impl Default for OnnxImagePreprocessing {
    fn default() -> Self {
        Self {
            input_width: 640,
            input_height: 640,
            rescale_factor: 1.0 / 255.0,
            mean: [0.0, 0.0, 0.0],
            std: [1.0, 1.0, 1.0],
            channel_order: ChannelOrder::Rgb,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OnnxObjectDetectionOptions {
    pub preprocessing: OnnxImagePreprocessing,
    pub score_threshold: f32,
    pub labels: BTreeMap<i64, String>,
    pub box_format: BoxFormat,
}

impl Default for OnnxObjectDetectionOptions {
    fn default() -> Self {
        Self {
            preprocessing: OnnxImagePreprocessing::default(),
            score_threshold: 0.0,
            labels: BTreeMap::new(),
            box_format: BoxFormat::CxcywhNormalized,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxFormat {
    XyxyAbsolute,
    XyxyNormalized,
    CxcywhAbsolute,
    CxcywhNormalized,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OnnxObjectDetectionOutput {
    pub boxes: Vec<[f32; 4]>,
    pub class_ids: Vec<i64>,
    pub scores: Vec<f32>,
    pub box_format: BoxFormat,
}

pub trait OnnxObjectDetectionRunner {
    fn run_object_detection(
        &mut self,
        input: &OnnxImageTensor,
    ) -> Result<OnnxObjectDetectionOutput>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct OnnxImageTensor {
    pub channels: usize,
    pub width: u32,
    pub height: u32,
    pub values: Vec<f32>,
}

#[derive(Debug, Clone, Default)]
pub struct UnavailableOnnxRunner;

impl OnnxObjectDetectionRunner for UnavailableOnnxRunner {
    fn run_object_detection(
        &mut self,
        _input: &OnnxImageTensor,
    ) -> Result<OnnxObjectDetectionOutput> {
        Err(DetectError::Source(
            "native ONNX vision execution is unavailable; construct with a runner or enable an executor"
                .to_string(),
        ))
    }
}

#[cfg(feature = "onnxruntime")]
#[derive(Debug)]
pub struct NativeOnnxRunner {
    session: Mutex<ort::session::Session>,
}

#[cfg(feature = "onnxruntime")]
impl NativeOnnxRunner {
    pub fn new(model_path: impl Into<PathBuf>) -> Result<Self> {
        let model_path = model_path.into();
        if !model_path.is_file() {
            return Err(DetectError::InvalidArgument(format!(
                "ONNX model file `{}` does not exist",
                model_path.display()
            )));
        }
        let session = ort::session::Session::builder()
            .map_err(ort_error)?
            .commit_from_file(&model_path)
            .map_err(ort_error)?;
        Ok(Self {
            session: Mutex::new(session),
        })
    }
}

#[cfg(feature = "onnxruntime")]
impl OnnxObjectDetectionRunner for NativeOnnxRunner {
    fn run_object_detection(
        &mut self,
        input: &OnnxImageTensor,
    ) -> Result<OnnxObjectDetectionOutput> {
        use std::borrow::Cow;

        use ort::session::SessionInputValue;
        use ort::value::Tensor;

        let mut session = self
            .session
            .lock()
            .map_err(|_| DetectError::Source("ONNX session mutex was poisoned".to_string()))?;
        let input_name = session
            .inputs()
            .first()
            .map(|input| input.name().to_string())
            .ok_or_else(|| {
                DetectError::InvalidArgument("ONNX vision model exposes no inputs".to_string())
            })?;
        let shape = vec![
            1_i64,
            input.channels as i64,
            input.height as i64,
            input.width as i64,
        ];
        let inputs = vec![(
            Cow::from(input_name),
            Tensor::<f32>::from_array((shape, input.values.clone()))
                .map_err(ort_error)?
                .into(),
        ) as (Cow<'_, str>, SessionInputValue<'_>)];
        let outputs = session.run(inputs).map_err(ort_error)?;
        if outputs.len() < 2 {
            return Err(DetectError::InvalidArgument(
                "ONNX object detection model must return logits and boxes".to_string(),
            ));
        }
        let first = extract_f32_output(&outputs[0])?;
        let second = extract_f32_output(&outputs[1])?;
        decode_detr_like_outputs(first, second)
    }
}

#[derive(Debug, Clone)]
pub struct OnnxObjectDetector<R = UnavailableOnnxRunner> {
    preprocessing: OnnxImagePreprocessing,
    score_threshold: f32,
    labels: BTreeMap<i64, String>,
    box_format: BoxFormat,
    runner: R,
}

#[cfg(not(feature = "onnxruntime"))]
impl OnnxObjectDetector<UnavailableOnnxRunner> {
    pub fn from_bundle(bundle: ModelBundle) -> Result<Self> {
        let options = options_from_bundle(&bundle)?;
        Ok(Self {
            preprocessing: options.preprocessing,
            score_threshold: options.score_threshold,
            labels: options.labels,
            box_format: options.box_format,
            runner: UnavailableOnnxRunner,
        })
    }
}

#[cfg(feature = "onnxruntime")]
impl OnnxObjectDetector<NativeOnnxRunner> {
    pub fn from_bundle(bundle: ModelBundle) -> Result<Self> {
        let info = validate_onnx_vision_bundle(&bundle)?;
        let options = options_from_bundle(&bundle)?;
        Ok(Self {
            preprocessing: options.preprocessing,
            score_threshold: options.score_threshold,
            labels: options.labels,
            box_format: options.box_format,
            runner: NativeOnnxRunner::new(info.model_path)?,
        })
    }
}

impl<R: OnnxObjectDetectionRunner> OnnxObjectDetector<R> {
    pub fn from_runner(bundle: ModelBundle, runner: R) -> Result<Self> {
        let options = options_from_bundle(&bundle)?;
        Ok(Self {
            preprocessing: options.preprocessing,
            score_threshold: options.score_threshold,
            labels: options.labels,
            box_format: options.box_format,
            runner,
        })
    }

    pub fn with_options(options: OnnxObjectDetectionOptions, runner: R) -> Result<Self> {
        validate_preprocessing(&options.preprocessing)?;
        validate_threshold(options.score_threshold)?;
        Ok(Self {
            preprocessing: options.preprocessing,
            score_threshold: options.score_threshold,
            labels: options.labels,
            box_format: options.box_format,
            runner,
        })
    }

    pub fn preprocessing(&self) -> &OnnxImagePreprocessing {
        &self.preprocessing
    }

    pub fn labels(&self) -> &BTreeMap<i64, String> {
        &self.labels
    }

    pub fn predict_frame_with_original_size(
        &mut self,
        frame: &VideoFrame<'_>,
    ) -> Result<Vec<RawPrediction>> {
        let input = preprocess_frame(frame, &self.preprocessing)?;
        let mut output = self.runner.run_object_detection(&input)?;
        if output.box_format != self.box_format {
            output.box_format = self.box_format;
        }
        Ok(decode_object_detections(
            &output,
            &self.labels,
            self.score_threshold,
            (frame.width, frame.height),
        ))
    }
}

impl<R: OnnxObjectDetectionRunner> VisionModelBackend for OnnxObjectDetector<R> {
    fn task(&self) -> ModelTask {
        ModelTask::ObjectDetection
    }

    fn predict_frame(&mut self, frame: &VideoFrame<'_>) -> Result<Vec<RawPrediction>> {
        self.predict_frame_with_original_size(frame)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OnnxPose2dOptions {
    pub preprocessing: OnnxImagePreprocessing,
    pub skeleton: Skeleton,
    pub output_space: KeypointSpace,
    pub min_pose_score: f32,
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
pub struct UnavailablePose2dRunner;

pub trait OnnxPose2dRunner {
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
pub struct OnnxPose2dEstimator<R = UnavailablePose2dRunner> {
    options: OnnxPose2dOptions,
    runner: R,
}

impl OnnxPose2dEstimator<UnavailablePose2dRunner> {
    pub fn from_bundle(bundle: ModelBundle) -> Result<Self> {
        Ok(Self {
            options: pose_2d_options_from_bundle(&bundle)?,
            runner: UnavailablePose2dRunner,
        })
    }
}

impl<R: OnnxPose2dRunner> OnnxPose2dEstimator<R> {
    pub fn from_runner(bundle: ModelBundle, runner: R) -> Result<Self> {
        Ok(Self {
            options: pose_2d_options_from_bundle(&bundle)?,
            runner,
        })
    }

    pub fn with_options(options: OnnxPose2dOptions, runner: R) -> Result<Self> {
        validate_preprocessing(&options.preprocessing)?;
        validate_threshold(options.min_pose_score)?;
        validate_threshold(options.min_keypoint_score)?;
        Ok(Self { options, runner })
    }

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
pub struct OnnxPoseLifterOptions {
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
pub struct UnavailablePoseLiftRunner;

pub trait OnnxPoseLiftRunner {
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
pub struct OnnxPoseLifter<R = UnavailablePoseLiftRunner> {
    options: OnnxPoseLifterOptions,
    runner: R,
}

impl OnnxPoseLifter<UnavailablePoseLiftRunner> {
    pub fn from_bundle(bundle: ModelBundle) -> Result<Self> {
        validate_onnx_pose_bundle(&bundle, ModelTask::PoseLifting3d)?;
        Ok(Self {
            options: OnnxPoseLifterOptions::default(),
            runner: UnavailablePoseLiftRunner,
        })
    }
}

impl<R: OnnxPoseLiftRunner> OnnxPoseLifter<R> {
    pub fn from_runner(bundle: ModelBundle, runner: R) -> Result<Self> {
        validate_onnx_pose_bundle(&bundle, ModelTask::PoseLifting3d)?;
        Ok(Self {
            options: OnnxPoseLifterOptions::default(),
            runner,
        })
    }

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

pub fn validate_onnx_pose_bundle(
    bundle: &ModelBundle,
    task: ModelTask,
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

pub fn pose_2d_options_from_bundle(bundle: &ModelBundle) -> Result<OnnxPose2dOptions> {
    let info = validate_onnx_pose_bundle(bundle, ModelTask::PoseEstimation2d)?;
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

fn skeleton_from_config(config: &Value) -> Skeleton {
    if let Some(names) = config
        .get("video_analysis")
        .and_then(|value| value.get("keypoint_names"))
        .or_else(|| config.get("keypoint_names"))
        .and_then(Value::as_array)
    {
        let keypoints = names
            .iter()
            .filter_map(Value::as_str)
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

pub fn validate_onnx_vision_bundle(bundle: &ModelBundle) -> Result<OnnxVisionBundleInfo> {
    if bundle.manifest.task != ModelTask::ObjectDetection {
        return Err(DetectError::InvalidArgument(format!(
            "ONNX vision bundle task must be object_detection, got {:?}",
            bundle.manifest.task
        )));
    }
    let config_path = required_bundle_file(bundle, "config.json")?;
    let preprocessor_config_path = bundle.file_path("preprocessor_config.json");
    let onnx_files = bundle_files_with_extension(bundle, "onnx");
    let model_path = match onnx_files.as_slice() {
        [path] => path.clone(),
        [] => {
            return Err(DetectError::InvalidArgument(
                "ONNX vision bundle must contain exactly one `.onnx` model file".to_string(),
            ))
        }
        files => {
            return Err(DetectError::InvalidArgument(format!(
                "ONNX vision bundle must contain exactly one `.onnx` model file, found {}",
                files.len()
            )))
        }
    };
    let config = read_json(&config_path)?;
    let labels = label_map_from_config(&config)?;
    Ok(OnnxVisionBundleInfo {
        config_path,
        preprocessor_config_path,
        model_path,
        labels,
    })
}

pub fn options_from_bundle(bundle: &ModelBundle) -> Result<OnnxObjectDetectionOptions> {
    let info = validate_onnx_vision_bundle(bundle)?;
    let config = read_json(&info.config_path)?;
    let preprocessor = if let Some(path) = &info.preprocessor_config_path {
        preprocessing_from_config(&read_json(path)?)?
    } else {
        OnnxImagePreprocessing::default()
    };
    validate_preprocessing(&preprocessor)?;
    let box_format = box_format_from_config(&config);
    Ok(OnnxObjectDetectionOptions {
        preprocessing: preprocessor,
        score_threshold: 0.0,
        labels: info.labels,
        box_format,
    })
}

pub fn preprocess_frame(
    frame: &VideoFrame<'_>,
    options: &OnnxImagePreprocessing,
) -> Result<OnnxImageTensor> {
    validate_preprocessing(options)?;
    let image = ImageView::from_video_frame(frame)?;
    let resized = if image.width == options.input_width && image.height == options.input_height {
        OwnedImage::from_video_frame(frame)?
    } else {
        resize_nearest(&image, options.input_width, options.input_height)?
    };
    image_to_tensor(&resized.as_view(), options)
}

pub fn image_to_tensor(
    image: &ImageView<'_>,
    options: &OnnxImagePreprocessing,
) -> Result<OnnxImageTensor> {
    validate_preprocessing(options)?;
    image.validate()?;
    let width = image.width;
    let height = image.height;
    let pixels = width as usize * height as usize;
    let mut values = vec![0.0_f32; 3 * pixels];
    for y in 0..height {
        for x in 0..width {
            let rgb = image.pixel_rgb(x, y);
            let channels = match options.channel_order {
                ChannelOrder::Rgb => [rgb[0], rgb[1], rgb[2]],
                ChannelOrder::Bgr => [rgb[2], rgb[1], rgb[0]],
            };
            let pixel_index = y as usize * width as usize + x as usize;
            for channel in 0..3 {
                let value = channels[channel] as f32 * options.rescale_factor;
                values[channel * pixels + pixel_index] =
                    (value - options.mean[channel]) / options.std[channel];
            }
        }
    }
    Ok(OnnxImageTensor {
        channels: 3,
        width,
        height,
        values,
    })
}

pub fn decode_object_detections(
    output: &OnnxObjectDetectionOutput,
    labels: &BTreeMap<i64, String>,
    score_threshold: f32,
    original_size: (u32, u32),
) -> Vec<RawPrediction> {
    output
        .boxes
        .iter()
        .zip(output.class_ids.iter())
        .zip(output.scores.iter())
        .filter_map(|((bbox, class_id), score)| {
            if !score.is_finite() || *score < score_threshold {
                return None;
            }
            let region = raw_box(*bbox, output.box_format, original_size)?;
            let label = labels
                .get(class_id)
                .cloned()
                .unwrap_or_else(|| format!("LABEL_{class_id}"));
            Some(RawPrediction::object(label, *score, region))
        })
        .collect()
}

fn raw_box(bbox: [f32; 4], format: BoxFormat, original_size: (u32, u32)) -> Option<RawBoundingBox> {
    if bbox.iter().any(|value| !value.is_finite()) {
        return None;
    }
    let (width, height) = (original_size.0 as f32, original_size.1 as f32);
    let (xmin, ymin, xmax, ymax, normalized) = match format {
        BoxFormat::XyxyAbsolute => (bbox[0], bbox[1], bbox[2], bbox[3], false),
        BoxFormat::XyxyNormalized => (bbox[0], bbox[1], bbox[2], bbox[3], true),
        BoxFormat::CxcywhAbsolute => {
            let half_width = bbox[2] / 2.0;
            let half_height = bbox[3] / 2.0;
            (
                bbox[0] - half_width,
                bbox[1] - half_height,
                bbox[0] + half_width,
                bbox[1] + half_height,
                false,
            )
        }
        BoxFormat::CxcywhNormalized => {
            let half_width = bbox[2] / 2.0;
            let half_height = bbox[3] / 2.0;
            (
                bbox[0] - half_width,
                bbox[1] - half_height,
                bbox[0] + half_width,
                bbox[1] + half_height,
                true,
            )
        }
    };
    if xmax <= xmin || ymax <= ymin {
        return None;
    }
    if normalized {
        Some(RawBoundingBox {
            xmin: Some(xmin.clamp(0.0, 1.0)),
            ymin: Some(ymin.clamp(0.0, 1.0)),
            xmax: Some(xmax.clamp(0.0, 1.0)),
            ymax: Some(ymax.clamp(0.0, 1.0)),
            normalized: true,
            ..RawBoundingBox::default()
        })
    } else {
        Some(RawBoundingBox::xyxy(
            xmin.clamp(0.0, width),
            ymin.clamp(0.0, height),
            xmax.clamp(0.0, width),
            ymax.clamp(0.0, height),
        ))
    }
}

#[cfg(feature = "onnxruntime")]
#[derive(Debug, Clone, PartialEq)]
struct F32Output {
    shape: Vec<usize>,
    values: Vec<f32>,
}

#[cfg(feature = "onnxruntime")]
fn extract_f32_output(value: &ort::value::DynValue) -> Result<F32Output> {
    let (shape, values) = value.try_extract_tensor::<f32>().map_err(ort_error)?;
    let shape = shape
        .iter()
        .map(|dim| {
            usize::try_from(*dim).map_err(|_| {
                DetectError::InvalidArgument(
                    "ONNX output shape contains a negative dimension".to_string(),
                )
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(F32Output {
        shape,
        values: values.to_vec(),
    })
}

#[cfg(feature = "onnxruntime")]
fn decode_detr_like_outputs(
    first: F32Output,
    second: F32Output,
) -> Result<OnnxObjectDetectionOutput> {
    let (boxes, logits) = match (is_box_output(&first), is_box_output(&second)) {
        (true, false) => (first, second),
        (false, true) => (second, first),
        _ => {
            return Err(DetectError::InvalidArgument(
                "ONNX object detection outputs must include one box tensor ending in 4 values and one logits tensor".to_string(),
            ))
        }
    };
    let query_count = output_query_count(&boxes.shape)?;
    let class_count = output_last_dim(&logits.shape)?;
    if output_query_count(&logits.shape)? != query_count {
        return Err(DetectError::InvalidArgument(
            "ONNX logits and boxes output query counts differ".to_string(),
        ));
    }
    if boxes.values.len() != query_count * 4 || logits.values.len() != query_count * class_count {
        return Err(DetectError::InvalidArgument(
            "ONNX output tensor shapes do not match their values".to_string(),
        ));
    }
    if class_count < 2 {
        return Err(DetectError::InvalidArgument(
            "ONNX object detection logits must include at least one class plus background"
                .to_string(),
        ));
    }

    let mut decoded_boxes = Vec::with_capacity(query_count);
    let mut class_ids = Vec::with_capacity(query_count);
    let mut scores = Vec::with_capacity(query_count);
    for query in 0..query_count {
        decoded_boxes.push([
            boxes.values[query * 4],
            boxes.values[query * 4 + 1],
            boxes.values[query * 4 + 2],
            boxes.values[query * 4 + 3],
        ]);
        let start = query * class_count;
        let probabilities = softmax(&logits.values[start..start + class_count]);
        let (class_index, score) = probabilities[..class_count - 1]
            .iter()
            .copied()
            .enumerate()
            .max_by(|left, right| left.1.total_cmp(&right.1))
            .ok_or_else(|| {
                DetectError::InvalidArgument(
                    "ONNX logits contain no foreground classes".to_string(),
                )
            })?;
        class_ids.push(class_index as i64);
        scores.push(score);
    }

    Ok(OnnxObjectDetectionOutput {
        boxes: decoded_boxes,
        class_ids,
        scores,
        box_format: BoxFormat::CxcywhNormalized,
    })
}

#[cfg(feature = "onnxruntime")]
fn is_box_output(output: &F32Output) -> bool {
    matches!(output_last_dim(&output.shape), Ok(4))
}

#[cfg(feature = "onnxruntime")]
fn output_query_count(shape: &[usize]) -> Result<usize> {
    match shape {
        [queries, _] => Ok(*queries),
        [1, queries, _] => Ok(*queries),
        _ => Err(DetectError::InvalidArgument(format!(
            "unsupported ONNX output shape `{shape:?}`"
        ))),
    }
}

#[cfg(feature = "onnxruntime")]
fn output_last_dim(shape: &[usize]) -> Result<usize> {
    match shape {
        [_, last] | [1, _, last] => Ok(*last),
        _ => Err(DetectError::InvalidArgument(format!(
            "unsupported ONNX output shape `{shape:?}`"
        ))),
    }
}

#[cfg(feature = "onnxruntime")]
fn softmax(logits: &[f32]) -> Vec<f32> {
    if logits.is_empty() {
        return Vec::new();
    }
    let max = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let mut probabilities = logits
        .iter()
        .map(|value| (value - max).exp())
        .collect::<Vec<_>>();
    let total = probabilities.iter().sum::<f32>();
    if total <= f32::EPSILON || !total.is_finite() {
        return vec![0.0; logits.len()];
    }
    for probability in &mut probabilities {
        *probability /= total;
    }
    probabilities
}

fn preprocessing_from_config(config: &Value) -> Result<OnnxImagePreprocessing> {
    let mut options = OnnxImagePreprocessing::default();
    if let Some(size) = config.get("size") {
        if let Some(shortest) = get_u32(size, "shortest_edge") {
            options.input_width = shortest;
            options.input_height = shortest;
        }
        if let Some(width) = get_u32(size, "width") {
            options.input_width = width;
        }
        if let Some(height) = get_u32(size, "height") {
            options.input_height = height;
        }
    }
    if let Some(value) = config.get("rescale_factor").and_then(Value::as_f64) {
        options.rescale_factor = value as f32;
    }
    if let Some(values) = get_f32_array(config, "image_mean") {
        options.mean = values;
    }
    if let Some(values) = get_f32_array(config, "image_std") {
        options.std = values;
    }
    if let Some(order) = config.get("channel_order").and_then(Value::as_str) {
        options.channel_order = match order {
            "rgb" | "RGB" => ChannelOrder::Rgb,
            "bgr" | "BGR" => ChannelOrder::Bgr,
            other => {
                return Err(DetectError::InvalidArgument(format!(
                    "unsupported ONNX channel order `{other}`"
                )))
            }
        };
    }
    Ok(options)
}

fn label_map_from_config(config: &Value) -> Result<BTreeMap<i64, String>> {
    let Some(id2label) = config.get("id2label").and_then(Value::as_object) else {
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

fn box_format_from_config(config: &Value) -> BoxFormat {
    config
        .get("video_analysis")
        .and_then(|value| value.get("box_format"))
        .and_then(Value::as_str)
        .and_then(|value| match value {
            "xyxy_absolute" => Some(BoxFormat::XyxyAbsolute),
            "xyxy_normalized" => Some(BoxFormat::XyxyNormalized),
            "cxcywh_absolute" => Some(BoxFormat::CxcywhAbsolute),
            "cxcywh_normalized" => Some(BoxFormat::CxcywhNormalized),
            _ => None,
        })
        .unwrap_or(BoxFormat::CxcywhNormalized)
}

fn validate_preprocessing(options: &OnnxImagePreprocessing) -> Result<()> {
    if options.input_width == 0 || options.input_height == 0 {
        return Err(DetectError::InvalidDimensions {
            width: options.input_width,
            height: options.input_height,
        });
    }
    if !options.rescale_factor.is_finite() || options.rescale_factor <= 0.0 {
        return Err(DetectError::InvalidArgument(
            "ONNX rescale factor must be finite and greater than zero".to_string(),
        ));
    }
    if options
        .mean
        .iter()
        .chain(options.std.iter())
        .any(|value| !value.is_finite())
        || options.std.contains(&0.0)
    {
        return Err(DetectError::InvalidArgument(
            "ONNX image mean/std values must be finite and std values must be non-zero".to_string(),
        ));
    }
    Ok(())
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

fn read_json(path: &Path) -> Result<Value> {
    let data = std::fs::read(path)?;
    serde_json::from_slice(&data).map_err(|err| {
        DetectError::Source(format!("failed to parse JSON `{}`: {err}", path.display()))
    })
}

#[cfg(feature = "onnxruntime")]
fn ort_error(error: ort::Error) -> DetectError {
    DetectError::Source(format!("ONNX runtime error: {error}"))
}

fn get_u32(value: &Value, key: &str) -> Option<u32> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
}

fn get_f32_array(config: &Value, key: &str) -> Option<[f32; 3]> {
    let values = config.get(key)?.as_array()?;
    if values.len() != 3 {
        return None;
    }
    Some([
        values[0].as_f64()? as f32,
        values[1].as_f64()? as f32,
        values[2].as_f64()? as f32,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use image_analysis_core::ImagePixelFormat;
    use num_rational::Rational64;
    use video_analysis_core::{FramePosition, PixelFormat, Timebase, Timestamp};
    use video_analysis_models::{ModelBundleFile, ModelBundleManifest};

    #[derive(Debug, Clone)]
    struct FakeRunner {
        output: OnnxObjectDetectionOutput,
        seen_input: Option<OnnxImageTensor>,
    }

    #[derive(Debug, Clone)]
    struct FakePose2dRunner {
        output: Vec<RawPose2dPrediction>,
        seen_input: Option<OnnxImageTensor>,
    }

    impl OnnxPose2dRunner for FakePose2dRunner {
        fn run_pose_2d(&mut self, input: &OnnxImageTensor) -> Result<Vec<RawPose2dPrediction>> {
            self.seen_input = Some(input.clone());
            Ok(self.output.clone())
        }
    }

    #[derive(Debug, Clone)]
    struct FakePoseLiftRunner {
        output: Vec<RawPose3dPrediction>,
    }

    impl OnnxPoseLiftRunner for FakePoseLiftRunner {
        fn run_pose_lift(
            &mut self,
            _sequence: &[RawPose2dPrediction],
        ) -> Result<Vec<RawPose3dPrediction>> {
            Ok(self.output.clone())
        }
    }

    impl OnnxObjectDetectionRunner for FakeRunner {
        fn run_object_detection(
            &mut self,
            input: &OnnxImageTensor,
        ) -> Result<OnnxObjectDetectionOutput> {
            self.seen_input = Some(input.clone());
            Ok(self.output.clone())
        }
    }

    fn fake_bundle_with_task(root: &Path, task: ModelTask, files: &[(&str, &str)]) -> ModelBundle {
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

    fn fake_bundle(root: &Path, files: &[(&str, &str)]) -> ModelBundle {
        fake_bundle_with_task(root, ModelTask::ObjectDetection, files)
    }

    fn frame() -> VideoFrame<'static> {
        static DATA: [u8; 12] = [255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255];
        VideoFrame::rgb24(
            FramePosition {
                frame_index: 0,
                timestamp: Timestamp::new(0, Timebase::new(1, 30)),
            },
            2,
            2,
            &DATA,
        )
        .unwrap()
    }

    #[test]
    fn onnx_backend_rejects_bundle_without_model_file() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = fake_bundle(
            dir.path(),
            &[
                ("config.json", r#"{"id2label":{"0":"person"}}"#),
                ("preprocessor_config.json", "{}"),
            ],
        );
        let error = validate_onnx_vision_bundle(&bundle).unwrap_err();
        assert!(error.to_string().contains("exactly one `.onnx`"));
    }

    #[test]
    fn onnx_backend_rejects_bundle_with_multiple_model_files() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = fake_bundle(
            dir.path(),
            &[
                ("config.json", r#"{"id2label":{"0":"person"}}"#),
                ("model.onnx", "fake"),
                ("onnx/model_quantized.onnx", "fake"),
            ],
        );
        let error = validate_onnx_vision_bundle(&bundle).unwrap_err();
        assert!(error.to_string().contains("found 2"));
    }

    #[test]
    fn onnx_backend_loads_label_map_from_config() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = fake_bundle(
            dir.path(),
            &[
                ("config.json", r#"{"id2label":{"0":"person","2":"car"}}"#),
                (
                    "preprocessor_config.json",
                    r#"{"size":{"width":2,"height":2}}"#,
                ),
                ("onnx/model.onnx", "fake"),
            ],
        );
        let info = validate_onnx_vision_bundle(&bundle).unwrap();
        assert_eq!(info.labels.get(&0).unwrap(), "person");
        assert_eq!(info.labels.get(&2).unwrap(), "car");
    }

    #[test]
    fn preprocessing_converts_rgb_frame_to_nchw_tensor() {
        let options = OnnxImagePreprocessing {
            input_width: 2,
            input_height: 2,
            rescale_factor: 1.0 / 255.0,
            mean: [0.0, 0.0, 0.0],
            std: [1.0, 1.0, 1.0],
            channel_order: ChannelOrder::Rgb,
        };
        let tensor = preprocess_frame(&frame(), &options).unwrap();
        assert_eq!(tensor.channels, 3);
        assert_eq!(tensor.width, 2);
        assert_eq!(tensor.height, 2);
        assert_eq!(tensor.values[0], 1.0);
        assert_eq!(tensor.values[4], 0.0);
        assert_eq!(tensor.values[8], 0.0);
    }

    #[test]
    fn onnx_object_detection_converts_boxes_to_raw_predictions() {
        let mut labels = BTreeMap::new();
        labels.insert(0, "person".to_string());
        let output = OnnxObjectDetectionOutput {
            boxes: vec![[0.5, 0.5, 0.5, 0.5], [10.0, 10.0, 20.0, 20.0]],
            class_ids: vec![0, 9],
            scores: vec![0.9, 0.1],
            box_format: BoxFormat::CxcywhNormalized,
        };
        let predictions = decode_object_detections(&output, &labels, 0.5, (200, 100));
        assert_eq!(predictions.len(), 1);
        assert_eq!(predictions[0].label.as_deref(), Some("person"));
        assert_eq!(
            predictions[0].region,
            Some(RawBoundingBox {
                xmin: Some(0.25),
                ymin: Some(0.25),
                xmax: Some(0.75),
                ymax: Some(0.75),
                normalized: true,
                ..RawBoundingBox::default()
            })
        );
    }

    #[test]
    fn fake_runner_integrates_with_vision_backend() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = fake_bundle(
            dir.path(),
            &[
                (
                    "config.json",
                    r#"{"id2label":{"1":"cat"},"video_analysis":{"box_format":"xyxy_absolute"}}"#,
                ),
                (
                    "preprocessor_config.json",
                    r#"{"size":{"width":2,"height":2}}"#,
                ),
                ("onnx/model.onnx", "fake"),
            ],
        );
        let runner = FakeRunner {
            output: OnnxObjectDetectionOutput {
                boxes: vec![[0.0, 0.0, 2.0, 2.0]],
                class_ids: vec![1],
                scores: vec![0.75],
                box_format: BoxFormat::XyxyAbsolute,
            },
            seen_input: None,
        };
        let mut backend = OnnxObjectDetector::from_runner(bundle, runner).unwrap();
        let predictions = backend.predict_frame(&frame()).unwrap();
        assert_eq!(predictions.len(), 1);
        assert_eq!(predictions[0].label.as_deref(), Some("cat"));
        assert_eq!(backend.runner.seen_input.as_ref().unwrap().width, 2);
    }

    #[test]
    fn pose_2d_estimator_filters_scores_and_maps_to_pixels() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = fake_bundle_with_task(
            dir.path(),
            ModelTask::PoseEstimation2d,
            &[
                (
                    "config.json",
                    r#"{"video_analysis":{"keypoint_names":["nose","left_eye"]}}"#,
                ),
                (
                    "preprocessor_config.json",
                    r#"{"size":{"width":2,"height":2}}"#,
                ),
                ("onnx/model.onnx", "fake"),
            ],
        );
        let runner = FakePose2dRunner {
            output: vec![RawPose2dPrediction {
                score: Some(0.9),
                keypoints: vec![
                    video_analysis_models::RawKeypoint2d {
                        name: "nose".to_string(),
                        x: 0.5,
                        y: 0.25,
                        score: Some(0.9),
                        visible: Some(true),
                    },
                    video_analysis_models::RawKeypoint2d {
                        name: "left_eye".to_string(),
                        x: 0.1,
                        y: 0.2,
                        score: Some(0.1),
                        visible: Some(true),
                    },
                ],
                ..RawPose2dPrediction::default()
            }],
            seen_input: None,
        };
        let mut backend = OnnxPose2dEstimator::with_options(
            OnnxPose2dOptions {
                preprocessing: OnnxImagePreprocessing {
                    input_width: 2,
                    input_height: 2,
                    ..OnnxImagePreprocessing::default()
                },
                output_space: KeypointSpace::Pixel,
                min_pose_score: 0.5,
                min_keypoint_score: 0.5,
                ..pose_2d_options_from_bundle(&bundle).unwrap()
            },
            runner,
        )
        .unwrap();
        let predictions = backend.predict_frame(&frame()).unwrap();
        assert_eq!(predictions.len(), 1);
        assert_eq!(predictions[0].keypoints.len(), 1);
        assert!((predictions[0].keypoints[0].x - 1.0).abs() < 0.001);
    }

    #[test]
    fn pose_lifter_passes_through_fake_runner() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = fake_bundle_with_task(
            dir.path(),
            ModelTask::PoseLifting3d,
            &[("config.json", "{}"), ("onnx/model.onnx", "fake")],
        );
        let mut lifter = OnnxPoseLifter::from_runner(
            bundle,
            FakePoseLiftRunner {
                output: vec![RawPose3dPrediction {
                    score: Some(0.8),
                    keypoints: vec![video_analysis_models::RawKeypoint3d::new(
                        "nose", 0.0, 1.0, 2.0,
                    )],
                    ..RawPose3dPrediction::default()
                }],
            },
        )
        .unwrap();
        let lifted = lifter
            .lift_poses(&[RawPose2dPrediction {
                keypoints: vec![video_analysis_models::RawKeypoint2d::new("nose", 0.0, 0.0)],
                ..RawPose2dPrediction::default()
            }])
            .unwrap();
        assert_eq!(lifted.len(), 1);
        assert_eq!(lifted[0].keypoints.len(), 1);
    }

    #[cfg(feature = "onnxruntime")]
    #[test]
    fn decodes_detr_like_runtime_outputs() {
        let logits = F32Output {
            shape: vec![1, 2, 3],
            values: vec![4.0, 1.0, -2.0, 0.5, 3.0, -1.0],
        };
        let boxes = F32Output {
            shape: vec![1, 2, 4],
            values: vec![0.5, 0.5, 0.25, 0.25, 0.25, 0.25, 0.1, 0.1],
        };

        let output = decode_detr_like_outputs(logits, boxes).unwrap();

        assert_eq!(output.box_format, BoxFormat::CxcywhNormalized);
        assert_eq!(output.boxes.len(), 2);
        assert_eq!(output.class_ids, vec![0, 1]);
        assert!(output.scores[0] > output.scores[1]);
    }

    #[test]
    fn frame_position_helper_uses_imported_rational_for_dev_dependency() {
        let position = FramePosition::from_frame_index(15, Rational64::new(30, 1));
        assert_eq!(position.frame_index, 15);
    }

    #[test]
    #[ignore = "requires a real ONNX object detection bundle and runtime"]
    fn onnx_backend_runs_tiny_detection_model() {
        panic!("external ONNX object detection fixture is intentionally opt-in");
    }

    #[test]
    fn accepts_bgr_input_via_pixel_rgb_conversion() {
        let data = [0_u8, 0, 255];
        let frame = VideoFrame::packed(
            FramePosition {
                frame_index: 0,
                timestamp: Timestamp::new(0, Timebase::new(1, 1)),
            },
            1,
            1,
            PixelFormat::Bgr24,
            &data,
            3,
        )
        .unwrap();
        let image =
            ImageView::new(1, 1, ImagePixelFormat::Bgr24, frame.data, frame.stride).unwrap();
        assert_eq!(image.pixel_rgb(0, 0), [255, 0, 0]);
    }
}
