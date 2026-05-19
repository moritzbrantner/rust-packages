use std::str::FromStr;

use video_analysis_core::{DetectError, Result};

use crate::{HuggingFaceModelSpec, ModelTask};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Variants describing model preset.
pub enum ModelPreset {
    /// The detr resnet50 variant.
    DetrResnet50,
    /// The yolos tiny variant.
    YolosTiny,
    /// The distilbert sst2 variant.
    DistilbertSst2,
    /// The bert base ner variant.
    BertBaseNer,
    /// The mini lm l6 v2 variant.
    MiniLmL6V2,
    /// The xenova distilbert sst2 ONNX variant.
    XenovaDistilbertSst2Onnx,
    /// The xenova mini lm l6 v2 ONNX variant.
    XenovaMiniLmL6V2Onnx,
    /// The xenova DETR ResNet-50 ONNX variant.
    XenovaDetrResnet50Onnx,
}

impl ModelPreset {
    /// Constant for all.
    pub const ALL: &'static [Self] = &[
        Self::DetrResnet50,
        Self::YolosTiny,
        Self::DistilbertSst2,
        Self::BertBaseNer,
        Self::MiniLmL6V2,
        Self::XenovaDistilbertSst2Onnx,
        Self::XenovaMiniLmL6V2Onnx,
        Self::XenovaDetrResnet50Onnx,
    ];

    /// Borrows this value as a str.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DetrResnet50 => "detr-resnet-50",
            Self::YolosTiny => "yolos-tiny",
            Self::DistilbertSst2 => "distilbert-sst2",
            Self::BertBaseNer => "bert-base-ner",
            Self::MiniLmL6V2 => "minilm-l6-v2",
            Self::XenovaDistilbertSst2Onnx => "xenova-distilbert-sst2-onnx",
            Self::XenovaMiniLmL6V2Onnx => "xenova-minilm-l6-v2-onnx",
            Self::XenovaDetrResnet50Onnx => "xenova-detr-resnet-50-onnx",
        }
    }

    /// Returns spec.
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
            Self::XenovaDistilbertSst2Onnx => HuggingFaceModelSpec::new(
                "Xenova/distilbert-base-uncased-finetuned-sst-2-english",
                ModelTask::TextClassification,
            )
            .name(self.as_str())
            .file("config.json")
            .file("tokenizer.json")
            .file("tokenizer_config.json")
            .first_available_file([
                "onnx/model.onnx",
                "onnx/model_quantized.onnx",
                "onnx/model_int8.onnx",
            ]),
            Self::XenovaMiniLmL6V2Onnx => {
                HuggingFaceModelSpec::new("Xenova/all-MiniLM-L6-v2", ModelTask::TextEmbedding)
                    .name(self.as_str())
                    .file("config.json")
                    .file("tokenizer.json")
                    .file("tokenizer_config.json")
                    .first_available_file(["onnx/model.onnx", "onnx/model_quantized.onnx"])
            }
            Self::XenovaDetrResnet50Onnx => {
                HuggingFaceModelSpec::new("Xenova/detr-resnet-50", ModelTask::ObjectDetection)
                    .name(self.as_str())
                    .file("config.json")
                    .file("preprocessor_config.json")
                    .first_available_file(["onnx/model.onnx", "onnx/model_quantized.onnx"])
            }
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
