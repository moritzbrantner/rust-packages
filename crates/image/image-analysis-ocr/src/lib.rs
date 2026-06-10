#![doc = include_str!("../README.md")]

pub mod surface;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;
use std::str::FromStr;

use image_analysis_core::{compact_image, ImageView};
use model_runtime::{HuggingFaceModelSpec, ModelRuntimeBackend, ModelTask};
use serde::{Deserialize, Serialize};
use video_analysis_core::{BoundingBox, DetectError, FramePosition, Result, VideoFrame};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// Variants describing OCR model presets.
pub enum OcrPreset {
    #[default]
    /// Printed document text recognition with TrOCR.
    TrOcrBasePrinted,
    /// Handwritten text recognition with TrOCR.
    TrOcrBaseHandwritten,
    /// OCR-focused document understanding with Donut on CORD receipts.
    DonutBaseCordV2,
    /// Document transcription model for academic/scientific pages.
    NougatBase,
}

impl OcrPreset {
    /// Constant for all.
    pub const ALL: &'static [Self] = &[
        Self::TrOcrBasePrinted,
        Self::TrOcrBaseHandwritten,
        Self::DonutBaseCordV2,
        Self::NougatBase,
    ];

    /// Borrows this value as a str.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TrOcrBasePrinted => "trocr-base-printed",
            Self::TrOcrBaseHandwritten => "trocr-base-handwritten",
            Self::DonutBaseCordV2 => "donut-base-cord-v2",
            Self::NougatBase => "nougat-base",
        }
    }

    /// Returns repo identifier.
    pub fn repo_id(self) -> &'static str {
        match self {
            Self::TrOcrBasePrinted => "microsoft/trocr-base-printed",
            Self::TrOcrBaseHandwritten => "microsoft/trocr-base-handwritten",
            Self::DonutBaseCordV2 => "naver-clova-ix/donut-base-finetuned-cord-v2",
            Self::NougatBase => "facebook/nougat-base",
        }
    }

    /// Returns model spec.
    pub fn model_spec(self) -> HuggingFaceModelSpec {
        let base = HuggingFaceModelSpec::new(
            self.repo_id(),
            ModelTask::Custom("optical_character_recognition".to_string()),
        )
        .name(self.as_str())
        .file("config.json")
        .file("preprocessor_config.json")
        .first_available_file(["model.safetensors", "pytorch_model.bin"]);

        match self {
            Self::TrOcrBasePrinted | Self::TrOcrBaseHandwritten => base
                .file("tokenizer.json")
                .file("vocab.json")
                .file("merges.txt"),
            Self::DonutBaseCordV2 => base
                .file("tokenizer.json")
                .optional_file("sentencepiece.bpe.model"),
            Self::NougatBase => base
                .file("tokenizer.json")
                .optional_file("tokenizer_config.json")
                .optional_file("special_tokens_map.json"),
        }
    }
}

impl FromStr for OcrPreset {
    type Err = DetectError;

