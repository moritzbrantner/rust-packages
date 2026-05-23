#![doc = include_str!("../README.md")]

pub mod surface;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
#[cfg(feature = "onnxruntime")]
use std::sync::Mutex;

use image_analysis_core::{
    compact_image, ImageBatchView, ImagePixelFormat, ImageView, OwnedImage, OwnedImageBatch,
};
use image_analysis_detection::{
    FaceBox, FaceDetection, FaceDetectorBackend, FaceLandmarks, ImageDetection,
};
use image_analysis_processing::resize_nearest;
use image_analysis_tasks::{
    FaceEmbedderBackend, FaceEmbedding, ImageClassification, ImageClassifierBackend,
    ImageEmbedderBackend, ImageEmbedding,
};
use model_runtime::{ModelBundle, ModelTask};
use serde_json::Value;
use video_analysis_core::{BoundingBox, DetectError, Result};

#[derive(Debug, Clone, PartialEq)]
/// Data type for ONNX vision bundle info.
pub struct OnnxVisionBundleInfo {
    /// The config path value.
    pub config_path: PathBuf,
    /// The preprocessor config path value.
    pub preprocessor_config_path: Option<PathBuf>,
    /// The model path value.
    pub model_path: PathBuf,
    /// The labels value.
    pub labels: BTreeMap<i64, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Variants describing channel order.
pub enum ChannelOrder {
    /// The RGB variant.
    Rgb,
    /// The BGR variant.
    Bgr,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for ONNX image preprocessing.
pub struct OnnxImagePreprocessing {
    /// The input width value.
    pub input_width: u32,
    /// The input height value.
    pub input_height: u32,
    /// The rescale factor value.
    pub rescale_factor: f32,
    /// The mean value.
    pub mean: [f32; 3],
    /// The std value.
    pub std: [f32; 3],
    /// The channel order value.
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
/// Data type for ONNX image tensor.
pub struct OnnxImageTensor {
    /// Number of audio channels.
    pub channels: usize,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// The values value.
    pub values: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for ONNX image batch tensor.
pub struct OnnxImageBatchTensor {
    /// The batch size value.
    pub batch_size: usize,
    /// Number of audio channels.
    pub channels: usize,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// The values value.
    pub values: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for f32 ONNX tensor output.
pub struct OnnxF32Tensor {
    /// Tensor shape.
    pub shape: Vec<usize>,
    /// Tensor values.
    pub values: Vec<f32>,
}

impl OnnxF32Tensor {
    /// Creates a new value.
    pub fn new(shape: Vec<usize>, values: Vec<f32>) -> Result<Self> {
        if values.iter().any(|value| !value.is_finite()) {
            return Err(DetectError::InvalidArgument(
                "ONNX f32 tensor values must be finite".to_string(),
            ));
        }
        let expected = shape.iter().product::<usize>();
        if expected != values.len() {
            return Err(DetectError::InvalidArgument(format!(
                "ONNX f32 tensor shape {shape:?} does not match {} value(s)",
                values.len()
            )));
        }
        Ok(Self { shape, values })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Variants describing box format.
pub enum BoxFormat {
    /// The xyxy absolute variant.
    XyxyAbsolute,
    /// The xyxy normalized variant.
    XyxyNormalized,
    /// The cxcywh absolute variant.
    CxcywhAbsolute,
    /// The cxcywh normalized variant.
    CxcywhNormalized,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for ONNX object detection options.
pub struct OnnxObjectDetectionOptions {
    /// The preprocessing value.
    pub preprocessing: OnnxImagePreprocessing,
    /// The score threshold value.
    pub score_threshold: f32,
    /// The labels value.
    pub labels: BTreeMap<i64, String>,
    /// The box format value.
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
/// Data type for ONNX object detection output.
pub struct OnnxObjectDetectionOutput {
    /// The boxes value.
    pub boxes: Vec<[f32; 4]>,
    /// The class identifiers value.
    pub class_ids: Vec<i64>,
    /// The scores value.
    pub scores: Vec<f32>,
    /// The box format value.
    pub box_format: BoxFormat,
}

/// Trait for ONNX object detection runner implementations.
pub trait OnnxObjectDetectionRunner {
    /// Runs object detection.
    fn run_object_detection(
        &mut self,
        input: &OnnxImageTensor,
    ) -> Result<OnnxObjectDetectionOutput>;
}

#[derive(Debug, Clone, Default)]
/// Data type for unavailable ONNX runner.
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
/// Data type for native ONNX runner.
pub struct NativeOnnxRunner {
    session: Mutex<ort::session::Session>,
}

#[cfg(feature = "onnxruntime")]
impl NativeOnnxRunner {
    /// Creates a new value.
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

#[cfg(feature = "onnxruntime")]
impl OnnxImageEmbeddingRunner for NativeOnnxRunner {
    fn run_image_embedding(&mut self, input: &OnnxImageTensor) -> Result<Vec<OnnxF32Tensor>> {
        run_native_f32_outputs(&self.session, input).map(|outputs| {
            outputs
                .into_iter()
                .map(|(_, tensor)| tensor)
                .collect::<Vec<_>>()
        })
    }
}

#[cfg(feature = "onnxruntime")]
impl OnnxFaceDetectionRunner for NativeOnnxRunner {
    fn run_face_detection(&mut self, input: &OnnxImageTensor) -> Result<OnnxFaceDetectionOutput> {
        Ok(OnnxFaceDetectionOutput {
            tensors: run_native_f32_outputs(&self.session, input)?
                .into_iter()
                .collect(),
        })
    }
}

#[cfg(feature = "onnxruntime")]
impl OnnxFaceEmbeddingRunner for NativeOnnxRunner {
    fn run_face_embedding(&mut self, input: &OnnxImageTensor) -> Result<Vec<OnnxF32Tensor>> {
        run_native_f32_outputs(&self.session, input).map(|outputs| {
            outputs
                .into_iter()
                .map(|(_, tensor)| tensor)
                .collect::<Vec<_>>()
        })
    }
}

#[derive(Debug, Clone)]
/// Data type for ONNX object detector.
pub struct OnnxObjectDetector<R = UnavailableOnnxRunner> {
    preprocessing: OnnxImagePreprocessing,
    score_threshold: f32,
    labels: BTreeMap<i64, String>,
    box_format: BoxFormat,
    runner: R,
}

#[cfg(not(feature = "onnxruntime"))]
impl OnnxObjectDetector<UnavailableOnnxRunner> {
    /// Builds this value from bundle.
    pub fn from_bundle(bundle: ModelBundle) -> Result<Self> {
        Self::from_runner(bundle, UnavailableOnnxRunner)
    }
}

#[cfg(feature = "onnxruntime")]
impl OnnxObjectDetector<NativeOnnxRunner> {
    /// Builds this value from bundle.
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
    /// Builds this value from runner.
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

    /// Returns this value with options.
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

    /// Returns preprocessing.
    pub fn preprocessing(&self) -> &OnnxImagePreprocessing {
        &self.preprocessing
    }

    /// Returns labels.
    pub fn labels(&self) -> &BTreeMap<i64, String> {
        &self.labels
    }

    /// Returns detect image.
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
/// Data type for ONNX image classification options.
pub struct OnnxImageClassificationOptions {
    /// The preprocessing value.
    pub preprocessing: OnnxImagePreprocessing,
    /// The score threshold value.
    pub score_threshold: f32,
    /// The labels value.
    pub labels: BTreeMap<i64, String>,
    /// The top k value.
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
/// Data type for ONNX image classification output.
pub struct OnnxImageClassificationOutput {
    /// The class identifiers value.
    pub class_ids: Vec<i64>,
    /// The scores value.
    pub scores: Vec<f32>,
}

/// Trait for ONNX image classification runner implementations.
pub trait OnnxImageClassificationRunner {
    /// Runs image classification.
    fn run_image_classification(
        &mut self,
        input: &OnnxImageTensor,
    ) -> Result<OnnxImageClassificationOutput>;
}

#[derive(Debug, Clone, Default)]
/// Data type for unavailable ONNX image classification runner.
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
/// Data type for ONNX image classifier.
pub struct OnnxImageClassifier<R = UnavailableOnnxImageClassificationRunner> {
    options: OnnxImageClassificationOptions,
    runner: R,
}

impl OnnxImageClassifier<UnavailableOnnxImageClassificationRunner> {
    /// Builds this value from bundle.
    pub fn from_bundle(bundle: ModelBundle) -> Result<Self> {
        Ok(Self {
            options: classification_options_from_bundle(&bundle)?,
            runner: UnavailableOnnxImageClassificationRunner,
        })
    }
}

impl<R: OnnxImageClassificationRunner> OnnxImageClassifier<R> {
    /// Builds this value from runner.
    pub fn from_runner(bundle: ModelBundle, runner: R) -> Result<Self> {
        Ok(Self {
            options: classification_options_from_bundle(&bundle)?,
            runner,
        })
    }

    /// Returns this value with options.
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

    /// Returns options.
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

#[derive(Debug, Clone, PartialEq)]
/// Data type for ONNX image embedding options.
pub struct OnnxImageEmbeddingOptions {
    /// The preprocessing value.
    pub preprocessing: OnnxImagePreprocessing,
    /// Expected vector size.
    pub expected_vector_size: Option<usize>,
    /// Whether to L2 normalize output.
    pub normalize: bool,
}

impl Default for OnnxImageEmbeddingOptions {
    fn default() -> Self {
        Self {
            preprocessing: OnnxImagePreprocessing {
                input_width: 224,
                input_height: 224,
                rescale_factor: 1.0 / 255.0,
                mean: [0.48145466, 0.4578275, 0.40821073],
                std: [0.26862954, 0.261_302_6, 0.275_777_1],
                channel_order: ChannelOrder::Rgb,
            },
            expected_vector_size: None,
            normalize: true,
        }
    }
}

/// Trait for ONNX image embedding runner implementations.
pub trait OnnxImageEmbeddingRunner {
    /// Runs image embedding.
    fn run_image_embedding(&mut self, input: &OnnxImageTensor) -> Result<Vec<OnnxF32Tensor>>;
}

#[derive(Debug, Clone, Default)]
/// Data type for unavailable ONNX image embedding runner.
pub struct UnavailableOnnxImageEmbeddingRunner;

impl OnnxImageEmbeddingRunner for UnavailableOnnxImageEmbeddingRunner {
    fn run_image_embedding(&mut self, _input: &OnnxImageTensor) -> Result<Vec<OnnxF32Tensor>> {
        Err(DetectError::Source(
            "native ONNX image embedding execution is unavailable; construct with a runner"
                .to_string(),
        ))
    }
}

#[derive(Debug, Clone)]
/// Data type for ONNX image embedder.
pub struct OnnxImageEmbedder<R = UnavailableOnnxImageEmbeddingRunner> {
    options: OnnxImageEmbeddingOptions,
    runner: R,
}

impl OnnxImageEmbedder<UnavailableOnnxImageEmbeddingRunner> {
    /// Builds this value from bundle.
    pub fn from_bundle(bundle: ModelBundle) -> Result<Self> {
        Ok(Self {
            options: image_embedding_options_from_bundle(&bundle)?,
            runner: UnavailableOnnxImageEmbeddingRunner,
        })
    }
}

#[cfg(feature = "onnxruntime")]
impl OnnxImageEmbedder<NativeOnnxRunner> {
    /// Builds this value from model and preprocessor paths.
    pub fn from_paths(
        model_path: impl Into<PathBuf>,
        preprocessor_path: impl AsRef<Path>,
        expected_vector_size: Option<usize>,
    ) -> Result<Self> {
        let preprocessing = preprocessing_from_config(&read_json(preprocessor_path.as_ref())?)?;
        let options = OnnxImageEmbeddingOptions {
            preprocessing,
            expected_vector_size,
            normalize: true,
        };
        Self::with_options(options, NativeOnnxRunner::new(model_path)?)
    }

    /// Builds this value from bundle.
    pub fn from_bundle(bundle: ModelBundle) -> Result<Self> {
        let info = validate_onnx_vision_bundle(&bundle)?;
        Self::with_options(
            image_embedding_options_from_bundle(&bundle)?,
            NativeOnnxRunner::new(info.model_path)?,
        )
    }
}

impl<R: OnnxImageEmbeddingRunner> OnnxImageEmbedder<R> {
    /// Builds this value from runner.
    pub fn from_runner(bundle: ModelBundle, runner: R) -> Result<Self> {
        Ok(Self {
            options: image_embedding_options_from_bundle(&bundle)?,
            runner,
        })
    }

    /// Returns this value with options.
    pub fn with_options(options: OnnxImageEmbeddingOptions, runner: R) -> Result<Self> {
        validate_preprocessing(&options.preprocessing)?;
        if let Some(size) = options.expected_vector_size {
            if size == 0 {
                return Err(DetectError::InvalidArgument(
                    "ONNX image embedding expected vector size must be greater than zero"
                        .to_string(),
                ));
            }
        }
        Ok(Self { options, runner })
    }

    /// Returns options.
    pub fn options(&self) -> &OnnxImageEmbeddingOptions {
        &self.options
    }
}

impl<R: OnnxImageEmbeddingRunner> ImageEmbedderBackend for OnnxImageEmbedder<R> {
    fn embed_image(&mut self, image: &ImageView<'_>) -> Result<ImageEmbedding> {
        let input = preprocess_image(image, &self.options.preprocessing)?;
        let output = self.runner.run_image_embedding(&input)?;
        let mut vector = select_embedding_vector(&output, self.options.expected_vector_size)?;
        if self.options.normalize {
            normalize_vector(&mut vector);
        }
        ImageEmbedding::new(vector)
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for ONNX face detection output.
pub struct OnnxFaceDetectionOutput {
    /// Named output tensors.
    pub tensors: BTreeMap<String, OnnxF32Tensor>,
}

/// Trait for ONNX face detection runner implementations.
pub trait OnnxFaceDetectionRunner {
    /// Runs face detection.
    fn run_face_detection(&mut self, input: &OnnxImageTensor) -> Result<OnnxFaceDetectionOutput>;
}

#[derive(Debug, Clone, Default)]
/// Data type for unavailable ONNX face detection runner.
pub struct UnavailableOnnxFaceDetectionRunner;

impl OnnxFaceDetectionRunner for UnavailableOnnxFaceDetectionRunner {
    fn run_face_detection(&mut self, _input: &OnnxImageTensor) -> Result<OnnxFaceDetectionOutput> {
        Err(DetectError::Source(
            "native ONNX face detection execution is unavailable; construct with a runner"
                .to_string(),
        ))
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for ONNX face detection options.
pub struct OnnxFaceDetectionOptions {
    /// The preprocessing value.
    pub preprocessing: OnnxImagePreprocessing,
    /// The score threshold value.
    pub score_threshold: f32,
    /// The NMS threshold value.
    pub nms_threshold: f32,
    /// The top k value.
    pub top_k: usize,
}

impl Default for OnnxFaceDetectionOptions {
    fn default() -> Self {
        Self {
            preprocessing: OnnxImagePreprocessing {
                input_width: 320,
                input_height: 320,
                rescale_factor: 1.0,
                mean: [0.0, 0.0, 0.0],
                std: [1.0, 1.0, 1.0],
                channel_order: ChannelOrder::Rgb,
            },
            score_threshold: 0.9,
            nms_threshold: 0.3,
            top_k: 5000,
        }
    }
}

#[derive(Debug, Clone)]
/// Data type for ONNX face detector.
pub struct OnnxFaceDetector<R = UnavailableOnnxFaceDetectionRunner> {
    options: OnnxFaceDetectionOptions,
    runner: R,
}

impl OnnxFaceDetector<UnavailableOnnxFaceDetectionRunner> {
    /// Builds this value from bundle.
    pub fn from_bundle(bundle: ModelBundle) -> Result<Self> {
        let _ = face_detection_options_from_bundle(&bundle)?;
        Ok(Self {
            options: OnnxFaceDetectionOptions::default(),
            runner: UnavailableOnnxFaceDetectionRunner,
        })
    }
}

#[cfg(feature = "onnxruntime")]
impl OnnxFaceDetector<NativeOnnxRunner> {
    /// Builds this value from model path.
    pub fn from_model_path(model_path: impl Into<PathBuf>, score_threshold: f32) -> Result<Self> {
        let options = OnnxFaceDetectionOptions {
            score_threshold,
            ..OnnxFaceDetectionOptions::default()
        };
        Self::with_options(options, NativeOnnxRunner::new(model_path)?)
    }

    /// Builds this value from bundle.
    pub fn from_bundle(bundle: ModelBundle) -> Result<Self> {
        let info = validate_onnx_vision_bundle(&bundle)?;
        Self::with_options(
            face_detection_options_from_bundle(&bundle)?,
            NativeOnnxRunner::new(info.model_path)?,
        )
    }
}

impl<R: OnnxFaceDetectionRunner> OnnxFaceDetector<R> {
    /// Builds this value from runner.
    pub fn from_runner(bundle: ModelBundle, runner: R) -> Result<Self> {
        Ok(Self {
            options: face_detection_options_from_bundle(&bundle)?,
            runner,
        })
    }

    /// Returns this value with options.
    pub fn with_options(options: OnnxFaceDetectionOptions, runner: R) -> Result<Self> {
        validate_preprocessing(&options.preprocessing)?;
        validate_threshold(options.score_threshold)?;
        validate_threshold(options.nms_threshold)?;
        if options.top_k == 0 {
            return Err(DetectError::InvalidArgument(
                "ONNX face detection top_k must be greater than zero".to_string(),
            ));
        }
        Ok(Self { options, runner })
    }

    /// Returns options.
    pub fn options(&self) -> &OnnxFaceDetectionOptions {
        &self.options
    }
}

impl<R: OnnxFaceDetectionRunner> FaceDetectorBackend for OnnxFaceDetector<R> {
    fn detect_faces(&mut self, image: &ImageView<'_>) -> Result<Vec<FaceDetection>> {
        let input = preprocess_image(image, &self.options.preprocessing)?;
        let output = self.runner.run_face_detection(&input)?;
        decode_yunet_faces(
            &output.tensors,
            &self.options,
            (image.width as f32, image.height as f32),
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for ONNX face embedding options.
pub struct OnnxFaceEmbeddingOptions {
    /// The preprocessing value.
    pub preprocessing: OnnxImagePreprocessing,
    /// Expected vector size.
    pub expected_vector_size: Option<usize>,
    /// Whether to L2 normalize output.
    pub normalize: bool,
}

impl Default for OnnxFaceEmbeddingOptions {
    fn default() -> Self {
        Self {
            preprocessing: OnnxImagePreprocessing {
                input_width: 112,
                input_height: 112,
                rescale_factor: 1.0,
                mean: [0.0, 0.0, 0.0],
                std: [1.0, 1.0, 1.0],
                channel_order: ChannelOrder::Rgb,
            },
            expected_vector_size: None,
            normalize: true,
        }
    }
}

/// Trait for ONNX face embedding runner implementations.
pub trait OnnxFaceEmbeddingRunner {
    /// Runs face embedding.
    fn run_face_embedding(&mut self, input: &OnnxImageTensor) -> Result<Vec<OnnxF32Tensor>>;
}

#[derive(Debug, Clone, Default)]
/// Data type for unavailable ONNX face embedding runner.
pub struct UnavailableOnnxFaceEmbeddingRunner;

impl OnnxFaceEmbeddingRunner for UnavailableOnnxFaceEmbeddingRunner {
    fn run_face_embedding(&mut self, _input: &OnnxImageTensor) -> Result<Vec<OnnxF32Tensor>> {
        Err(DetectError::Source(
            "native ONNX face embedding execution is unavailable; construct with a runner"
                .to_string(),
        ))
    }
}

#[derive(Debug, Clone)]
/// Data type for ONNX face embedder.
pub struct OnnxFaceEmbedder<R = UnavailableOnnxFaceEmbeddingRunner> {
    options: OnnxFaceEmbeddingOptions,
    runner: R,
}

impl OnnxFaceEmbedder<UnavailableOnnxFaceEmbeddingRunner> {
    /// Builds this value from bundle.
    pub fn from_bundle(bundle: ModelBundle) -> Result<Self> {
        let _ = face_embedding_options_from_bundle(&bundle)?;
        Ok(Self {
            options: OnnxFaceEmbeddingOptions::default(),
            runner: UnavailableOnnxFaceEmbeddingRunner,
        })
    }
}

#[cfg(feature = "onnxruntime")]
impl OnnxFaceEmbedder<NativeOnnxRunner> {
    /// Builds this value from model path.
    pub fn from_model_path(
        model_path: impl Into<PathBuf>,
        expected_vector_size: Option<usize>,
    ) -> Result<Self> {
        let options = OnnxFaceEmbeddingOptions {
            expected_vector_size,
            ..OnnxFaceEmbeddingOptions::default()
        };
        Self::with_options(options, NativeOnnxRunner::new(model_path)?)
    }

    /// Builds this value from bundle.
    pub fn from_bundle(bundle: ModelBundle) -> Result<Self> {
        let info = validate_onnx_vision_bundle(&bundle)?;
        Self::with_options(
            face_embedding_options_from_bundle(&bundle)?,
            NativeOnnxRunner::new(info.model_path)?,
        )
    }
}

impl<R: OnnxFaceEmbeddingRunner> OnnxFaceEmbedder<R> {
    /// Builds this value from runner.
    pub fn from_runner(bundle: ModelBundle, runner: R) -> Result<Self> {
        Ok(Self {
            options: face_embedding_options_from_bundle(&bundle)?,
            runner,
        })
    }

    /// Returns this value with options.
    pub fn with_options(options: OnnxFaceEmbeddingOptions, runner: R) -> Result<Self> {
        validate_preprocessing(&options.preprocessing)?;
        if let Some(size) = options.expected_vector_size {
            if size == 0 {
                return Err(DetectError::InvalidArgument(
                    "ONNX face embedding expected vector size must be greater than zero"
                        .to_string(),
                ));
            }
        }
        Ok(Self { options, runner })
    }

    /// Returns options.
    pub fn options(&self) -> &OnnxFaceEmbeddingOptions {
        &self.options
    }
}

impl<R: OnnxFaceEmbeddingRunner> FaceEmbedderBackend for OnnxFaceEmbedder<R> {
    fn embed_face(
        &mut self,
        image: &ImageView<'_>,
        detection: Option<&FaceDetection>,
    ) -> Result<FaceEmbedding> {
        let aligned = face_chip(image, detection, self.options.preprocessing.input_width)?;
        let input = preprocess_image(&aligned.as_view(), &self.options.preprocessing)?;
        let output = self.runner.run_face_embedding(&input)?;
        let mut vector = select_embedding_vector(&output, self.options.expected_vector_size)?;
        if self.options.normalize {
            normalize_vector(&mut vector);
        }
        FaceEmbedding::new(vector)
    }
}

/// Validates ONNX vision bundle.
pub fn validate_onnx_vision_bundle(bundle: &ModelBundle) -> Result<OnnxVisionBundleInfo> {
    match bundle.manifest.task {
        ModelTask::ObjectDetection | ModelTask::ImageClassification => {}
        ModelTask::Custom(ref task)
            if task == "image_embedding" || task == "face_detection" || task == "face_embedding" => {}
        ref task => {
            return Err(DetectError::InvalidArgument(format!(
                "ONNX image bundle task must be object_detection, image_classification, image_embedding, face_detection, or face_embedding, got {:?}",
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

/// Returns options from bundle.
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

/// Returns classification options from bundle.
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

/// Returns image embedding options from bundle.
pub fn image_embedding_options_from_bundle(
    bundle: &ModelBundle,
) -> Result<OnnxImageEmbeddingOptions> {
    if bundle.manifest.task != ModelTask::Custom("image_embedding".to_string()) {
        return Err(DetectError::InvalidArgument(format!(
            "ONNX image embedding bundle task must be image_embedding, got {:?}",
            bundle.manifest.task
        )));
    }
    let info = validate_onnx_vision_bundle(bundle)?;
    let preprocessing = if let Some(path) = &info.preprocessor_config_path {
        preprocessing_from_config(&read_json(path)?)?
    } else {
        OnnxImageEmbeddingOptions::default().preprocessing
    };
    validate_preprocessing(&preprocessing)?;
    Ok(OnnxImageEmbeddingOptions {
        preprocessing,
        expected_vector_size: None,
        normalize: true,
    })
}

/// Returns face detection options from bundle.
pub fn face_detection_options_from_bundle(
    bundle: &ModelBundle,
) -> Result<OnnxFaceDetectionOptions> {
    if bundle.manifest.task != ModelTask::Custom("face_detection".to_string()) {
        return Err(DetectError::InvalidArgument(format!(
            "ONNX face detection bundle task must be face_detection, got {:?}",
            bundle.manifest.task
        )));
    }
    validate_onnx_vision_bundle(bundle)?;
    Ok(OnnxFaceDetectionOptions::default())
}

/// Returns face embedding options from bundle.
pub fn face_embedding_options_from_bundle(
    bundle: &ModelBundle,
) -> Result<OnnxFaceEmbeddingOptions> {
    if bundle.manifest.task != ModelTask::Custom("face_embedding".to_string()) {
        return Err(DetectError::InvalidArgument(format!(
            "ONNX face embedding bundle task must be face_embedding, got {:?}",
            bundle.manifest.task
        )));
    }
    validate_onnx_vision_bundle(bundle)?;
    Ok(OnnxFaceEmbeddingOptions::default())
}

/// Returns preprocess image.
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

/// Returns preprocess image batch.
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

/// Returns image to tensor.
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

/// Returns image batch to tensor.
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

/// Returns decode object detections.
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

/// Returns preprocessing from config.
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

/// Validates preprocessing.
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
fn run_native_f32_outputs(
    session: &Mutex<ort::session::Session>,
    input: &OnnxImageTensor,
) -> Result<Vec<(String, OnnxF32Tensor)>> {
    use std::borrow::Cow;

    use ort::session::SessionInputValue;
    use ort::value::Tensor;

    let mut session = session
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
    let names = outputs
        .keys()
        .map(ToOwned::to_owned)
        .collect::<Vec<String>>();
    let mut tensors = Vec::with_capacity(names.len());
    for name in names {
        let output = outputs.get(&name).ok_or_else(|| {
            DetectError::Source(format!("ONNX output `{name}` disappeared during decode"))
        })?;
        let extracted = extract_f32_output(output)?;
        tensors.push((name, OnnxF32Tensor::new(extracted.shape, extracted.values)?));
    }
    Ok(tensors)
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

fn select_embedding_vector(
    outputs: &[OnnxF32Tensor],
    expected_vector_size: Option<usize>,
) -> Result<Vec<f32>> {
    let mut candidates = Vec::new();
    for output in outputs {
        if output.shape.is_empty()
            || output.values.is_empty()
            || output.values.iter().any(|value| !value.is_finite())
        {
            continue;
        }
        let accepted_shape = match output.shape.as_slice() {
            [dimensions] => Some(*dimensions),
            [1, dimensions] => Some(*dimensions),
            _ if output.shape.first() == Some(&1) => output.shape.last().copied(),
            _ => None,
        };
        let Some(dimensions) = accepted_shape else {
            continue;
        };
        if dimensions != output.values.len() {
            continue;
        }
        if expected_vector_size.is_some_and(|expected| expected != dimensions) {
            continue;
        }
        candidates.push(output.values.clone());
    }
    match candidates.as_slice() {
        [vector] => Ok(vector.clone()),
        [] => Err(DetectError::InvalidArgument(
            "ONNX embedding model returned no compatible f32 vector output".to_string(),
        )),
        _ => Err(DetectError::InvalidArgument(
            "ONNX embedding model returned multiple compatible f32 vector outputs".to_string(),
        )),
    }
}

fn normalize_vector(vector: &mut [f32]) {
    let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm > 0.0 && norm.is_finite() {
        for value in vector {
            *value /= norm;
        }
    }
}

fn decode_yunet_faces(
    outputs: &BTreeMap<String, OnnxF32Tensor>,
    options: &OnnxFaceDetectionOptions,
    original_size: (f32, f32),
) -> Result<Vec<FaceDetection>> {
    let mut decoded = Vec::new();
    for stride in [8_usize, 16, 32] {
        let cls = required_output(outputs, &format!("cls_{stride}"))?;
        let obj = required_output(outputs, &format!("obj_{stride}"))?;
        let bbox = required_output(outputs, &format!("bbox_{stride}"))?;
        let kps = required_output(outputs, &format!("kps_{stride}"))?;
        let count = cls.values.len().min(obj.values.len());
        if bbox.values.len() < count * 4 || kps.values.len() < count * 10 {
            return Err(DetectError::InvalidArgument(format!(
                "YuNet stride {stride} output tensors have inconsistent lengths"
            )));
        }
        let grid_width = (options.preprocessing.input_width as usize).div_ceil(stride);
        for index in 0..count {
            let score = cls.values[index] * obj.values[index];
            if !score.is_finite() || score < options.score_threshold {
                continue;
            }
            let grid_x = index % grid_width;
            let grid_y = index / grid_width.max(1);
            let center_x = (grid_x as f32 + 0.5) * stride as f32;
            let center_y = (grid_y as f32 + 0.5) * stride as f32;
            let box_offset = index * 4;
            let face_width = (bbox.values[box_offset + 2] * 0.2).exp() * stride as f32;
            let face_height = (bbox.values[box_offset + 3] * 0.2).exp() * stride as f32;
            let face_center_x = center_x + bbox.values[box_offset] * 0.1 * stride as f32;
            let face_center_y = center_y + bbox.values[box_offset + 1] * 0.1 * stride as f32;
            let x = (face_center_x - face_width / 2.0) / options.preprocessing.input_width as f32;
            let y = (face_center_y - face_height / 2.0) / options.preprocessing.input_height as f32;
            let width = face_width / options.preprocessing.input_width as f32;
            let height = face_height / options.preprocessing.input_height as f32;
            let bbox = FaceBox::new(
                x.clamp(0.0, 1.0),
                y.clamp(0.0, 1.0),
                width.clamp(0.0, 1.0),
                height.clamp(0.0, 1.0),
            )?;
            let mut points = Vec::with_capacity(5);
            let kps_offset = index * 10;
            for point in 0..5 {
                let point_x = (center_x + kps.values[kps_offset + point * 2] * 0.1 * stride as f32)
                    / options.preprocessing.input_width as f32;
                let point_y = (center_y
                    + kps.values[kps_offset + point * 2 + 1] * 0.1 * stride as f32)
                    / options.preprocessing.input_height as f32;
                points.push([point_x.clamp(0.0, 1.0), point_y.clamp(0.0, 1.0)]);
            }
            let detection = FaceDetection::new(bbox, score)?
                .landmarks(FaceLandmarks::new(points)?)
                .attribute("model", "yunet");
            decoded.push(detection);
        }
    }
    decoded.sort_by(|left, right| right.confidence.total_cmp(&left.confidence));
    decoded.truncate(options.top_k);
    Ok(non_max_suppression(
        decoded,
        options.nms_threshold,
        original_size,
    ))
}

fn required_output<'a>(
    outputs: &'a BTreeMap<String, OnnxF32Tensor>,
    name: &str,
) -> Result<&'a OnnxF32Tensor> {
    outputs
        .get(name)
        .ok_or_else(|| DetectError::InvalidArgument(format!("YuNet output `{name}` is missing")))
}

fn non_max_suppression(
    detections: Vec<FaceDetection>,
    threshold: f32,
    original_size: (f32, f32),
) -> Vec<FaceDetection> {
    let mut kept: Vec<FaceDetection> = Vec::new();
    for detection in detections {
        if kept
            .iter()
            .all(|existing| face_iou(&existing.bbox, &detection.bbox, original_size) < threshold)
        {
            kept.push(detection);
        }
    }
    kept
}

fn face_iou(left: &FaceBox, right: &FaceBox, original_size: (f32, f32)) -> f32 {
    let width = original_size.0.max(1.0);
    let height = original_size.1.max(1.0);
    let left_x1 = left.x * width;
    let left_y1 = left.y * height;
    let left_x2 = (left.x + left.width) * width;
    let left_y2 = (left.y + left.height) * height;
    let right_x1 = right.x * width;
    let right_y1 = right.y * height;
    let right_x2 = (right.x + right.width) * width;
    let right_y2 = (right.y + right.height) * height;
    let intersection_width = (left_x2.min(right_x2) - left_x1.max(right_x1)).max(0.0);
    let intersection_height = (left_y2.min(right_y2) - left_y1.max(right_y1)).max(0.0);
    let intersection = intersection_width * intersection_height;
    let left_area = (left_x2 - left_x1).max(0.0) * (left_y2 - left_y1).max(0.0);
    let right_area = (right_x2 - right_x1).max(0.0) * (right_y2 - right_y1).max(0.0);
    let union = left_area + right_area - intersection;
    if union <= f32::EPSILON {
        0.0
    } else {
        intersection / union
    }
}

fn face_chip(
    image: &ImageView<'_>,
    detection: Option<&FaceDetection>,
    output_size: u32,
) -> Result<OwnedImage> {
    image.validate()?;
    let output_size = output_size.max(1);
    let (x, y, size) = face_crop_rect(image, detection);
    let mut data = Vec::with_capacity(output_size as usize * output_size as usize * 3);
    for out_y in 0..output_size {
        for out_x in 0..output_size {
            let src_x = x + (out_x as f32 + 0.5) * size / output_size as f32;
            let src_y = y + (out_y as f32 + 0.5) * size / output_size as f32;
            let src_x = src_x.floor().clamp(0.0, (image.width - 1) as f32) as u32;
            let src_y = src_y.floor().clamp(0.0, (image.height - 1) as f32) as u32;
            data.extend_from_slice(&image.pixel_rgb(src_x, src_y));
        }
    }
    OwnedImage::new(
        output_size,
        output_size,
        ImagePixelFormat::Rgb24,
        data,
        output_size as usize * 3,
    )
}

fn face_crop_rect(image: &ImageView<'_>, detection: Option<&FaceDetection>) -> (f32, f32, f32) {
    let Some(detection) = detection else {
        let size = image.width.min(image.height).max(1) as f32;
        return (0.0, 0.0, size);
    };
    let image_width = image.width as f32;
    let image_height = image.height as f32;
    let mut x1 = detection.bbox.x * image_width;
    let mut y1 = detection.bbox.y * image_height;
    let mut x2 = (detection.bbox.x + detection.bbox.width) * image_width;
    let mut y2 = (detection.bbox.y + detection.bbox.height) * image_height;
    if let Some(landmarks) = &detection.landmarks {
        for [x, y] in &landmarks.points {
            x1 = x1.min(x * image_width);
            y1 = y1.min(y * image_height);
            x2 = x2.max(x * image_width);
            y2 = y2.max(y * image_height);
        }
    }
    let center_x = (x1 + x2) / 2.0;
    let center_y = (y1 + y2) / 2.0;
    let size = ((x2 - x1).max(y2 - y1) * 1.35).max(1.0);
    let x = (center_x - size / 2.0).clamp(0.0, (image_width - size).max(0.0));
    let y = (center_y - size / 2.0).clamp(0.0, (image_height - size).max(0.0));
    (x, y, size.min(image_width).min(image_height).max(1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::BTreeMap;

    use image_analysis_core::{OwnedImage, OwnedImageBatch};
    use image_analysis_detection::FaceDetectorBackend;
    use image_analysis_tasks::{FaceEmbedderBackend, ImageClassifierBackend, ImageEmbedderBackend};
    use model_runtime::{ModelBundleFile, ModelBundleManifest};
    use tempfile::tempdir;

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

    #[derive(Debug, Clone)]
    struct FakeEmbeddingRunner {
        outputs: Vec<OnnxF32Tensor>,
    }

    impl OnnxImageEmbeddingRunner for FakeEmbeddingRunner {
        fn run_image_embedding(&mut self, _input: &OnnxImageTensor) -> Result<Vec<OnnxF32Tensor>> {
            Ok(self.outputs.clone())
        }
    }

    #[derive(Debug, Clone)]
    struct FakeFaceDetectionRunner {
        output: OnnxFaceDetectionOutput,
    }

    impl OnnxFaceDetectionRunner for FakeFaceDetectionRunner {
        fn run_face_detection(
            &mut self,
            _input: &OnnxImageTensor,
        ) -> Result<OnnxFaceDetectionOutput> {
            Ok(self.output.clone())
        }
    }

    #[derive(Debug, Clone)]
    struct FakeFaceEmbeddingRunner {
        outputs: Vec<OnnxF32Tensor>,
    }

    impl OnnxFaceEmbeddingRunner for FakeFaceEmbeddingRunner {
        fn run_face_embedding(&mut self, _input: &OnnxImageTensor) -> Result<Vec<OnnxF32Tensor>> {
            Ok(self.outputs.clone())
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

    #[test]
    fn image_embedder_uses_fake_runner_and_normalizes_output() {
        let options = OnnxImageEmbeddingOptions {
            preprocessing: OnnxImageEmbeddingOptions::default().preprocessing,
            expected_vector_size: Some(2),
            normalize: true,
        };
        let mut embedder = OnnxImageEmbedder::with_options(
            options,
            FakeEmbeddingRunner {
                outputs: vec![OnnxF32Tensor::new(vec![1, 2], vec![3.0, 4.0]).unwrap()],
            },
        )
        .unwrap();
        let image = OwnedImage::new_rgb(2, 2, vec![255; 12]).unwrap();

        let embedding = embedder.embed_image(&image.as_view()).unwrap();

        assert_eq!(embedding.vector.len(), 2);
        assert!((embedding.vector[0] - 0.6).abs() < 0.0001);
        assert!((embedding.vector[1] - 0.8).abs() < 0.0001);
    }

    #[test]
    fn embedding_vector_selection_rejects_ambiguous_outputs() {
        let outputs = vec![
            OnnxF32Tensor::new(vec![2], vec![1.0, 0.0]).unwrap(),
            OnnxF32Tensor::new(vec![1, 2], vec![0.0, 1.0]).unwrap(),
        ];

        assert!(select_embedding_vector(&outputs, Some(2)).is_err());
    }

    #[test]
    fn face_detector_decodes_yunet_outputs_and_applies_nms() {
        let mut tensors = BTreeMap::new();
        for stride in [8_usize, 16, 32] {
            let count = (320_usize.div_ceil(stride)) * (320_usize.div_ceil(stride));
            let mut cls = vec![0.0; count];
            let mut obj = vec![0.0; count];
            let bbox = vec![0.0; count * 4];
            let kps = vec![0.0; count * 10];
            if stride == 8 {
                cls[0] = 0.95;
                obj[0] = 1.0;
                cls[1] = 0.94;
                obj[1] = 1.0;
            }
            tensors.insert(
                format!("cls_{stride}"),
                OnnxF32Tensor::new(vec![count], cls).unwrap(),
            );
            tensors.insert(
                format!("obj_{stride}"),
                OnnxF32Tensor::new(vec![count], obj).unwrap(),
            );
            tensors.insert(
                format!("bbox_{stride}"),
                OnnxF32Tensor::new(vec![count, 4], bbox).unwrap(),
            );
            tensors.insert(
                format!("kps_{stride}"),
                OnnxF32Tensor::new(vec![count, 10], kps).unwrap(),
            );
        }
        let options = OnnxFaceDetectionOptions {
            score_threshold: 0.9,
            top_k: 10,
            ..OnnxFaceDetectionOptions::default()
        };
        let mut detector = OnnxFaceDetector::with_options(
            options,
            FakeFaceDetectionRunner {
                output: OnnxFaceDetectionOutput { tensors },
            },
        )
        .unwrap();
        let image = OwnedImage::new_rgb(320, 320, vec![255; 320 * 320 * 3]).unwrap();

        let faces = detector.detect_faces(&image.as_view()).unwrap();

        assert_eq!(faces.len(), 2);
        assert!(faces[0].bbox.width > 0.0);
        assert_eq!(faces[0].landmarks.as_ref().unwrap().points.len(), 5);
    }

    #[test]
    fn face_embedder_uses_fake_runner_and_normalizes_output() {
        let mut embedder = OnnxFaceEmbedder::with_options(
            OnnxFaceEmbeddingOptions {
                expected_vector_size: Some(2),
                ..OnnxFaceEmbeddingOptions::default()
            },
            FakeFaceEmbeddingRunner {
                outputs: vec![OnnxF32Tensor::new(vec![1, 2], vec![5.0, 0.0]).unwrap()],
            },
        )
        .unwrap();
        let image = OwnedImage::new_rgb(24, 24, vec![255; 24 * 24 * 3]).unwrap();

        let embedding = embedder.embed_face(&image.as_view(), None).unwrap();

        assert_eq!(embedding.vector, vec![1.0, 0.0]);
    }
}
