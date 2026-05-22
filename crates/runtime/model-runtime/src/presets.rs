use std::str::FromStr;

use crate::{HuggingFaceModelSpec, ModelRuntimeError, ModelTask, Result};

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
    /// The all mpnet base v2 ONNX variant.
    XenovaAllMpnetBaseV2Onnx,
    /// The twitter roberta sentiment ONNX variant.
    XenovaTwitterRobertaSentimentOnnx,
    /// The BART MNLI zero-shot ONNX variant.
    XenovaBartLargeMnliOnnx,
    /// The BART CNN summarization ONNX variant.
    XenovaBartLargeCnnOnnx,
    /// The MS MARCO MiniLM reranker ONNX variant.
    XenovaMsMarcoMiniLmL6V2Onnx,
    /// The RoBERTa SQuAD2 QA ONNX variant.
    XenovaRobertaBaseSquad2Onnx,
    /// The xenova DETR ResNet-50 ONNX variant.
    XenovaDetrResnet50Onnx,
    /// The AST AudioSet classification variant.
    AstAudioset,
    /// The Xenova AST AudioSet ONNX variant.
    XenovaAstAudiosetOnnx,
    /// The CLAP audio embedding variant.
    ClapHtsatUnfused,
    /// The Whisper tiny English ASR variant.
    WhisperTinyEn,
    /// The wav2vec2 base ASR variant.
    Wav2Vec2Base960h,
    /// The pyannote speaker diarization variant.
    PyannoteSpeakerDiarization31,
    /// The Demucs music separation variant.
    DemucsMusicSeparation,
    /// The MusicGen small generation variant.
    MusicgenSmall,
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
        Self::XenovaAllMpnetBaseV2Onnx,
        Self::XenovaTwitterRobertaSentimentOnnx,
        Self::XenovaBartLargeMnliOnnx,
        Self::XenovaBartLargeCnnOnnx,
        Self::XenovaMsMarcoMiniLmL6V2Onnx,
        Self::XenovaRobertaBaseSquad2Onnx,
        Self::XenovaDetrResnet50Onnx,
        Self::AstAudioset,
        Self::XenovaAstAudiosetOnnx,
        Self::ClapHtsatUnfused,
        Self::WhisperTinyEn,
        Self::Wav2Vec2Base960h,
        Self::PyannoteSpeakerDiarization31,
        Self::DemucsMusicSeparation,
        Self::MusicgenSmall,
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
            Self::XenovaAllMpnetBaseV2Onnx => "xenova-all-mpnet-base-v2-onnx",
            Self::XenovaTwitterRobertaSentimentOnnx => {
                "xenova-twitter-roberta-sentiment-latest-onnx"
            }
            Self::XenovaBartLargeMnliOnnx => "xenova-bart-large-mnli-onnx",
            Self::XenovaBartLargeCnnOnnx => "xenova-bart-large-cnn-onnx",
            Self::XenovaMsMarcoMiniLmL6V2Onnx => "xenova-ms-marco-minilm-l6-v2-onnx",
            Self::XenovaRobertaBaseSquad2Onnx => "xenova-roberta-base-squad2-onnx",
            Self::XenovaDetrResnet50Onnx => "xenova-detr-resnet-50-onnx",
            Self::AstAudioset => "ast-audioset",
            Self::XenovaAstAudiosetOnnx => "xenova-ast-audioset-onnx",
            Self::ClapHtsatUnfused => "clap-htsat-unfused",
            Self::WhisperTinyEn => "whisper-tiny-en",
            Self::Wav2Vec2Base960h => "wav2vec2-base-960h",
            Self::PyannoteSpeakerDiarization31 => "pyannote-speaker-diarization-3-1",
            Self::DemucsMusicSeparation => "demucs-music-separation",
            Self::MusicgenSmall => "musicgen-small",
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
            Self::XenovaAllMpnetBaseV2Onnx => {
                HuggingFaceModelSpec::new("Xenova/all-mpnet-base-v2", ModelTask::TextEmbedding)
                    .name(self.as_str())
                    .file("config.json")
                    .file("tokenizer.json")
                    .file("tokenizer_config.json")
                    .optional_file("modules.json")
                    .first_available_file(["onnx/model.onnx", "onnx/model_quantized.onnx"])
            }
            Self::XenovaTwitterRobertaSentimentOnnx => HuggingFaceModelSpec::new(
                "Xenova/twitter-roberta-base-sentiment-latest",
                ModelTask::TextClassification,
            )
            .name(self.as_str())
            .file("config.json")
            .file("tokenizer.json")
            .file("tokenizer_config.json")
            .first_available_file(["onnx/model.onnx", "onnx/model_quantized.onnx"]),
            Self::XenovaBartLargeMnliOnnx => HuggingFaceModelSpec::new(
                "Xenova/bart-large-mnli",
                ModelTask::ZeroShotClassification,
            )
            .name(self.as_str())
            .file("config.json")
            .file("tokenizer.json")
            .file("tokenizer_config.json")
            .first_available_file([
                "onnx/encoder_model.onnx",
                "onnx/model.onnx",
                "onnx/model_quantized.onnx",
            ]),
            Self::XenovaBartLargeCnnOnnx => {
                HuggingFaceModelSpec::new("Xenova/bart-large-cnn", ModelTask::Summarization)
                    .name(self.as_str())
                    .file("config.json")
                    .file("tokenizer.json")
                    .file("tokenizer_config.json")
                    .first_available_file([
                        "onnx/encoder_model.onnx",
                        "onnx/model.onnx",
                        "onnx/model_quantized.onnx",
                    ])
            }
            Self::XenovaMsMarcoMiniLmL6V2Onnx => {
                HuggingFaceModelSpec::new("Xenova/ms-marco-MiniLM-L-6-v2", ModelTask::Reranking)
                    .name(self.as_str())
                    .file("config.json")
                    .file("tokenizer.json")
                    .file("tokenizer_config.json")
                    .first_available_file(["onnx/model.onnx", "onnx/model_quantized.onnx"])
            }
            Self::XenovaRobertaBaseSquad2Onnx => HuggingFaceModelSpec::new(
                "Xenova/roberta-base-squad2",
                ModelTask::QuestionAnswering,
            )
            .name(self.as_str())
            .file("config.json")
            .file("tokenizer.json")
            .file("tokenizer_config.json")
            .first_available_file(["onnx/model.onnx", "onnx/model_quantized.onnx"]),
            Self::XenovaDetrResnet50Onnx => {
                HuggingFaceModelSpec::new("Xenova/detr-resnet-50", ModelTask::ObjectDetection)
                    .name(self.as_str())
                    .file("config.json")
                    .file("preprocessor_config.json")
                    .first_available_file(["onnx/model.onnx", "onnx/model_quantized.onnx"])
            }
            Self::AstAudioset => HuggingFaceModelSpec::new(
                "MIT/ast-finetuned-audioset-10-10-0.4593",
                ModelTask::AudioClassification,
            )
            .name(self.as_str())
            .file("config.json")
            .file("preprocessor_config.json")
            .first_available_file(["model.safetensors", "pytorch_model.bin"]),
            Self::XenovaAstAudiosetOnnx => HuggingFaceModelSpec::new(
                "Xenova/ast-finetuned-audioset-10-10-0.4593",
                ModelTask::AudioClassification,
            )
            .name(self.as_str())
            .file("config.json")
            .file("preprocessor_config.json")
            .first_available_file(["onnx/model.onnx", "onnx/model_quantized.onnx"]),
            Self::ClapHtsatUnfused => {
                HuggingFaceModelSpec::new("laion/clap-htsat-unfused", ModelTask::AudioEmbedding)
                    .name(self.as_str())
                    .file("config.json")
                    .file("preprocessor_config.json")
                    .optional_file("tokenizer.json")
                    .first_available_file(["model.safetensors", "pytorch_model.bin"])
            }
            Self::WhisperTinyEn => {
                HuggingFaceModelSpec::new("openai/whisper-tiny.en", ModelTask::SpeechRecognition)
                    .name(self.as_str())
                    .file("config.json")
                    .file("preprocessor_config.json")
                    .file("tokenizer.json")
                    .first_available_file(["model.safetensors", "pytorch_model.bin"])
            }
            Self::Wav2Vec2Base960h => HuggingFaceModelSpec::new(
                "facebook/wav2vec2-base-960h",
                ModelTask::SpeechRecognition,
            )
            .name(self.as_str())
            .file("config.json")
            .file("preprocessor_config.json")
            .file("tokenizer.json")
            .first_available_file(["model.safetensors", "pytorch_model.bin"]),
            Self::PyannoteSpeakerDiarization31 => HuggingFaceModelSpec::new(
                "pyannote/speaker-diarization-3.1",
                ModelTask::SpeakerDiarization,
            )
            .name(self.as_str())
            .file("config.yaml")
            .optional_file("pytorch_model.bin"),
            Self::DemucsMusicSeparation => {
                HuggingFaceModelSpec::new("facebook/demucs", ModelTask::SourceSeparation)
                    .name(self.as_str())
                    .file("config.json")
                    .first_available_file(["pytorch_model.bin", "model.safetensors"])
            }
            Self::MusicgenSmall => {
                HuggingFaceModelSpec::new("facebook/musicgen-small", ModelTask::AudioGeneration)
                    .name(self.as_str())
                    .file("config.json")
                    .file("preprocessor_config.json")
                    .file("tokenizer.json")
                    .first_available_file(["model.safetensors", "pytorch_model.bin"])
            }
        }
    }
}

impl FromStr for ModelPreset {
    type Err = ModelRuntimeError;

    fn from_str(input: &str) -> Result<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|preset| preset.as_str() == input)
            .ok_or_else(|| {
                ModelRuntimeError::InvalidArgument(format!(
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