    fn from_str(input: &str) -> Result<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|preset| preset.as_str() == input)
            .ok_or_else(|| {
                DetectError::InvalidArgument(format!(
                    "unknown OCR preset `{input}`; expected one of {}",
                    Self::ALL
                        .iter()
                        .map(|preset| preset.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
            })
    }
}

/// Returns default OCR model spec.
pub fn default_ocr_model_spec() -> HuggingFaceModelSpec {
    OcrPreset::default().model_spec()
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Variants describing OCR technique families.
pub enum OcrTechnique {
    /// A Hugging Face model-backed recognizer.
    HuggingFaceModel(HuggingFaceModelSpec),
    /// A local or remote command-style adapter such as Tesseract, EasyOCR, or PaddleOCR.
    ExternalCommand(ExternalOcrCommandSpec),
    /// A deterministic recognizer, usually for tests or simple threshold/layout passes.
    Heuristic,
    /// A custom integration.
    Custom(String),
}

impl OcrTechnique {
    /// Returns a compact technique kind.
    pub fn kind(&self) -> &str {
        match self {
            Self::HuggingFaceModel(_) => "huggingface_model",
            Self::ExternalCommand(_) => "external_command",
            Self::Heuristic => "heuristic",
            Self::Custom(value) => value.as_str(),
        }
    }

    /// Returns the model runtime backend usually associated with this technique.
    pub fn runtime_backend(&self) -> ModelRuntimeBackend {
        match self {
            Self::HuggingFaceModel(_) => ModelRuntimeBackend::Candle,
            Self::ExternalCommand(_) => ModelRuntimeBackend::External,
            Self::Heuristic => ModelRuntimeBackend::Heuristic,
            Self::Custom(value) => ModelRuntimeBackend::Custom(value.clone()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Data type for external OCR command integration metadata.
pub struct ExternalOcrCommandSpec {
    /// Human-readable name.
    pub name: String,
    /// Executable or service command.
    pub command: String,
    /// Default command arguments.
    pub args: Vec<String>,
    /// Attributes for adapter-specific metadata.
    pub attributes: BTreeMap<String, String>,
}

impl ExternalOcrCommandSpec {
    /// Creates a new value.
    pub fn new(name: impl Into<String>, command: impl Into<String>) -> Result<Self> {
        let name = name.into();
        let command = command.into();
        if name.trim().is_empty() || command.trim().is_empty() {
            return Err(DetectError::InvalidArgument(
                "external OCR command name and command are required".to_string(),
            ));
        }
        Ok(Self {
            name,
            command,
            args: Vec::new(),
            attributes: BTreeMap::new(),
        })
    }

    /// Returns arg.
    pub fn arg(mut self, value: impl Into<String>) -> Self {
        self.args.push(value.into());
        self
    }

    /// Returns attribute.
    pub fn attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Data type for OCR request options.
pub struct OcrRequest {
    /// Optional technique override.
    pub technique: Option<OcrTechnique>,
    /// Preferred languages, using BCP-47 or engine-specific short codes.
    pub languages: Vec<String>,
    /// Whether recognizers should preserve line/block/page layout when possible.
    pub preserve_layout: bool,
    /// Whether recognizers should include token-level spans when possible.
    pub include_tokens: bool,
    /// Minimum confidence accepted in post-processing.
    pub min_confidence: Option<u8>,
    /// Attributes for adapter-specific options.
    pub attributes: BTreeMap<String, String>,
}

impl Default for OcrRequest {
    fn default() -> Self {
        Self {
            technique: Some(OcrTechnique::HuggingFaceModel(default_ocr_model_spec())),
            languages: Vec::new(),
            preserve_layout: true,
            include_tokens: true,
            min_confidence: None,
            attributes: BTreeMap::new(),
        }
    }
}

impl OcrRequest {
    /// Creates a new value.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns technique.
    pub fn technique(mut self, value: OcrTechnique) -> Self {
        self.technique = Some(value);
        self
    }

    /// Returns model preset.
    pub fn model_preset(mut self, preset: OcrPreset) -> Self {
        self.technique = Some(OcrTechnique::HuggingFaceModel(preset.model_spec()));
        self
    }

    /// Returns languages.
    pub fn languages(mut self, values: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.languages = values.into_iter().map(Into::into).collect();
        self
    }

    /// Returns preserve layout.
    pub fn preserve_layout(mut self, value: bool) -> Self {
        self.preserve_layout = value;
        self
    }

    /// Returns include tokens.
    pub fn include_tokens(mut self, value: bool) -> Self {
        self.include_tokens = value;
        self
    }

    /// Returns min confidence.
    pub fn min_confidence(mut self, value: u8) -> Self {
        self.min_confidence = Some(value.min(100));
        self
    }

    /// Returns attribute.
    pub fn attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
/// Data type for OCR confidence as a 0-100 score.
pub struct OcrConfidence(u8);

impl OcrConfidence {
    /// Creates a new value.
    pub fn new(value: u8) -> Self {
        Self(value.min(100))
    }

    /// Returns raw value.
    pub fn value(self) -> u8 {
        self.0
    }
}

impl From<u8> for OcrConfidence {
    fn from(value: u8) -> Self {
        Self::new(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Data type for OCR text token.
pub struct OcrToken {
    /// Text content.
    pub text: String,
    /// Optional region in image coordinates.
    pub region: Option<BoundingBox>,
    /// Optional confidence.
    pub confidence: Option<OcrConfidence>,
    /// Attributes for adapter-specific metadata.
    pub attributes: BTreeMap<String, String>,
}

impl OcrToken {
    /// Creates a new value.
    pub fn new(text: impl Into<String>) -> Result<Self> {
        let text = text.into();
        if text.trim().is_empty() {
            return Err(DetectError::InvalidArgument(
                "OCR token text is required".to_string(),
            ));
        }
        Ok(Self {
            text,
            region: None,
            confidence: None,
            attributes: BTreeMap::new(),
        })
    }

    /// Returns region.
    pub fn region(mut self, value: BoundingBox) -> Self {
        self.region = Some(value);
        self
    }

    /// Returns confidence.
    pub fn confidence(mut self, value: impl Into<OcrConfidence>) -> Self {
        self.confidence = Some(value.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Data type for OCR text line.
pub struct OcrTextLine {
    /// Text content.
    pub text: String,
    /// Optional region in image coordinates.
    pub region: Option<BoundingBox>,
    /// Optional confidence.
    pub confidence: Option<OcrConfidence>,
    /// Tokens in this line.
    pub tokens: Vec<OcrToken>,
    /// Attributes for adapter-specific metadata.
    pub attributes: BTreeMap<String, String>,
}

impl OcrTextLine {
    /// Creates a new value.
    pub fn new(text: impl Into<String>) -> Result<Self> {
        let text = text.into();
        if text.trim().is_empty() {
            return Err(DetectError::InvalidArgument(
                "OCR line text is required".to_string(),
            ));
        }
        Ok(Self {
            text,
            region: None,
            confidence: None,
            tokens: Vec::new(),
            attributes: BTreeMap::new(),
        })
    }

    /// Returns region.
    pub fn region(mut self, value: BoundingBox) -> Self {
        self.region = Some(value);
        self
    }

    /// Returns confidence.
    pub fn confidence(mut self, value: impl Into<OcrConfidence>) -> Self {
        self.confidence = Some(value.into());
        self
    }

    /// Returns token.
    pub fn token(mut self, value: OcrToken) -> Self {
        self.tokens.push(value);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// Variants describing OCR block role.
pub enum OcrBlockKind {
    /// Paragraph-like flowing text.
    Paragraph,
    /// Heading or title text.
    Heading,
    /// Table-like structured text.
    Table,
    /// Form/key-value text.
    Form,
    /// Caption text.
    Caption,
    /// Custom block role.
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Data type for OCR text block.
pub struct OcrTextBlock {
    /// Block role.
    pub kind: OcrBlockKind,
    /// Text content.
    pub text: String,
    /// Optional region in image coordinates.
    pub region: Option<BoundingBox>,
    /// Optional confidence.
    pub confidence: Option<OcrConfidence>,
    /// Lines in this block.
    pub lines: Vec<OcrTextLine>,
    /// Attributes for adapter-specific metadata.
    pub attributes: BTreeMap<String, String>,
}

impl OcrTextBlock {
    /// Creates a new paragraph block.
    pub fn paragraph(text: impl Into<String>) -> Result<Self> {
        Self::new(OcrBlockKind::Paragraph, text)
    }

    /// Creates a new value.
    pub fn new(kind: OcrBlockKind, text: impl Into<String>) -> Result<Self> {
        let text = text.into();
        if text.trim().is_empty() {
            return Err(DetectError::InvalidArgument(
                "OCR block text is required".to_string(),
            ));
        }
        Ok(Self {
            kind,
            text,
            region: None,
            confidence: None,
            lines: Vec::new(),
            attributes: BTreeMap::new(),
        })
    }

    /// Returns region.
    pub fn region(mut self, value: BoundingBox) -> Self {
        self.region = Some(value);
        self
    }

    /// Returns confidence.
    pub fn confidence(mut self, value: impl Into<OcrConfidence>) -> Self {
        self.confidence = Some(value.into());
        self
    }

    /// Returns line.
    pub fn line(mut self, value: OcrTextLine) -> Self {
        self.lines.push(value);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Data type for rich OCR output from an image.
pub struct OcrDocument {
    /// Full extracted text, usually reading-order normalized.
    pub text: String,
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
    /// Optional detected language.
    pub language: Option<String>,
    /// Optional confidence.
    pub confidence: Option<OcrConfidence>,
    /// Structured text blocks.
    pub blocks: Vec<OcrTextBlock>,
    /// Attributes for adapter-specific metadata.
    pub attributes: BTreeMap<String, String>,
}

impl OcrDocument {
    /// Creates a new value.
    pub fn new(text: impl Into<String>, width: u32, height: u32) -> Result<Self> {
        if width == 0 || height == 0 {
            return Err(DetectError::InvalidDimensions { width, height });
        }
        let text = text.into();
        Ok(Self {
            text,
            width,
            height,
            language: None,
            confidence: None,
            blocks: Vec::new(),
            attributes: BTreeMap::new(),
        })
    }

    /// Builds an empty result from an image.
    pub fn empty_for_image(image: &ImageView<'_>) -> Self {
        Self {
            text: String::new(),
            width: image.width,
            height: image.height,
            language: None,
            confidence: None,
            blocks: Vec::new(),
            attributes: BTreeMap::new(),
        }
    }

    /// Returns language.
    pub fn language(mut self, value: impl Into<String>) -> Self {
        self.language = Some(value.into());
        self
    }

    /// Returns confidence.
    pub fn confidence(mut self, value: impl Into<OcrConfidence>) -> Self {
        self.confidence = Some(value.into());
        self
    }

    /// Returns block.
    pub fn block(mut self, value: OcrTextBlock) -> Self {
        self.blocks.push(value);
        self
    }

    /// Returns attribute.
    pub fn attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }
}

/// Trait for OCR backend implementations.
pub trait OcrBackend {
    /// Returns recognize image.
    fn recognize_image(
        &mut self,
        image: &ImageView<'_>,
        request: &OcrRequest,
    ) -> Result<OcrDocument>;
}

/// Trait for model-backed OCR backend implementations.
pub trait ModelBackedOcrBackend: OcrBackend {
    /// Returns model spec.
    fn model_spec(&self) -> HuggingFaceModelSpec;
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// OCR backend that invokes an external command and reads an `OcrDocument` JSON object from stdout.
pub struct JsonCommandOcrBackend {
    command: PathBuf,
    args: Vec<String>,
}

impl JsonCommandOcrBackend {
    /// Creates a new backend.
    pub fn new(command: impl Into<PathBuf>) -> Self {
        Self {
            command: command.into(),
            args: Vec::new(),
        }
    }

    /// Returns this backend with command args.
    pub fn args(mut self, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.args = args.into_iter().map(Into::into).collect();
        self
    }

    /// Returns command path.
    pub fn command(&self) -> &std::path::Path {
        &self.command
    }

    /// Returns command args.
    pub fn command_args(&self) -> &[String] {
        &self.args
    }
}

impl OcrBackend for JsonCommandOcrBackend {
    fn recognize_image(
        &mut self,
        image: &ImageView<'_>,
        request: &OcrRequest,
    ) -> Result<OcrDocument> {
        let temp_image = tempfile::Builder::new()
            .suffix(".png")
            .tempfile()
            .map_err(DetectError::Io)?;
        let image_path = temp_image.path().to_path_buf();
        let owned = compact_image(image)?;
        image_analysis_io::write_image(&image_path, &owned)?;

        let mut saw_placeholder = false;
        let mut args = Vec::with_capacity(self.args.len() + 1);
        let image_arg = image_path.to_string_lossy().into_owned();
        for arg in &self.args {
            if arg.contains("{image}") {
                saw_placeholder = true;
                args.push(arg.replace("{image}", &image_arg));
            } else {
                args.push(arg.clone());
            }
        }
        if !saw_placeholder {
            args.push(image_arg);
        }

        let request_json = serde_json::to_string(&JsonCommandOcrRequest::from_request(
            request,
            image.width,
            image.height,
        ))
        .map_err(|err| DetectError::Source(format!("failed to serialize OCR request: {err}")))?;

        let output = Command::new(&self.command)
            .args(&args)
            .env("OCR_REQUEST_JSON", request_json)
            .output()
            .map_err(|err| {
                DetectError::Source(format!(
                    "failed to run OCR command `{}`: {err}",
                    self.command.display()
                ))
            })?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DetectError::Source(format!(
                "OCR command `{}` exited with status {}{}",
                self.command.display(),
                output.status,
                if stderr.trim().is_empty() {
                    String::new()
                } else {
                    format!(": {}", stderr.trim())
                }
            )));
        }

        let mut value: serde_json::Value = serde_json::from_slice(&output.stdout)
            .map_err(|err| DetectError::Source(format!("invalid OCR JSON: {err}")))?;
        let object = value.as_object_mut().ok_or_else(|| {
            DetectError::Source("invalid OCR document: expected JSON object".to_string())
        })?;
        object
            .entry("width")
            .or_insert_with(|| serde_json::json!(image.width));
        object
            .entry("height")
            .or_insert_with(|| serde_json::json!(image.height));
        let document: OcrDocument = serde_json::from_value(value)
            .map_err(|err| DetectError::Source(format!("invalid OCR document: {err}")))?;
        validate_ocr_document(&document)?;
        Ok(document)
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonCommandOcrRequest<'a> {
    width: u32,
    height: u32,
    languages: &'a [String],
    preserve_layout: bool,
    include_tokens: bool,
    min_confidence: Option<u8>,
    attributes: &'a BTreeMap<String, String>,
}

impl<'a> JsonCommandOcrRequest<'a> {
    fn from_request(request: &'a OcrRequest, width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            languages: &request.languages,
            preserve_layout: request.preserve_layout,
            include_tokens: request.include_tokens,
            min_confidence: request.min_confidence,
            attributes: &request.attributes,
        }
    }
}

fn validate_ocr_document(document: &OcrDocument) -> Result<()> {
    if document.width == 0 || document.height == 0 {
        return Err(DetectError::InvalidDimensions {
            width: document.width,
            height: document.height,
        });
    }
    for block in &document.blocks {
        if block.text.trim().is_empty() {
            return Err(DetectError::Source(
                "invalid OCR document: block text is required".to_string(),
            ));
        }
        for line in &block.lines {
            if line.text.trim().is_empty() {
                return Err(DetectError::Source(
                    "invalid OCR document: line text is required".to_string(),
                ));
            }
            for token in &line.tokens {
                if token.text.trim().is_empty() {
                    return Err(DetectError::Source(
                        "invalid OCR document: token text is required".to_string(),
                    ));
                }
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Data type for OCR output at one video frame.
pub struct FrameOcrDocument {
    /// The position value.
    pub position: FramePosition,
    /// OCR output for this frame.
    pub document: OcrDocument,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
/// Data type for OCR output over video frames.
pub struct VideoOcrDocument {
    /// OCR output for each sampled frame.
    pub frames: Vec<FrameOcrDocument>,
}

impl VideoOcrDocument {
    /// Returns combined text.
    pub fn combined_text(&self) -> String {
        self.frames
            .iter()
            .map(|frame| frame.document.text.trim())
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Data type for OCR pipeline.
pub struct OcrPipeline<B> {
    backend: B,
    request: OcrRequest,
}

impl<B> OcrPipeline<B> {
    /// Creates a new value.
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            request: OcrRequest::default(),
        }
    }

    /// Returns request.
    pub fn request(mut self, value: OcrRequest) -> Self {
        self.request = value;
        self
    }

    /// Returns backend.
    pub fn backend(&self) -> &B {
        &self.backend
    }

    /// Returns backend mut.
    pub fn backend_mut(&mut self) -> &mut B {
        &mut self.backend
    }
}

impl<B: OcrBackend> OcrPipeline<B> {
    /// Returns recognize image.
    pub fn recognize_image(&mut self, image: &ImageView<'_>) -> Result<OcrDocument> {
        self.backend.recognize_image(image, &self.request)
    }

    /// Returns recognize frame.
    pub fn recognize_frame(&mut self, frame: &VideoFrame<'_>) -> Result<FrameOcrDocument> {
        let image = ImageView::from_video_frame(frame)?;
        let document = self.recognize_image(&image)?;
        Ok(FrameOcrDocument {
            position: frame.position,
            document,
        })
    }

    /// Returns recognize frames.
    pub fn recognize_frames<'a>(
        &mut self,
        frames: impl IntoIterator<Item = &'a VideoFrame<'a>>,
    ) -> Result<VideoOcrDocument> {
        let mut output = VideoOcrDocument::default();
        for frame in frames {
            output.frames.push(self.recognize_frame(frame)?);
        }
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image_analysis_core::OwnedImage;
    use std::fs;
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::tempdir;
    use video_analysis_core::{PixelFormat, Timebase, Timestamp};

    struct StubOcrBackend;

    impl OcrBackend for StubOcrBackend {
        fn recognize_image(
            &mut self,
            image: &ImageView<'_>,
            request: &OcrRequest,
        ) -> Result<OcrDocument> {
            let language = request
                .languages
                .first()
                .cloned()
                .unwrap_or_else(|| "und".to_string());
            Ok(OcrDocument::new("Hello OCR", image.width, image.height)?
                .language(language)
                .confidence(96)
                .block(
                    OcrTextBlock::paragraph("Hello OCR")?
                        .region(BoundingBox::new(1, 1, image.width - 1, image.height - 1)?)
                        .line(
                            OcrTextLine::new("Hello OCR")?
                                .token(OcrToken::new("Hello")?)
                                .token(OcrToken::new("OCR")?),
                        ),
                ))
        }
    }

    #[test]
    fn default_preset_uses_trocr_printed() {
        assert_eq!(
            default_ocr_model_spec().repo_id_value(),
            Some("microsoft/trocr-base-printed")
        );
        assert_eq!(
            OcrPreset::from_str("trocr-base-handwritten")
                .unwrap()
                .model_spec()
                .repo_id_value(),
            Some("microsoft/trocr-base-handwritten")
        );
    }

    #[test]
    fn request_can_select_multiple_model_and_command_techniques() {
        let request = OcrRequest::new()
            .model_preset(OcrPreset::DonutBaseCordV2)
            .languages(["en", "de"])
            .min_confidence(125);
        assert_eq!(request.min_confidence, Some(100));
        assert_eq!(request.languages, ["en".to_string(), "de".to_string()]);
        assert!(matches!(
            request.technique,
            Some(OcrTechnique::HuggingFaceModel(_))
        ));

        let command = ExternalOcrCommandSpec::new("tesseract", "tesseract")
            .unwrap()
            .arg("--psm")
            .arg("6");
        let technique = OcrTechnique::ExternalCommand(command);
        assert_eq!(technique.kind(), "external_command");
        assert_eq!(technique.runtime_backend(), ModelRuntimeBackend::External);
    }

    #[test]
    fn rich_text_blocks_preserve_layout() {
        let block = OcrTextBlock::new(OcrBlockKind::Heading, "Invoice 42")
            .unwrap()
            .confidence(91)
            .line(
                OcrTextLine::new("Invoice 42")
                    .unwrap()
                    .token(OcrToken::new("Invoice").unwrap())
                    .token(OcrToken::new("42").unwrap()),
            );
        let document = OcrDocument::new("Invoice 42", 320, 180)
            .unwrap()
            .language("en")
            .block(block);
        assert_eq!(document.blocks[0].lines[0].tokens.len(), 2);
        assert_eq!(document.language.as_deref(), Some("en"));
    }

    #[test]
    fn pipeline_runs_ocr_on_images_and_video_frames() {
        let image = OwnedImage::new_rgb(8, 8, vec![255; 8 * 8 * 3]).unwrap();
        let mut pipeline = OcrPipeline::new(StubOcrBackend)
            .request(OcrRequest::new().languages(["en"]).include_tokens(true));

        let document = pipeline.recognize_image(&image.as_view()).unwrap();
        assert_eq!(document.text, "Hello OCR");
        assert_eq!(document.blocks[0].lines[0].tokens[0].text, "Hello");

        let frame_position = FramePosition {
            frame_index: 3,
            timestamp: Timestamp::new(3, Timebase::new(1, 30)),
        };
        let frame = VideoFrame {
            position: frame_position,
            width: 8,
            height: 8,
            pixel_format: PixelFormat::Rgb24,
            data: &image.data,
            stride: image.stride,
        };
        let frame_document = pipeline.recognize_frame(&frame).unwrap();
        assert_eq!(frame_document.position.frame_index, 3);
        assert_eq!(frame_document.document.text, "Hello OCR");
    }

    #[test]
    fn empty_document_uses_image_dimensions() {
        let image = OwnedImage::new_rgb(2, 3, vec![0; 18]).unwrap();
        let document = OcrDocument::empty_for_image(&image.as_view());
        assert_eq!((document.width, document.height), (2, 3));
        assert!(document.text.is_empty());
    }

    #[test]
    fn rejects_empty_structural_text() {
        assert!(OcrToken::new(" ").is_err());
        assert!(OcrTextLine::new("").is_err());
        assert!(OcrTextBlock::paragraph("").is_err());
        assert!(OcrDocument::new("", 0, 1).is_err());
    }

    #[test]
    fn frame_positions_remain_plain_core_types() {
        let position = FramePosition {
            frame_index: 9,
            timestamp: Timestamp::new(9, Timebase::new(1, 24)),
        };
        assert_eq!(position.timestamp.seconds(), 0.375);
    }

    #[test]
    fn json_command_backend_parses_valid_document() {
        let script = fixture_script(
            r#"#!/bin/sh
test -f "$1" || exit 9
printf '%s' '{"text":"Slide Title","width":4,"height":3,"language":"en","confidence":88,"blocks":[{"kind":"heading","text":"Slide Title","region":{"x":1,"y":1,"width":2,"height":1},"confidence":90,"lines":[],"attributes":{}}],"attributes":{}}'
"#,
        );
        let image = OwnedImage::new_rgb(4, 3, vec![255; 36]).unwrap();
        let mut backend = JsonCommandOcrBackend::new(script.path());
        let document = backend
            .recognize_image(&image.as_view(), &OcrRequest::new().languages(["en"]))
            .unwrap();
        assert_eq!(document.text, "Slide Title");
        assert_eq!((document.width, document.height), (4, 3));
        assert_eq!(document.blocks[0].kind, OcrBlockKind::Heading);
        assert_eq!(document.blocks[0].region.unwrap().x, 1);
    }

    #[test]
    fn json_command_backend_fills_missing_dimensions() {
        let script = fixture_script(
            r#"#!/bin/sh
printf '%s' '{"text":"No dimensions","blocks":[{"kind":"paragraph","text":"No dimensions","lines":[],"attributes":{}}],"attributes":{}}'
"#,
        );
        let image = OwnedImage::new_rgb(5, 2, vec![0; 30]).unwrap();
        let mut backend = JsonCommandOcrBackend::new(script.path()).args(["--image={image}"]);
        let document = backend
            .recognize_image(&image.as_view(), &OcrRequest::new())
            .unwrap();
        assert_eq!((document.width, document.height), (5, 2));
    }

    #[test]
    fn json_command_backend_reports_command_failure_and_invalid_json() {
        let failing = fixture_script(
            r#"#!/bin/sh
echo "no OCR available" >&2
exit 42
"#,
        );
        let invalid = fixture_script(
            r#"#!/bin/sh
printf '%s' 'not-json'
"#,
        );
        let image = OwnedImage::new_rgb(2, 2, vec![0; 12]).unwrap();

        let mut failing_backend = JsonCommandOcrBackend::new(failing.path());
        let error = failing_backend
            .recognize_image(&image.as_view(), &OcrRequest::new())
            .unwrap_err()
            .to_string();
        assert!(error.contains("exited with status"));
        assert!(error.contains("no OCR available"));

        let mut invalid_backend = JsonCommandOcrBackend::new(invalid.path());
        let error = invalid_backend
            .recognize_image(&image.as_view(), &OcrRequest::new())
            .unwrap_err()
            .to_string();
        assert!(error.contains("invalid OCR JSON"));
    }

    #[test]
    fn json_command_backend_exposes_request_environment() {
        let script = fixture_script(
            r#"#!/bin/sh
case "$OCR_REQUEST_JSON" in
  *'"languages":["de"]'*'"preserveLayout":false'*'"includeTokens":false'*'"minConfidence":77'*)
    printf '%s' '{"text":"Umgebung","blocks":[],"attributes":{}}'
    ;;
  *)
    echo "$OCR_REQUEST_JSON" >&2
    exit 7
    ;;
esac
"#,
        );
        let image = OwnedImage::new_rgb(3, 3, vec![0; 27]).unwrap();
        let request = OcrRequest::new()
            .languages(["de"])
            .preserve_layout(false)
            .include_tokens(false)
            .min_confidence(77);
        let mut backend = JsonCommandOcrBackend::new(script.path());
        let document = backend.recognize_image(&image.as_view(), &request).unwrap();
        assert_eq!(document.text, "Umgebung");
        assert_eq!((document.width, document.height), (3, 3));
    }

    struct FixtureScript {
        _dir: tempfile::TempDir,
        path: std::path::PathBuf,
    }

    impl FixtureScript {
        fn path(&self) -> &std::path::Path {
            &self.path
        }
    }

    fn fixture_script(contents: &str) -> FixtureScript {
        let dir = tempdir().unwrap();
        let path = dir.path().join("ocr-fixture.sh");
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(contents.as_bytes()).unwrap();
        let mut permissions = file.metadata().unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions).unwrap();
        FixtureScript { _dir: dir, path }
    }
}
