#![doc = include_str!("../README.md")]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
#[cfg(feature = "onnxruntime")]
use std::sync::Mutex;

use image_analysis_core::{compact_image, ImageBatchView, ImageView, OwnedImageBatch};
use image_analysis_detection::ImageDetection;
use image_analysis_models::{ImageClassification, ImageClassifierBackend};
use image_analysis_processing::resize_nearest;
use serde_json::Value;
use video_analysis_core::{BoundingBox, DetectError, Result};
use video_analysis_models::{ModelBundle, ModelTask};

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
pub struct OnnxImageTensor {
    pub channels: usize,
    pub width: u32,
    pub height: u32,
    pub values: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OnnxImageBatchTensor {
    pub batch_size: usize,
    pub channels: usize,
    pub width: u32,
    pub height: u32,
    pub values: Vec<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxFormat {
    XyxyAbsolute,
    XyxyNormalized,
    CxcywhAbsolute,
    CxcywhNormalized,
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

#[derive(Debug, Clone, Default)]
pub struct UnavailableOnnxRunner;

impl OnnxObjectDetectionRunner for UnavailableOnnxRunner {
    fn run_object_detection(
        &mut self,
        _input: &OnnxImageTensor,
    ) -> Result<OnnxObjectDetectionOutput> {
        Err(DetectError::Source(
            "native ONNX image execution is unavailable; construct with a runner or enable an executor"
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
                DetectError::InvalidArgument("ONNX image model exposes no inputs".to_string())
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
        Self::from_runner(bundle, UnavailableOnnxRunner)
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

    pub fn detect_image(&mut self, image: &ImageView<'_>) -> Result<Vec<ImageDetection>> {
        let input = preprocess_image(image, &self.preprocessing)?;
        let mut output = self.runner.run_object_detection(&input)?;
        if output.box_format != self.box_format {
            output.box_format = self.box_format;
        }
        Ok(decode_object_detections(
            &output,
            &self.labels,
            self.score_threshold,
            (image.width, image.height),
        ))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OnnxImageClassificationOptions {
    pub preprocessing: OnnxImagePreprocessing,
    pub score_threshold: f32,
    pub labels: BTreeMap<i64, String>,
    pub top_k: usize,
}

impl Default for OnnxImageClassificationOptions {
    fn default() -> Self {
        Self {
            preprocessing: OnnxImagePreprocessing::default(),
            score_threshold: 0.0,
            labels: BTreeMap::new(),
            top_k: 5,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OnnxImageClassificationOutput {
    pub class_ids: Vec<i64>,
    pub scores: Vec<f32>,
}

pub trait OnnxImageClassificationRunner {
    fn run_image_classification(
        &mut self,
        input: &OnnxImageTensor,
    ) -> Result<OnnxImageClassificationOutput>;
}

#[derive(Debug, Clone, Default)]
pub struct UnavailableOnnxImageClassificationRunner;

impl OnnxImageClassificationRunner for UnavailableOnnxImageClassificationRunner {
    fn run_image_classification(
        &mut self,
        _input: &OnnxImageTensor,
    ) -> Result<OnnxImageClassificationOutput> {
        Err(DetectError::Source(
            "native ONNX image classification execution is unavailable; construct with a runner"
                .to_string(),
        ))
    }
}

#[derive(Debug, Clone)]
pub struct OnnxImageClassifier<R = UnavailableOnnxImageClassificationRunner> {
    options: OnnxImageClassificationOptions,
    runner: R,
}

impl OnnxImageClassifier<UnavailableOnnxImageClassificationRunner> {
    pub fn from_bundle(bundle: ModelBundle) -> Result<Self> {
        Ok(Self {
            options: classification_options_from_bundle(&bundle)?,
            runner: UnavailableOnnxImageClassificationRunner,
        })
    }
}

impl<R: OnnxImageClassificationRunner> OnnxImageClassifier<R> {
    pub fn from_runner(bundle: ModelBundle, runner: R) -> Result<Self> {
        Ok(Self {
            options: classification_options_from_bundle(&bundle)?,
            runner,
        })
    }

    pub fn with_options(options: OnnxImageClassificationOptions, runner: R) -> Result<Self> {
        validate_preprocessing(&options.preprocessing)?;
        validate_threshold(options.score_threshold)?;
        if options.top_k == 0 {
            return Err(DetectError::InvalidArgument(
                "ONNX classification top_k must be greater than zero".to_string(),
            ));
        }
        Ok(Self { options, runner })
    }

    pub fn options(&self) -> &OnnxImageClassificationOptions {
        &self.options
    }
}

impl<R: OnnxImageClassificationRunner> ImageClassifierBackend for OnnxImageClassifier<R> {
    fn classify_image(&mut self, image: &ImageView<'_>) -> Result<Vec<ImageClassification>> {
        let input = preprocess_image(image, &self.options.preprocessing)?;
        let output = self.runner.run_image_classification(&input)?;
        if output.class_ids.len() != output.scores.len() {
            return Err(DetectError::InvalidArgument(
                "ONNX image classification outputs must have the same number of class ids and scores"
                    .to_string(),
            ));
        }
        let mut labels = output
            .class_ids
            .into_iter()
            .zip(output.scores)
            .filter(|(_, score)| score.is_finite() && *score >= self.options.score_threshold)
            .map(|(class_id, score)| {
                ImageClassification::new(
                    self.options
                        .labels
                        .get(&class_id)
                        .cloned()
                        .unwrap_or_else(|| format!("LABEL_{class_id}")),
                )
                .score(score)
            })
            .collect::<Vec<_>>();
        labels.sort_by(|left, right| {
            right
                .score
                .unwrap_or(0.0)
                .total_cmp(&left.score.unwrap_or(0.0))
        });
        labels.truncate(self.options.top_k);
        Ok(labels)
    }
}

pub fn validate_onnx_vision_bundle(bundle: &ModelBundle) -> Result<OnnxVisionBundleInfo> {
    match bundle.manifest.task {
        ModelTask::ObjectDetection | ModelTask::ImageClassification => {}
        ref task => {
            return Err(DetectError::InvalidArgument(format!(
                "ONNX image bundle task must be object_detection or image_classification, got {:?}",
                task
            )))
        }
    }
    let config_path = required_bundle_file(bundle, "config.json")?;
    let preprocessor_config_path = bundle.file_path("preprocessor_config.json");
    let onnx_files = bundle_files_with_extension(bundle, "onnx");
    let model_path = match onnx_files.as_slice() {
        [path] => path.clone(),
        [] => {
            return Err(DetectError::InvalidArgument(
                "ONNX image bundle must contain exactly one `.onnx` model file".to_string(),
            ))
        }
        files => {
            return Err(DetectError::InvalidArgument(format!(
                "ONNX image bundle must contain exactly one `.onnx` model file, found {}",
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

pub fn options_from_bundle(bundle: &ModelBundle) -> Result<OnnxObjectDetectionOptions> {
    if bundle.manifest.task != ModelTask::ObjectDetection {
        return Err(DetectError::InvalidArgument(format!(
            "ONNX object detection bundle task must be object_detection, got {:?}",
            bundle.manifest.task
        )));
    }
    let info = validate_onnx_vision_bundle(bundle)?;
    let config = read_json(&info.config_path)?;
    let preprocessing = if let Some(path) = &info.preprocessor_config_path {
        preprocessing_from_config(&read_json(path)?)?
    } else {
        OnnxImagePreprocessing::default()
    };
    validate_preprocessing(&preprocessing)?;
    Ok(OnnxObjectDetectionOptions {
        preprocessing,
        score_threshold: 0.0,
        labels: info.labels,
        box_format: box_format_from_config(&config),
    })
}

pub fn classification_options_from_bundle(
    bundle: &ModelBundle,
) -> Result<OnnxImageClassificationOptions> {
    if bundle.manifest.task != ModelTask::ImageClassification {
        return Err(DetectError::InvalidArgument(format!(
            "ONNX image classification bundle task must be image_classification, got {:?}",
            bundle.manifest.task
        )));
    }
    let info = validate_onnx_vision_bundle(bundle)?;
    let preprocessing = if let Some(path) = &info.preprocessor_config_path {
        preprocessing_from_config(&read_json(path)?)?
    } else {
        OnnxImagePreprocessing::default()
    };
    validate_preprocessing(&preprocessing)?;
    Ok(OnnxImageClassificationOptions {
        preprocessing,
        score_threshold: 0.0,
        labels: info.labels,
        top_k: 5,
    })
}

pub fn preprocess_image(
    image: &ImageView<'_>,
    options: &OnnxImagePreprocessing,
) -> Result<OnnxImageTensor> {
    validate_preprocessing(options)?;
    image.validate()?;
    let resized = if image.width == options.input_width && image.height == options.input_height {
        compact_image(image)?
    } else {
        resize_nearest(image, options.input_width, options.input_height)?
    };
    image_to_tensor(&resized.as_view(), options)
}

pub fn preprocess_image_batch(
    batch: ImageBatchView<'_>,
    options: &OnnxImagePreprocessing,
) -> Result<OnnxImageBatchTensor> {
    validate_preprocessing(options)?;
    batch.validate()?;
    if batch.width == options.input_width && batch.height == options.input_height {
        return image_batch_to_tensor(batch, options);
    }

    let mut resized = Vec::with_capacity(batch.batch_size);
    for index in 0..batch.batch_size {
        let image = batch.image(index)?;
        resized.push(resize_nearest(
            &image,
            options.input_width,
            options.input_height,
        )?);
    }
    let resized_batch = OwnedImageBatch::from_images(&resized)?;
    image_batch_to_tensor(resized_batch.as_view(), options)
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

pub fn image_batch_to_tensor(
    batch: ImageBatchView<'_>,
    options: &OnnxImagePreprocessing,
) -> Result<OnnxImageBatchTensor> {
    validate_preprocessing(options)?;
    batch.validate()?;
    let pixels_per_image = batch.width as usize * batch.height as usize;
    let mut values = Vec::with_capacity(batch.batch_size * 3 * pixels_per_image);
    for index in 0..batch.batch_size {
        let image = batch.image(index)?;
        let tensor = image_to_tensor(&image, options)?;
        values.extend(tensor.values);
    }
    Ok(OnnxImageBatchTensor {
        batch_size: batch.batch_size,
        channels: 3,
        width: batch.width,
        height: batch.height,
        values,
    })
}

pub fn decode_object_detections(
    output: &OnnxObjectDetectionOutput,
    labels: &BTreeMap<i64, String>,
    score_threshold: f32,
    original_size: (u32, u32),
) -> Vec<ImageDetection> {
    output
        .boxes
        .iter()
        .zip(output.class_ids.iter())
        .zip(output.scores.iter())
        .filter_map(|((bbox, class_id), score)| {
            if !score.is_finite() || *score < score_threshold {
                return None;
            }
            let region = decode_bounding_box(*bbox, output.box_format, original_size)?;
            Some(ImageDetection {
                label: labels
                    .get(class_id)
                    .cloned()
                    .unwrap_or_else(|| format!("LABEL_{class_id}")),
                score: Some(*score),
                region,
                attributes: BTreeMap::new(),
            })
        })
        .collect()
}

pub fn preprocessing_from_config(config: &Value) -> Result<OnnxImagePreprocessing> {
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

pub fn validate_preprocessing(options: &OnnxImagePreprocessing) -> Result<()> {
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

fn decode_bounding_box(
    bbox: [f32; 4],
    format: BoxFormat,
    original_size: (u32, u32),
) -> Option<BoundingBox> {
    if bbox.iter().any(|value| !value.is_finite()) {
        return None;
    }
    let (width, height) = (original_size.0 as f32, original_size.1 as f32);
    let (mut xmin, mut ymin, mut xmax, mut ymax) = match format {
        BoxFormat::XyxyAbsolute => (bbox[0], bbox[1], bbox[2], bbox[3]),
        BoxFormat::XyxyNormalized => (
            bbox[0] * width,
            bbox[1] * height,
            bbox[2] * width,
            bbox[3] * height,
        ),
        BoxFormat::CxcywhAbsolute => {
            let half_width = bbox[2] / 2.0;
            let half_height = bbox[3] / 2.0;
            (
                bbox[0] - half_width,
                bbox[1] - half_height,
                bbox[0] + half_width,
                bbox[1] + half_height,
            )
        }
        BoxFormat::CxcywhNormalized => {
            let half_width = bbox[2] * width / 2.0;
            let half_height = bbox[3] * height / 2.0;
            (
                bbox[0] * width - half_width,
                bbox[1] * height - half_height,
                bbox[0] * width + half_width,
                bbox[1] * height + half_height,
            )
        }
    };
    xmin = xmin.clamp(0.0, width);
    xmax = xmax.clamp(0.0, width);
    ymin = ymin.clamp(0.0, height);
    ymax = ymax.clamp(0.0, height);
    if xmax <= xmin || ymax <= ymin {
        return None;
    }
    let epsilon = 0.0001;
    let x = (xmin + epsilon).floor().max(0.0) as u32;
    let y = (ymin + epsilon).floor().max(0.0) as u32;
    let box_width = ((xmax - epsilon).ceil() as u32).saturating_sub(x);
    let box_height = ((ymax - epsilon).ceil() as u32).saturating_sub(y);
    BoundingBox::new(x, y, box_width, box_height).ok()
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

    use std::collections::BTreeMap;

    use image_analysis_core::{OwnedImage, OwnedImageBatch};
    use image_analysis_models::ImageClassifierBackend;
    use tempfile::tempdir;
    use video_analysis_models::{ModelBundleFile, ModelBundleManifest};

    #[derive(Debug, Clone)]
    struct FakeDetectionRunner {
        output: OnnxObjectDetectionOutput,
        seen_input: Option<OnnxImageTensor>,
    }

    impl OnnxObjectDetectionRunner for FakeDetectionRunner {
        fn run_object_detection(
            &mut self,
            input: &OnnxImageTensor,
        ) -> Result<OnnxObjectDetectionOutput> {
            self.seen_input = Some(input.clone());
            Ok(self.output.clone())
        }
    }

    #[derive(Debug, Clone)]
    struct FakeClassificationRunner {
        output: OnnxImageClassificationOutput,
    }

    impl OnnxImageClassificationRunner for FakeClassificationRunner {
        fn run_image_classification(
            &mut self,
            _input: &OnnxImageTensor,
        ) -> Result<OnnxImageClassificationOutput> {
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
                name: "fake-image".to_string(),
                repo_id: "fake/image".to_string(),
                revision: "main".to_string(),
                task,
                files: manifest_files,
            },
        }
    }

    #[test]
    fn preprocess_image_builds_channel_first_tensor() {
        let image = OwnedImage::new_rgb(2, 1, vec![255, 0, 0, 0, 255, 0]).unwrap();
        let tensor = image_to_tensor(&image.as_view(), &OnnxImagePreprocessing::default()).unwrap();
        assert_eq!(tensor.values.len(), 6);
        assert_eq!(tensor.channels, 3);
    }

    #[test]
    fn preprocess_image_batch_stacks_single_image_outputs_in_batch_order() {
        let first = OwnedImage::new_rgb(2, 1, vec![255, 0, 0, 0, 255, 0]).unwrap();
        let second = OwnedImage::new_rgb(2, 1, vec![0, 0, 255, 255, 255, 255]).unwrap();
        let batch = OwnedImageBatch::from_images(&[first.clone(), second.clone()]).unwrap();
        let options = OnnxImagePreprocessing::default();

        let stacked = preprocess_image_batch(batch.as_view(), &options).unwrap();
        let first_tensor = preprocess_image(&first.as_view(), &options).unwrap();
        let second_tensor = preprocess_image(&second.as_view(), &options).unwrap();
        let expected = [first_tensor.values, second_tensor.values].concat();

        assert_eq!(stacked.batch_size, 2);
        assert_eq!(stacked.channels, 3);
        assert_eq!(stacked.values, expected);
    }

    #[test]
    fn detector_decodes_fake_runner_output() {
        let dir = tempdir().unwrap();
        let bundle = fake_bundle_with_task(
            dir.path(),
            ModelTask::ObjectDetection,
            &[
                ("config.json", r#"{"id2label":{"0":"person"}}"#),
                ("onnx/model.onnx", "fake"),
            ],
        );
        let runner = FakeDetectionRunner {
            output: OnnxObjectDetectionOutput {
                boxes: vec![[0.5, 0.5, 0.5, 0.5]],
                class_ids: vec![0],
                scores: vec![0.9],
                box_format: BoxFormat::CxcywhNormalized,
            },
            seen_input: None,
        };
        let mut detector = OnnxObjectDetector::from_runner(bundle, runner).unwrap();
        let image = OwnedImage::new_rgb(4, 4, vec![255; 4 * 4 * 3]).unwrap();
        let detections = detector.detect_image(&image.as_view()).unwrap();
        assert_eq!(detections[0].label, "person");
        assert_eq!(detections[0].region, BoundingBox::new(1, 1, 2, 2).unwrap());
    }

    #[test]
    fn classifier_uses_fake_runner_and_bundle_labels() {
        let dir = tempdir().unwrap();
        let bundle = fake_bundle_with_task(
            dir.path(),
            ModelTask::ImageClassification,
            &[
                ("config.json", r#"{"id2label":{"0":"cat","1":"dog"}}"#),
                ("onnx/model.onnx", "fake"),
            ],
        );
        let mut classifier = OnnxImageClassifier::from_runner(
            bundle,
            FakeClassificationRunner {
                output: OnnxImageClassificationOutput {
                    class_ids: vec![1, 0],
                    scores: vec![0.8, 0.2],
                },
            },
        )
        .unwrap();
        let image = OwnedImage::new_rgb(2, 2, vec![255; 12]).unwrap();
        let labels = classifier.classify_image(&image.as_view()).unwrap();
        assert_eq!(labels[0].label, "dog");
        assert_eq!(labels.len(), 2);
    }
}
