use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::str::FromStr;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use hf_hub::api::sync::ApiBuilder;
use hf_hub::{Repo, RepoType};
use serde::{Deserialize, Serialize};
use video_analysis_core::{
    AnalysisEvent, BoundingBox, DetectError, Observation, ObservationKind, Result, TextAnalyzer,
    TextSegment, VideoAnalyzer, VideoFrame,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelTask {
    ObjectDetection,
    ImageClassification,
    TextClassification,
    TokenClassification,
    ZeroShotClassification,
    TextEmbedding,
    Custom(String),
}

impl ModelTask {
    pub fn default_kind(&self) -> ObservationKind {
        match self {
            Self::ObjectDetection => ObservationKind::Object,
            Self::ImageClassification => ObservationKind::Scene,
            Self::TextClassification | Self::TokenClassification | Self::ZeroShotClassification => {
                ObservationKind::Text
            }
            Self::TextEmbedding => ObservationKind::Custom("embedding".to_string()),
            Self::Custom(kind) => ObservationKind::Custom(kind.clone()),
        }
    }

    pub fn default_label(&self) -> &'static str {
        match self {
            Self::ObjectDetection => "object",
            Self::ImageClassification => "scene",
            Self::TextClassification => "semantic",
            Self::TokenClassification => "token",
            Self::ZeroShotClassification => "zero_shot",
            Self::TextEmbedding => "embedding",
            Self::Custom(_) => "custom",
        }
    }

    fn as_protocol_str(&self) -> &str {
        match self {
            Self::ObjectDetection => "object_detection",
            Self::ImageClassification => "image_classification",
            Self::TextClassification => "text_classification",
            Self::TokenClassification => "token_classification",
            Self::ZeroShotClassification => "zero_shot_classification",
            Self::TextEmbedding => "text_embedding",
            Self::Custom(kind) => kind.as_str(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelFileRequest {
    Required(String),
    Optional(String),
    FirstAvailable(Vec<String>),
}

impl ModelFileRequest {
    pub fn required(path: impl Into<String>) -> Self {
        Self::Required(path.into())
    }

    pub fn optional(path: impl Into<String>) -> Self {
        Self::Optional(path.into())
    }

    pub fn first_available(paths: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::FirstAvailable(paths.into_iter().map(Into::into).collect())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HuggingFaceModelSpec {
    pub name: String,
    pub repo_id: String,
    pub revision: String,
    pub task: ModelTask,
    pub files: Vec<ModelFileRequest>,
}

impl HuggingFaceModelSpec {
    pub fn new(repo_id: impl Into<String>, task: ModelTask) -> Self {
        let repo_id = repo_id.into();
        Self {
            name: repo_id.clone(),
            repo_id,
            revision: "main".to_string(),
            task,
            files: Vec::new(),
        }
    }

    pub fn from_preset(preset: ModelPreset) -> Self {
        preset.spec()
    }

    pub fn name(mut self, value: impl Into<String>) -> Self {
        self.name = value.into();
        self
    }

    pub fn revision(mut self, value: impl Into<String>) -> Self {
        self.revision = value.into();
        self
    }

    pub fn file(mut self, path: impl Into<String>) -> Self {
        self.files.push(ModelFileRequest::required(path));
        self
    }

    pub fn optional_file(mut self, path: impl Into<String>) -> Self {
        self.files.push(ModelFileRequest::optional(path));
        self
    }

    pub fn first_available_file(
        mut self,
        paths: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.files.push(ModelFileRequest::first_available(paths));
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelPreset {
    DetrResnet50,
    YolosTiny,
    DistilbertSst2,
    BertBaseNer,
    MiniLmL6V2,
}

impl ModelPreset {
    pub const ALL: &'static [Self] = &[
        Self::DetrResnet50,
        Self::YolosTiny,
        Self::DistilbertSst2,
        Self::BertBaseNer,
        Self::MiniLmL6V2,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::DetrResnet50 => "detr-resnet-50",
            Self::YolosTiny => "yolos-tiny",
            Self::DistilbertSst2 => "distilbert-sst2",
            Self::BertBaseNer => "bert-base-ner",
            Self::MiniLmL6V2 => "minilm-l6-v2",
        }
    }

    pub fn spec(self) -> HuggingFaceModelSpec {
        match self {
            Self::DetrResnet50 => {
                HuggingFaceModelSpec::new("facebook/detr-resnet-50", ModelTask::ObjectDetection)
                    .name(self.as_str())
                    .file("config.json")
                    .file("preprocessor_config.json")
                    .first_available_file(["model.safetensors", "pytorch_model.bin"])
            }
            Self::YolosTiny => {
                HuggingFaceModelSpec::new("hustvl/yolos-tiny", ModelTask::ObjectDetection)
                    .name(self.as_str())
                    .file("config.json")
                    .file("preprocessor_config.json")
                    .first_available_file(["model.safetensors", "pytorch_model.bin"])
            }
            Self::DistilbertSst2 => HuggingFaceModelSpec::new(
                "distilbert-base-uncased-finetuned-sst-2-english",
                ModelTask::TextClassification,
            )
            .name(self.as_str())
            .file("config.json")
            .file("tokenizer.json")
            .file("tokenizer_config.json")
            .file("vocab.txt")
            .first_available_file(["model.safetensors", "pytorch_model.bin"]),
            Self::BertBaseNer => {
                HuggingFaceModelSpec::new("dslim/bert-base-NER", ModelTask::TokenClassification)
                    .name(self.as_str())
                    .file("config.json")
                    .file("tokenizer_config.json")
                    .file("vocab.txt")
                    .optional_file("tokenizer.json")
                    .first_available_file(["model.safetensors", "pytorch_model.bin"])
            }
            Self::MiniLmL6V2 => HuggingFaceModelSpec::new(
                "sentence-transformers/all-MiniLM-L6-v2",
                ModelTask::TextEmbedding,
            )
            .name(self.as_str())
            .file("config.json")
            .file("tokenizer.json")
            .file("tokenizer_config.json")
            .file("vocab.txt")
            .file("modules.json")
            .optional_file("sentence_bert_config.json")
            .first_available_file(["model.safetensors", "pytorch_model.bin"]),
        }
    }
}

impl FromStr for ModelPreset {
    type Err = DetectError;

    fn from_str(input: &str) -> Result<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|preset| preset.as_str() == input)
            .ok_or_else(|| {
                DetectError::InvalidArgument(format!(
                    "unknown model preset `{input}`; expected one of {}",
                    Self::ALL
                        .iter()
                        .map(|preset| preset.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
            })
    }
}

#[derive(Debug, Clone)]
pub struct DownloadedModel {
    pub spec: HuggingFaceModelSpec,
    pub files: BTreeMap<String, PathBuf>,
}

impl DownloadedModel {
    pub fn model_dir(&self) -> Option<&Path> {
        self.files.values().next().and_then(|path| path.parent())
    }
}

#[derive(Debug, Clone)]
pub struct HuggingFaceDownloader {
    cache_dir: Option<PathBuf>,
    token: Option<String>,
    progress: bool,
    max_retries: usize,
}

impl Default for HuggingFaceDownloader {
    fn default() -> Self {
        Self {
            cache_dir: None,
            token: None,
            progress: true,
            max_retries: 0,
        }
    }
}

impl HuggingFaceDownloader {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cache_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.cache_dir = Some(path.into());
        self
    }

    pub fn token(mut self, value: impl Into<String>) -> Self {
        self.token = Some(value.into());
        self
    }

    pub fn progress(mut self, value: bool) -> Self {
        self.progress = value;
        self
    }

    pub fn max_retries(mut self, value: usize) -> Self {
        self.max_retries = value;
        self
    }

    pub fn download(&self, spec: &HuggingFaceModelSpec) -> Result<DownloadedModel> {
        if spec.files.is_empty() {
            return Err(DetectError::InvalidArgument(
                "at least one model file must be requested".to_string(),
            ));
        }

        let mut builder = ApiBuilder::from_env()
            .with_progress(self.progress)
            .with_retries(self.max_retries)
            .with_user_agent("video-analysis", env!("CARGO_PKG_VERSION"));
        if let Some(cache_dir) = &self.cache_dir {
            builder = builder.with_cache_dir(cache_dir.clone());
        }
        if self.token.is_some() {
            builder = builder.with_token(self.token.clone());
        }

        let api = builder
            .build()
            .map_err(|err| DetectError::Source(format!("huggingface api error: {err}")))?;
        let repo = api.repo(Repo::with_revision(
            spec.repo_id.clone(),
            RepoType::Model,
            spec.revision.clone(),
        ));

        let mut files = BTreeMap::new();
        for request in &spec.files {
            match request {
                ModelFileRequest::Required(path) => {
                    let local = repo.get(path).map_err(|err| {
                        DetectError::Source(format!(
                            "failed to download `{path}` from `{}`: {err}",
                            spec.repo_id
                        ))
                    })?;
                    files.insert(path.clone(), local);
                }
                ModelFileRequest::Optional(path) => {
                    if let Ok(local) = repo.get(path) {
                        files.insert(path.clone(), local);
                    }
                }
                ModelFileRequest::FirstAvailable(paths) => {
                    let mut last_error = None;
                    let mut found = None;
                    for path in paths {
                        match repo.get(path) {
                            Ok(local) => {
                                found = Some((path.clone(), local));
                                break;
                            }
                            Err(err) => last_error = Some(err.to_string()),
                        }
                    }
                    if let Some((path, local)) = found {
                        files.insert(path, local);
                    } else {
                        return Err(DetectError::Source(format!(
                            "none of the alternative files [{}] could be downloaded from `{}`{}",
                            paths.join(", "),
                            spec.repo_id,
                            last_error
                                .map(|err| format!("; last error: {err}"))
                                .unwrap_or_default()
                        )));
                    }
                }
            }
        }

        Ok(DownloadedModel {
            spec: spec.clone(),
            files,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RawPrediction {
    pub kind: Option<String>,
    pub label: Option<String>,
    pub text: Option<String>,
    pub score: Option<f32>,
    pub region: Option<RawBoundingBox>,
    #[serde(default)]
    pub attributes: BTreeMap<String, String>,
}

impl RawPrediction {
    pub fn object(label: impl Into<String>, score: f32, region: RawBoundingBox) -> Self {
        Self {
            kind: Some("object".to_string()),
            label: Some(label.into()),
            score: Some(score),
            region: Some(region),
            ..Self::default()
        }
    }

    pub fn label(label: impl Into<String>, score: f32) -> Self {
        Self {
            label: Some(label.into()),
            score: Some(score),
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct RawBoundingBox {
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub xmin: Option<f32>,
    pub ymin: Option<f32>,
    pub xmax: Option<f32>,
    pub ymax: Option<f32>,
    #[serde(default)]
    pub normalized: bool,
}

impl RawBoundingBox {
    pub fn xywh(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x: Some(x),
            y: Some(y),
            width: Some(width),
            height: Some(height),
            ..Self::default()
        }
    }

    pub fn xyxy(xmin: f32, ymin: f32, xmax: f32, ymax: f32) -> Self {
        Self {
            xmin: Some(xmin),
            ymin: Some(ymin),
            xmax: Some(xmax),
            ymax: Some(ymax),
            ..Self::default()
        }
    }

    fn to_bounding_box(self, dimensions: Option<(u32, u32)>, clamp: bool) -> Option<BoundingBox> {
        let (mut x0, mut y0, mut x1, mut y1) =
            if let (Some(x), Some(y), Some(width), Some(height)) =
                (self.x, self.y, self.width, self.height)
            {
                (x, y, x + width, y + height)
            } else if let (Some(xmin), Some(ymin), Some(xmax), Some(ymax)) =
                (self.xmin, self.ymin, self.xmax, self.ymax)
            {
                (xmin, ymin, xmax, ymax)
            } else {
                return None;
            };

        if self.normalized
            || [x0, y0, x1, y1]
                .into_iter()
                .all(|value| (0.0..=1.0).contains(&value))
        {
            let (width, height) = dimensions?;
            let width = width as f32;
            let height = height as f32;
            x0 *= width;
            x1 *= width;
            y0 *= height;
            y1 *= height;
        }

        if clamp {
            if let Some((width, height)) = dimensions {
                x0 = x0.clamp(0.0, width as f32);
                x1 = x1.clamp(0.0, width as f32);
                y0 = y0.clamp(0.0, height as f32);
                y1 = y1.clamp(0.0, height as f32);
            }
        }

        if x1 <= x0 || y1 <= y0 {
            return None;
        }
        let epsilon = 0.0001;
        let x = (x0 + epsilon).floor().max(0.0) as u32;
        let y = (y0 + epsilon).floor().max(0.0) as u32;
        let width = ((x1 - epsilon).ceil() as u32).saturating_sub(x);
        let height = ((y1 - epsilon).ceil() as u32).saturating_sub(y);
        BoundingBox::new(x, y, width, height).ok()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NormalizedPrediction {
    pub kind: ObservationKind,
    pub label: Option<String>,
    pub text: Option<String>,
    pub score: Option<f32>,
    pub region: Option<BoundingBox>,
    pub attributes: BTreeMap<String, String>,
}

impl NormalizedPrediction {
    pub fn to_observation(&self, analyzer: impl Into<String>) -> Observation {
        let mut observation = Observation::new(analyzer, self.kind.clone());
        if let Some(label) = &self.label {
            observation = observation.label(label.clone());
        }
        if let Some(text) = &self.text {
            observation = observation.text(text.clone());
        }
        if let Some(score) = self.score {
            observation = observation.score(score);
        }
        if let Some(region) = self.region {
            observation = observation.region(region);
        }
        for (key, value) in &self.attributes {
            observation = observation.attribute(key.clone(), value.clone());
        }
        observation
    }

    pub fn to_event(&self, analyzer: impl Into<String>) -> AnalysisEvent {
        let label = self
            .label
            .clone()
            .or_else(|| self.text.clone())
            .unwrap_or_else(|| "prediction".to_string());
        let mut event = AnalysisEvent::new(analyzer, label);
        if let Some(score) = self.score {
            event = event.score(score);
        }
        event
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PredictionRepairOptions {
    pub min_score: Option<f32>,
    pub clamp_regions: bool,
    pub nms_iou_threshold: Option<f32>,
    pub fill_missing_labels: bool,
}

impl Default for PredictionRepairOptions {
    fn default() -> Self {
        Self {
            min_score: None,
            clamp_regions: true,
            nms_iou_threshold: Some(0.5),
            fill_missing_labels: true,
        }
    }
}

pub fn normalize_predictions(
    raw: Vec<RawPrediction>,
    task: &ModelTask,
    dimensions: Option<(u32, u32)>,
    repair: PredictionRepairOptions,
) -> Vec<NormalizedPrediction> {
    let mut predictions = raw
        .into_iter()
        .filter(|prediction| {
            repair
                .min_score
                .zip(prediction.score)
                .map(|(minimum, score)| score >= minimum)
                .unwrap_or(true)
        })
        .map(|prediction| normalize_prediction(prediction, task, dimensions, repair))
        .collect::<Vec<_>>();

    if let Some(threshold) = repair.nms_iou_threshold {
        predictions = non_max_suppression(predictions, threshold);
    }
    predictions
}

fn normalize_prediction(
    prediction: RawPrediction,
    task: &ModelTask,
    dimensions: Option<(u32, u32)>,
    repair: PredictionRepairOptions,
) -> NormalizedPrediction {
    let kind = prediction
        .kind
        .as_deref()
        .map(kind_from_str)
        .unwrap_or_else(|| task.default_kind());
    let label = prediction.label.or_else(|| {
        repair
            .fill_missing_labels
            .then(|| task.default_label().to_string())
    });
    let region = prediction
        .region
        .and_then(|region| region.to_bounding_box(dimensions, repair.clamp_regions));

    NormalizedPrediction {
        kind,
        label,
        text: prediction.text,
        score: prediction.score,
        region,
        attributes: prediction.attributes,
    }
}

fn kind_from_str(kind: &str) -> ObservationKind {
    match kind {
        "object" | "detection" | "object_detection" => ObservationKind::Object,
        "text" | "ocr" | "token" | "text_classification" => ObservationKind::Text,
        "face" => ObservationKind::Face,
        "scene" | "image_classification" => ObservationKind::Scene,
        other => ObservationKind::Custom(other.to_string()),
    }
}

fn non_max_suppression(
    mut predictions: Vec<NormalizedPrediction>,
    threshold: f32,
) -> Vec<NormalizedPrediction> {
    predictions.sort_by(|left, right| {
        right
            .score
            .unwrap_or(0.0)
            .total_cmp(&left.score.unwrap_or(0.0))
    });
    let mut kept: Vec<NormalizedPrediction> = Vec::new();
    'candidate: for prediction in predictions {
        if let Some(region) = prediction.region {
            for existing in &kept {
                if existing.region.is_some()
                    && existing.kind == prediction.kind
                    && existing.label == prediction.label
                    && bbox_iou(existing.region.unwrap(), region) > threshold
                {
                    continue 'candidate;
                }
            }
        }
        kept.push(prediction);
    }
    kept
}

fn bbox_iou(left: BoundingBox, right: BoundingBox) -> f32 {
    let left_x1 = left.x + left.width;
    let left_y1 = left.y + left.height;
    let right_x1 = right.x + right.width;
    let right_y1 = right.y + right.height;

    let ix0 = left.x.max(right.x);
    let iy0 = left.y.max(right.y);
    let ix1 = left_x1.min(right_x1);
    let iy1 = left_y1.min(right_y1);
    if ix1 <= ix0 || iy1 <= iy0 {
        return 0.0;
    }
    let intersection = (ix1 - ix0) as f32 * (iy1 - iy0) as f32;
    let left_area = left.width as f32 * left.height as f32;
    let right_area = right.width as f32 * right.height as f32;
    intersection / (left_area + right_area - intersection)
}

pub trait VisionModelBackend {
    fn task(&self) -> ModelTask;
    fn predict_frame(&mut self, frame: &VideoFrame<'_>) -> Result<Vec<RawPrediction>>;
}

pub trait TextModelBackend {
    fn task(&self) -> ModelTask;
    fn predict_text(&mut self, segment: &TextSegment<'_>) -> Result<Vec<RawPrediction>>;
}

pub struct ModelVideoAnalyzer<B> {
    name: String,
    backend: B,
    repair: PredictionRepairOptions,
}

impl<B> ModelVideoAnalyzer<B> {
    pub fn new(name: impl Into<String>, backend: B) -> Self {
        Self {
            name: name.into(),
            backend,
            repair: PredictionRepairOptions::default(),
        }
    }

    pub fn repair_options(mut self, value: PredictionRepairOptions) -> Self {
        self.repair = value;
        self
    }

    pub fn backend(&self) -> &B {
        &self.backend
    }

    pub fn backend_mut(&mut self) -> &mut B {
        &mut self.backend
    }
}

impl<B: VisionModelBackend> VideoAnalyzer for ModelVideoAnalyzer<B> {
    fn name(&self) -> &str {
        &self.name
    }

    fn process_frame(&mut self, frame: &VideoFrame<'_>) -> Result<Vec<Observation>> {
        let task = self.backend.task();
        let raw = self.backend.predict_frame(frame)?;
        Ok(
            normalize_predictions(raw, &task, Some((frame.width, frame.height)), self.repair)
                .into_iter()
                .map(|prediction| prediction.to_observation(self.name()))
                .collect(),
        )
    }
}

pub struct ModelTextAnalyzer<B> {
    name: String,
    backend: B,
    repair: PredictionRepairOptions,
}

impl<B> ModelTextAnalyzer<B> {
    pub fn new(name: impl Into<String>, backend: B) -> Self {
        Self {
            name: name.into(),
            backend,
            repair: PredictionRepairOptions::default(),
        }
    }

    pub fn repair_options(mut self, value: PredictionRepairOptions) -> Self {
        self.repair = value;
        self
    }

    pub fn backend(&self) -> &B {
        &self.backend
    }

    pub fn backend_mut(&mut self) -> &mut B {
        &mut self.backend
    }
}

impl<B: TextModelBackend> TextAnalyzer for ModelTextAnalyzer<B> {
    fn name(&self) -> &str {
        &self.name
    }

    fn process_segment(&mut self, segment: &TextSegment<'_>) -> Result<Vec<AnalysisEvent>> {
        let task = self.backend.task();
        let raw = self.backend.predict_text(segment)?;
        Ok(normalize_predictions(raw, &task, None, self.repair)
            .into_iter()
            .map(|prediction| {
                let mut event = prediction.to_event(self.name());
                if let Some(timestamp) = segment.timestamp {
                    event = event.at_timestamp(timestamp);
                }
                event
            })
            .collect())
    }
}

#[derive(Debug, Clone)]
pub struct ExternalCommandModel {
    command: PathBuf,
    args: Vec<String>,
    model: DownloadedModel,
}

impl ExternalCommandModel {
    pub fn new(command: impl Into<PathBuf>, model: DownloadedModel) -> Self {
        Self {
            command: command.into(),
            args: Vec::new(),
            model,
        }
    }

    pub fn arg(mut self, value: impl Into<String>) -> Self {
        self.args.push(value.into());
        self
    }

    pub fn args(mut self, values: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.args.extend(values.into_iter().map(Into::into));
        self
    }

    pub fn model(&self) -> &DownloadedModel {
        &self.model
    }

    pub fn persistent(self) -> PersistentExternalCommandModel {
        PersistentExternalCommandModel::new(self.command, self.model).args(self.args)
    }

    fn run(&mut self, input: ExternalModelInput<'_>) -> Result<Vec<RawPrediction>> {
        let request = external_model_request(&self.model, input);
        let payload = serde_json::to_vec(&request)
            .map_err(|err| DetectError::Source(format!("failed to encode model request: {err}")))?;

        let mut child = Command::new(&self.command)
            .args(&self.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| DetectError::Source("model command stdin is unavailable".to_string()))?;
        stdin.write_all(&payload)?;
        drop(stdin);

        let output = child.wait_with_output()?;
        if !output.status.success() {
            return Err(DetectError::Source(format!(
                "model command `{}` failed with status {}: {}",
                self.command.display(),
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }

        let response: ExternalModelResponse =
            serde_json::from_slice(&output.stdout).map_err(|err| {
                DetectError::Source(format!(
                    "model command `{}` returned invalid JSON: {err}",
                    self.command.display()
                ))
            })?;
        Ok(response.predictions)
    }
}

pub struct PersistentExternalCommandModel {
    command: PathBuf,
    args: Vec<String>,
    model: DownloadedModel,
    child: Option<PersistentCommandChild>,
}

struct PersistentCommandChild {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl PersistentExternalCommandModel {
    pub fn new(command: impl Into<PathBuf>, model: DownloadedModel) -> Self {
        Self {
            command: command.into(),
            args: Vec::new(),
            model,
            child: None,
        }
    }

    pub fn arg(mut self, value: impl Into<String>) -> Self {
        self.args.push(value.into());
        self
    }

    pub fn args(mut self, values: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.args.extend(values.into_iter().map(Into::into));
        self
    }

    pub fn model(&self) -> &DownloadedModel {
        &self.model
    }

    pub fn stop(&mut self) -> Result<()> {
        if let Some(mut child) = self.child.take() {
            drop(child.stdin);
            let status = child.child.wait()?;
            if !status.success() {
                return Err(DetectError::Source(format!(
                    "persistent model command `{}` exited with status {status}",
                    self.command.display()
                )));
            }
        }
        Ok(())
    }

    fn run(&mut self, input: ExternalModelInput<'_>) -> Result<Vec<RawPrediction>> {
        let request = external_model_request(&self.model, input);
        let payload = serde_json::to_vec(&request)
            .map_err(|err| DetectError::Source(format!("failed to encode model request: {err}")))?;
        let command = self.command.display().to_string();
        let child = self.child()?;

        child.stdin.write_all(&payload)?;
        child.stdin.write_all(b"\n")?;
        child.stdin.flush()?;

        let mut line = String::new();
        let bytes = child.stdout.read_line(&mut line)?;
        if bytes == 0 {
            return Err(DetectError::Source(format!(
                "persistent model command `{command}` closed stdout"
            )));
        }
        let response: ExternalModelResponse = serde_json::from_str(&line).map_err(|err| {
            DetectError::Source(format!(
                "persistent model command `{command}` returned invalid JSON: {err}"
            ))
        })?;
        Ok(response.predictions)
    }

    fn child(&mut self) -> Result<&mut PersistentCommandChild> {
        if self.child.is_none() {
            let mut child = Command::new(&self.command)
                .args(&self.args)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()?;
            let stdin = child.stdin.take().ok_or_else(|| {
                DetectError::Source("persistent model command stdin is unavailable".to_string())
            })?;
            let stdout = child.stdout.take().ok_or_else(|| {
                DetectError::Source("persistent model command stdout is unavailable".to_string())
            })?;
            self.child = Some(PersistentCommandChild {
                child,
                stdin,
                stdout: BufReader::new(stdout),
            });
        }
        Ok(self.child.as_mut().expect("persistent model child exists"))
    }
}

impl Drop for PersistentExternalCommandModel {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.child.kill();
            let _ = child.child.wait();
        }
    }
}

impl VisionModelBackend for ExternalCommandModel {
    fn task(&self) -> ModelTask {
        self.model.spec.task.clone()
    }

    fn predict_frame(&mut self, frame: &VideoFrame<'_>) -> Result<Vec<RawPrediction>> {
        self.run(ExternalModelInput::VideoFrame {
            width: frame.width,
            height: frame.height,
            pixel_format: match frame.pixel_format {
                video_analysis_core::PixelFormat::Rgb24 => "rgb24",
                video_analysis_core::PixelFormat::Bgr24 => "bgr24",
            },
            stride: frame.stride,
            data_base64: BASE64.encode(frame.data),
        })
    }
}

impl VisionModelBackend for PersistentExternalCommandModel {
    fn task(&self) -> ModelTask {
        self.model.spec.task.clone()
    }

    fn predict_frame(&mut self, frame: &VideoFrame<'_>) -> Result<Vec<RawPrediction>> {
        self.run(ExternalModelInput::VideoFrame {
            width: frame.width,
            height: frame.height,
            pixel_format: match frame.pixel_format {
                video_analysis_core::PixelFormat::Rgb24 => "rgb24",
                video_analysis_core::PixelFormat::Bgr24 => "bgr24",
            },
            stride: frame.stride,
            data_base64: BASE64.encode(frame.data),
        })
    }
}

impl TextModelBackend for ExternalCommandModel {
    fn task(&self) -> ModelTask {
        self.model.spec.task.clone()
    }

    fn predict_text(&mut self, segment: &TextSegment<'_>) -> Result<Vec<RawPrediction>> {
        self.run(ExternalModelInput::Text {
            text: segment.text,
            language: segment.language,
            is_final: segment.is_final,
        })
    }
}

impl TextModelBackend for PersistentExternalCommandModel {
    fn task(&self) -> ModelTask {
        self.model.spec.task.clone()
    }

    fn predict_text(&mut self, segment: &TextSegment<'_>) -> Result<Vec<RawPrediction>> {
        self.run(ExternalModelInput::Text {
            text: segment.text,
            language: segment.language,
            is_final: segment.is_final,
        })
    }
}

fn external_model_request<'a>(
    model: &'a DownloadedModel,
    input: ExternalModelInput<'a>,
) -> ExternalModelRequest<'a> {
    ExternalModelRequest {
        task: model.spec.task.as_protocol_str(),
        model: ExternalModelInfo {
            name: &model.spec.name,
            repo_id: &model.spec.repo_id,
            revision: &model.spec.revision,
            files: model
                .files
                .iter()
                .map(|(key, path)| (key.as_str(), path.to_string_lossy().into_owned()))
                .collect(),
        },
        input,
    }
}

#[derive(Debug, Serialize)]
struct ExternalModelRequest<'a> {
    task: &'a str,
    model: ExternalModelInfo<'a>,
    input: ExternalModelInput<'a>,
}

#[derive(Debug, Serialize)]
struct ExternalModelInfo<'a> {
    name: &'a str,
    repo_id: &'a str,
    revision: &'a str,
    files: BTreeMap<&'a str, String>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ExternalModelInput<'a> {
    VideoFrame {
        width: u32,
        height: u32,
        pixel_format: &'static str,
        stride: usize,
        data_base64: String,
    },
    Text {
        text: &'a str,
        language: Option<&'a str>,
        is_final: bool,
    },
}

#[derive(Debug, Deserialize)]
struct ExternalModelResponse {
    #[serde(default)]
    predictions: Vec<RawPrediction>,
}

#[cfg(test)]
mod tests {
    use num_rational::Rational64;
    use video_analysis_core::{
        FramePosition, OwnedVideoFrame, PixelFormat, TextSegment, Timestamp,
    };

    use super::*;

    fn test_frame() -> OwnedVideoFrame {
        OwnedVideoFrame {
            position: FramePosition::from_frame_index(0, Rational64::new(30, 1)),
            width: 100,
            height: 50,
            pixel_format: PixelFormat::Rgb24,
            data: vec![0; 100 * 50 * 3],
            stride: 100 * 3,
        }
    }

    #[test]
    fn preset_specs_include_weight_fallbacks() {
        let spec = ModelPreset::DetrResnet50.spec();
        assert_eq!(spec.repo_id, "facebook/detr-resnet-50");
        assert_eq!(spec.task, ModelTask::ObjectDetection);
        assert!(spec
            .files
            .iter()
            .any(|file| matches!(file, ModelFileRequest::FirstAvailable(_))));
    }

    #[test]
    fn raw_boxes_are_clamped_and_normalized() {
        let raw = vec![RawPrediction::object(
            "person",
            0.9,
            RawBoundingBox {
                xmin: Some(-0.1),
                ymin: Some(0.1),
                xmax: Some(1.2),
                ymax: Some(0.6),
                normalized: true,
                ..RawBoundingBox::default()
            },
        )];

        let predictions = normalize_predictions(
            raw,
            &ModelTask::ObjectDetection,
            Some((100, 50)),
            PredictionRepairOptions::default(),
        );

        assert_eq!(predictions.len(), 1);
        assert_eq!(
            predictions[0].region,
            Some(BoundingBox::new(0, 5, 100, 25).unwrap())
        );
    }

    #[test]
    fn nms_removes_overlapping_same_label_boxes() {
        let raw = vec![
            RawPrediction::object("person", 0.9, RawBoundingBox::xywh(0.0, 0.0, 10.0, 10.0)),
            RawPrediction::object("person", 0.8, RawBoundingBox::xywh(1.0, 1.0, 10.0, 10.0)),
            RawPrediction::object("car", 0.7, RawBoundingBox::xywh(1.0, 1.0, 10.0, 10.0)),
        ];

        let predictions = normalize_predictions(
            raw,
            &ModelTask::ObjectDetection,
            Some((100, 100)),
            PredictionRepairOptions::default(),
        );

        assert_eq!(predictions.len(), 2);
        assert_eq!(predictions[0].label.as_deref(), Some("person"));
        assert_eq!(predictions[1].label.as_deref(), Some("car"));
    }

    #[test]
    fn model_video_analyzer_emits_observations() {
        struct StaticVisionBackend;

        impl VisionModelBackend for StaticVisionBackend {
            fn task(&self) -> ModelTask {
                ModelTask::ObjectDetection
            }

            fn predict_frame(&mut self, _frame: &VideoFrame<'_>) -> Result<Vec<RawPrediction>> {
                Ok(vec![RawPrediction::object(
                    "car",
                    0.8,
                    RawBoundingBox::xywh(10.0, 10.0, 20.0, 20.0),
                )])
            }
        }

        let frame = test_frame();
        let mut analyzer = ModelVideoAnalyzer::new("objects", StaticVisionBackend);
        let observations = analyzer.process_frame(&frame.as_frame()).unwrap();

        assert_eq!(observations.len(), 1);
        assert_eq!(observations[0].analyzer, "objects");
        assert_eq!(observations[0].label.as_deref(), Some("car"));
        assert_eq!(observations[0].kind, ObservationKind::Object);
    }

    #[test]
    fn model_text_analyzer_emits_dynamic_labels() {
        struct StaticTextBackend;

        impl TextModelBackend for StaticTextBackend {
            fn task(&self) -> ModelTask {
                ModelTask::TextClassification
            }

            fn predict_text(&mut self, _segment: &TextSegment<'_>) -> Result<Vec<RawPrediction>> {
                Ok(vec![RawPrediction::label("POSITIVE", 0.99)])
            }
        }

        let mut analyzer = ModelTextAnalyzer::new("sentiment", StaticTextBackend);
        let segment = TextSegment {
            segment_index: 1,
            timestamp: Some(Timestamp::new(
                30,
                video_analysis_core::Timebase::new(1, 30),
            )),
            text: "works well",
            language: Some("en"),
            is_final: true,
        };

        let events = analyzer.process_segment(&segment).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].analyzer, "sentiment");
        assert_eq!(events[0].label, "POSITIVE");
        assert_eq!(events[0].score, Some(0.99));
        assert_eq!(events[0].timestamp, segment.timestamp);
    }

    #[test]
    fn persistent_external_command_reuses_process_for_text_predictions() {
        let model = DownloadedModel {
            spec: HuggingFaceModelSpec::new("test-model", ModelTask::TextClassification),
            files: BTreeMap::new(),
        };
        let script =
            "while IFS= read -r line; do printf '%s\\n' '{\"predictions\":[{\"label\":\"ok\",\"score\":0.5}]}'; done";
        let mut backend = PersistentExternalCommandModel::new("sh", model)
            .arg("-c")
            .arg(script);
        let segment = TextSegment {
            segment_index: 0,
            timestamp: None,
            text: "hello",
            language: Some("en"),
            is_final: true,
        };

        let first = backend.predict_text(&segment).unwrap();
        let second = backend.predict_text(&segment).unwrap();
        backend.stop().unwrap();

        assert_eq!(first[0].label.as_deref(), Some("ok"));
        assert_eq!(second[0].score, Some(0.5));
    }
}
