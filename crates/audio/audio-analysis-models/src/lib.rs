#![doc = include_str!("../README.md")]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use video_analysis_core::{DetectError, Result};
use video_analysis_models::ModelPreset;

/// Audio task families exposed by the shared model layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioTask {
    /// Whole-clip or window-level audio classification.
    AudioClassification,
    /// Timestamped acoustic event detection.
    AudioEventDetection,
    /// Dense audio embedding.
    AudioEmbedding,
    /// Automatic speech recognition.
    SpeechRecognition,
    /// Speaker diarization.
    SpeakerDiarization,
    /// Music or speech source separation.
    SourceSeparation,
    /// Audio or music generation.
    AudioGeneration,
}

impl AudioTask {
    /// Returns all task variants.
    pub const ALL: &'static [Self] = &[
        Self::AudioClassification,
        Self::AudioEventDetection,
        Self::AudioEmbedding,
        Self::SpeechRecognition,
        Self::SpeakerDiarization,
        Self::SourceSeparation,
        Self::AudioGeneration,
    ];

    /// Returns the stable API path segment for this task.
    pub fn path_segment(self) -> &'static str {
        match self {
            Self::AudioClassification => "classify",
            Self::AudioEventDetection => "events",
            Self::AudioEmbedding => "embed",
            Self::SpeechRecognition => "transcribe",
            Self::SpeakerDiarization => "diarize",
            Self::SourceSeparation => "separate",
            Self::AudioGeneration => "generate",
        }
    }
}

/// Runtime families for audio execution and postprocessing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioRuntime {
    /// ONNX Runtime-backed native inference.
    Onnx,
    /// Candle-backed native inference.
    Candle,
    /// Whisper.cpp-backed ASR.
    WhisperCpp,
    /// External command-backed execution.
    External,
    /// Deterministic spectral fallback.
    Spectral,
    /// Deterministic heuristic fallback.
    Heuristic,
    /// Caller-supplied model predictions.
    ImportedPredictions,
}

/// Fallback behavior when the selected native model cannot run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FallbackPolicy {
    /// Return a typed error.
    Error,
    /// Use a fast deterministic fallback.
    FastFallback,
    /// Use a spectral or heuristic fallback.
    HeuristicFallback,
}

impl Default for FallbackPolicy {
    fn default() -> Self {
        Self::Error
    }
}

/// Model selection supplied by API, CLI, or UI callers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AudioModelSelection {
    /// Optional preset or Hugging Face model identifier.
    #[serde(default)]
    pub model_id: Option<String>,
    /// Optional preferred runtime.
    #[serde(default)]
    pub runtime: Option<AudioRuntime>,
    /// Fallback policy for unsupported native paths.
    #[serde(default)]
    pub fallback_policy: FallbackPolicy,
}

/// Shared model metadata for UI and CLI discovery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioModelMetadata {
    /// Stable local preset id.
    pub id: String,
    /// Upstream model id or strategy id.
    pub model_id: String,
    /// Task family.
    pub task: AudioTask,
    /// Preferred runtime.
    pub runtime: AudioRuntime,
    /// Whether the runtime is available in the default contributor build.
    pub supported: bool,
    /// Optional fallback preset id.
    pub fallback: Option<String>,
    /// Human-readable note for unsupported or fallback-only paths.
    pub note: Option<String>,
}

/// Feature summary for deterministic fallback paths.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioFeatureSummary {
    /// Clip duration in seconds.
    #[serde(default)]
    pub duration_seconds: f32,
    /// Sample rate in Hz.
    #[serde(default)]
    pub sample_rate: u32,
    /// Root mean square energy.
    #[serde(default)]
    pub rms: f32,
    /// Peak absolute amplitude.
    #[serde(default)]
    pub peak: f32,
    /// Zero crossing rate.
    #[serde(default)]
    pub zero_crossing_rate: f32,
    /// Dominant frequency in Hz.
    #[serde(default)]
    pub dominant_frequency_hz: f32,
    /// Spectral centroid in Hz.
    #[serde(default)]
    pub spectral_centroid_hz: f32,
}

/// Imported prediction used by server, CLI, and UI postprocessing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedAudioPrediction {
    /// Optional raw prediction kind.
    #[serde(default)]
    pub kind: Option<String>,
    /// Label emitted by the model.
    pub label: String,
    /// Confidence score.
    pub score: f32,
    /// Optional start time in seconds.
    #[serde(default)]
    pub start_seconds: Option<f32>,
    /// Optional end time in seconds.
    #[serde(default)]
    pub end_seconds: Option<f32>,
    /// Optional text payload.
    #[serde(default)]
    pub text: Option<String>,
    /// Arbitrary model attributes.
    #[serde(default)]
    pub attributes: BTreeMap<String, String>,
}

/// One label prediction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioClassPrediction {
    /// Label name.
    pub label: String,
    /// Confidence score.
    pub score: f32,
}

/// Request for audio classification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioClassificationRequest {
    /// Optional source identifier.
    #[serde(default)]
    pub source: Option<String>,
    /// Optional fixed labels for fallback classification.
    #[serde(default)]
    pub labels: Vec<String>,
    /// Maximum prediction count.
    #[serde(default = "default_top_k")]
    pub top_k: usize,
    /// Audio feature summary for deterministic fallback.
    #[serde(default)]
    pub features: Option<AudioFeatureSummary>,
    /// Model selection.
    #[serde(default)]
    pub model: AudioModelSelection,
    /// Caller-supplied model predictions.
    #[serde(default)]
    pub imported_predictions: Vec<ImportedAudioPrediction>,
}

/// Response for audio classification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioClassificationResponse {
    /// Accepted flag for generated package surfaces.
    pub accepted: bool,
    /// Operation name.
    pub operation: String,
    /// Selected model id.
    pub model_id: String,
    /// Runtime used.
    pub runtime: AudioRuntime,
    /// Ranked label predictions.
    pub predictions: Vec<AudioClassPrediction>,
}

/// Window-level feature frame for event fallback.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioFeatureFrame {
    /// Start time in seconds.
    pub start_seconds: f32,
    /// End time in seconds.
    pub end_seconds: f32,
    /// Root mean square energy.
    pub rms: f32,
    /// Peak absolute amplitude.
    #[serde(default)]
    pub peak: f32,
}

/// Request for acoustic event detection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioEventDetectionRequest {
    /// Optional source identifier.
    #[serde(default)]
    pub source: Option<String>,
    /// Frame summaries for deterministic fallback.
    #[serde(default)]
    pub frames: Vec<AudioFeatureFrame>,
    /// RMS threshold for fallback event detection.
    #[serde(default = "default_event_threshold")]
    pub threshold: f32,
    /// Model selection.
    #[serde(default)]
    pub model: AudioModelSelection,
    /// Caller-supplied model predictions.
    #[serde(default)]
    pub imported_predictions: Vec<ImportedAudioPrediction>,
}

/// One timestamped audio event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioEventPrediction {
    /// Event label.
    pub label: String,
    /// Confidence score.
    pub score: f32,
    /// Start time in seconds.
    pub start_seconds: f32,
    /// End time in seconds.
    pub end_seconds: f32,
}

/// Response for acoustic event detection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioEventDetectionResponse {
    /// Accepted flag for generated package surfaces.
    pub accepted: bool,
    /// Operation name.
    pub operation: String,
    /// Selected model id.
    pub model_id: String,
    /// Runtime used.
    pub runtime: AudioRuntime,
    /// Timestamped events.
    pub events: Vec<AudioEventPrediction>,
}

/// Request for audio embeddings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioEmbeddingRequest {
    /// Feature summaries to embed.
    #[serde(default)]
    pub features: Vec<AudioFeatureSummary>,
    /// Fallback vector dimensions.
    #[serde(default = "default_embedding_dimensions")]
    pub dimensions: usize,
    /// Whether to L2-normalize fallback embeddings.
    #[serde(default = "default_true")]
    pub normalize: bool,
    /// Model selection.
    #[serde(default)]
    pub model: AudioModelSelection,
    /// Caller-supplied embeddings.
    #[serde(default)]
    pub imported_embeddings: Vec<Vec<f32>>,
}

/// Response for audio embeddings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioEmbeddingResponse {
    /// Accepted flag for generated package surfaces.
    pub accepted: bool,
    /// Operation name.
    pub operation: String,
    /// Selected model id.
    pub model_id: String,
    /// Runtime used.
    pub runtime: AudioRuntime,
    /// Embedding dimensions.
    pub dimensions: usize,
    /// Dense embeddings.
    pub embeddings: Vec<Vec<f32>>,
}

/// Request for speech recognition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeechRecognitionRequest {
    /// Optional source identifier.
    #[serde(default)]
    pub source: Option<String>,
    /// Optional language hint.
    #[serde(default)]
    pub language: Option<String>,
    /// Model selection.
    #[serde(default)]
    pub model: AudioModelSelection,
    /// Caller-supplied transcript segments.
    #[serde(default)]
    pub imported_segments: Vec<TranscriptSegmentPrediction>,
}

/// One transcript segment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptSegmentPrediction {
    /// Segment index.
    pub index: usize,
    /// Start time in seconds.
    #[serde(default)]
    pub start_seconds: Option<f32>,
    /// End time in seconds.
    #[serde(default)]
    pub end_seconds: Option<f32>,
    /// Segment text.
    pub text: String,
    /// Confidence score.
    #[serde(default)]
    pub score: Option<f32>,
}

/// Response for speech recognition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeechRecognitionResponse {
    /// Accepted flag for generated package surfaces.
    pub accepted: bool,
    /// Operation name.
    pub operation: String,
    /// Selected model id.
    pub model_id: String,
    /// Runtime used.
    pub runtime: AudioRuntime,
    /// Full transcript text.
    pub text: String,
    /// Transcript segments.
    pub segments: Vec<TranscriptSegmentPrediction>,
}

/// Request for speaker diarization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeakerDiarizationRequest {
    /// Optional source identifier.
    #[serde(default)]
    pub source: Option<String>,
    /// Optional duration for heuristic fallback.
    #[serde(default)]
    pub duration_seconds: Option<f32>,
    /// Model selection.
    #[serde(default)]
    pub model: AudioModelSelection,
    /// Caller-supplied speaker segments.
    #[serde(default)]
    pub imported_segments: Vec<SpeakerSegmentPrediction>,
}

/// One speaker segment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeakerSegmentPrediction {
    /// Speaker label.
    pub speaker: String,
    /// Start time in seconds.
    pub start_seconds: f32,
    /// End time in seconds.
    pub end_seconds: f32,
    /// Confidence score.
    #[serde(default)]
    pub score: Option<f32>,
}

/// Response for speaker diarization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeakerDiarizationResponse {
    /// Accepted flag for generated package surfaces.
    pub accepted: bool,
    /// Operation name.
    pub operation: String,
    /// Selected model id.
    pub model_id: String,
    /// Runtime used.
    pub runtime: AudioRuntime,
    /// Speaker segments.
    pub segments: Vec<SpeakerSegmentPrediction>,
}

/// Request for source separation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceSeparationRequest {
    /// Optional source identifier.
    #[serde(default)]
    pub source: Option<String>,
    /// Requested stems.
    #[serde(default)]
    pub stems: Vec<String>,
    /// Model selection.
    #[serde(default)]
    pub model: AudioModelSelection,
    /// Caller-supplied stem descriptors.
    #[serde(default)]
    pub imported_stems: Vec<SeparatedStemPrediction>,
}

/// One separated stem descriptor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeparatedStemPrediction {
    /// Stem name.
    pub stem: String,
    /// Optional URI or path.
    #[serde(default)]
    pub uri: Option<String>,
    /// Confidence or quality score.
    #[serde(default)]
    pub score: Option<f32>,
}

/// Response for source separation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceSeparationResponse {
    /// Accepted flag for generated package surfaces.
    pub accepted: bool,
    /// Operation name.
    pub operation: String,
    /// Selected model id.
    pub model_id: String,
    /// Runtime used.
    pub runtime: AudioRuntime,
    /// Stem descriptors.
    pub stems: Vec<SeparatedStemPrediction>,
}

/// Request for audio generation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioGenerationRequest {
    /// Text prompt.
    pub prompt: String,
    /// Requested duration in seconds.
    #[serde(default = "default_generation_duration")]
    pub duration_seconds: f32,
    /// Model selection.
    #[serde(default)]
    pub model: AudioModelSelection,
}

/// Response for audio generation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioGenerationResponse {
    /// Accepted flag for generated package surfaces.
    pub accepted: bool,
    /// Operation name.
    pub operation: String,
    /// Selected model id.
    pub model_id: String,
    /// Runtime used.
    pub runtime: AudioRuntime,
    /// Prompt accepted by the model layer.
    pub prompt: String,
    /// Planned duration in seconds.
    pub duration_seconds: f32,
    /// Human-readable generation plan.
    pub plan: String,
}

/// Returns the shared model catalog, optionally filtered by task.
pub fn model_catalog(task: Option<AudioTask>) -> Vec<AudioModelMetadata> {
    let models = vec![
        metadata(
            "ast-audioset",
            "MIT/ast-finetuned-audioset-10-10-0.4593",
            AudioTask::AudioClassification,
            AudioRuntime::Onnx,
            false,
            Some("spectral-audio-classifier"),
            Some("Hugging Face preset metadata is registered; native AST execution is explicit/fallback-gated."),
        ),
        metadata(
            "spectral-audio-classifier",
            "spectral_heuristic",
            AudioTask::AudioClassification,
            AudioRuntime::Spectral,
            true,
            None,
            Some("Uses feature summaries for deterministic local classification."),
        ),
        metadata(
            "audioset-event-detector",
            "MIT/ast-finetuned-audioset-10-10-0.4593",
            AudioTask::AudioEventDetection,
            AudioRuntime::Onnx,
            false,
            Some("energy-event-detector"),
            Some("Windowed event schema is available; native model execution is not bundled."),
        ),
        metadata(
            "energy-event-detector",
            "energy_threshold",
            AudioTask::AudioEventDetection,
            AudioRuntime::Heuristic,
            true,
            None,
            Some("Detects high-energy windows from supplied frame summaries."),
        ),
        metadata(
            "clap-htsat-unfused",
            "laion/clap-htsat-unfused",
            AudioTask::AudioEmbedding,
            AudioRuntime::Onnx,
            false,
            Some("spectral-audio-embedding"),
            Some("Use imported embeddings or explicit fallback until CLAP execution is wired."),
        ),
        metadata(
            "spectral-audio-embedding",
            "spectral_embedding",
            AudioTask::AudioEmbedding,
            AudioRuntime::Spectral,
            true,
            None,
            Some("Builds deterministic vectors from feature summaries."),
        ),
        metadata(
            "whisper-tiny-en",
            "openai/whisper-tiny.en",
            AudioTask::SpeechRecognition,
            AudioRuntime::WhisperCpp,
            false,
            None,
            Some("Use text-transcripts native whisper.cpp support or imported segments for execution."),
        ),
        metadata(
            "wav2vec2-base-960h",
            "facebook/wav2vec2-base-960h",
            AudioTask::SpeechRecognition,
            AudioRuntime::Onnx,
            false,
            Some("whisper-tiny-en"),
            Some("ASR schema and imported transcript support are available; native ONNX decoding is not wired."),
        ),
        metadata(
            "pyannote-speaker-diarization-3.1",
            "pyannote/speaker-diarization-3.1",
            AudioTask::SpeakerDiarization,
            AudioRuntime::External,
            false,
            Some("single-speaker-heuristic"),
            Some("Gated external model; use imported segments or heuristic fallback."),
        ),
        metadata(
            "single-speaker-heuristic",
            "single_speaker",
            AudioTask::SpeakerDiarization,
            AudioRuntime::Heuristic,
            true,
            None,
            Some("Creates one speaker segment over the provided duration."),
        ),
        metadata(
            "demucs-music-separation",
            "facebook/demucs",
            AudioTask::SourceSeparation,
            AudioRuntime::External,
            false,
            Some("demucs-stem-plan"),
            Some("Use audio-analysis-separation for command-backed Demucs execution."),
        ),
        metadata(
            "demucs-stem-plan",
            "stem_plan",
            AudioTask::SourceSeparation,
            AudioRuntime::Heuristic,
            true,
            None,
            Some("Returns requested stem descriptors without writing audio files."),
        ),
        metadata(
            "musicgen-small",
            "facebook/musicgen-small",
            AudioTask::AudioGeneration,
            AudioRuntime::External,
            false,
            None,
            Some("Prompt schema is available; waveform generation is outside the default package app."),
        ),
    ];

    models
        .into_iter()
        .filter(|model| task.map(|task| task == model.task).unwrap_or(true))
        .collect()
}

/// Runs audio classification from imported predictions or an explicit fallback.
pub fn classify_audio(request: AudioClassificationRequest) -> Result<AudioClassificationResponse> {
    let model_id = request
        .model
        .model_id
        .clone()
        .unwrap_or_else(|| "ast-audioset".to_string());

    if !request.imported_predictions.is_empty() {
        return Ok(AudioClassificationResponse {
            accepted: true,
            operation: "classify".to_string(),
            model_id,
            runtime: AudioRuntime::ImportedPredictions,
            predictions: normalize_predictions(request.imported_predictions, request.top_k),
        });
    }

    match request.model.fallback_policy {
        FallbackPolicy::FastFallback | FallbackPolicy::HeuristicFallback => {
            let features = request.features.unwrap_or_default();
            Ok(AudioClassificationResponse {
                accepted: true,
                operation: "classify".to_string(),
                model_id,
                runtime: AudioRuntime::Spectral,
                predictions: spectral_label_scores(&features, &request.labels, request.top_k),
            })
        }
        FallbackPolicy::Error => unsupported_runtime(
            "native audio classification requires imported predictions or an explicit fallback policy",
        ),
    }
}

/// Runs event detection from imported predictions or energy threshold fallback.
pub fn detect_audio_events(
    request: AudioEventDetectionRequest,
) -> Result<AudioEventDetectionResponse> {
    let model_id = request
        .model
        .model_id
        .clone()
        .unwrap_or_else(|| "audioset-event-detector".to_string());

    if !request.imported_predictions.is_empty() {
        let events = request
            .imported_predictions
            .into_iter()
            .filter_map(|prediction| {
                Some(AudioEventPrediction {
                    label: prediction.label,
                    score: prediction.score,
                    start_seconds: prediction.start_seconds?,
                    end_seconds: prediction.end_seconds?,
                })
            })
            .collect::<Vec<_>>();
        return Ok(AudioEventDetectionResponse {
            accepted: true,
            operation: "events".to_string(),
            model_id,
            runtime: AudioRuntime::ImportedPredictions,
            events,
        });
    }

    match request.model.fallback_policy {
        FallbackPolicy::FastFallback | FallbackPolicy::HeuristicFallback => {
            let events = request
                .frames
                .into_iter()
                .filter(|frame| frame.rms >= request.threshold || frame.peak >= request.threshold)
                .map(|frame| AudioEventPrediction {
                    label: "high_energy".to_string(),
                    score: frame.rms.max(frame.peak).clamp(0.0, 1.0),
                    start_seconds: frame.start_seconds,
                    end_seconds: frame.end_seconds,
                })
                .collect::<Vec<_>>();
            Ok(AudioEventDetectionResponse {
                accepted: true,
                operation: "events".to_string(),
                model_id,
                runtime: AudioRuntime::Heuristic,
                events,
            })
        }
        FallbackPolicy::Error => unsupported_runtime(
            "native audio event detection requires imported predictions or an explicit fallback policy",
        ),
    }
}

/// Creates imported or deterministic fallback audio embeddings.
pub fn embed_audio(request: AudioEmbeddingRequest) -> Result<AudioEmbeddingResponse> {
    let model_id = request
        .model
        .model_id
        .clone()
        .unwrap_or_else(|| "clap-htsat-unfused".to_string());

    if !request.imported_embeddings.is_empty() {
        let dimensions = request
            .imported_embeddings
            .first()
            .map(Vec::len)
            .unwrap_or_default();
        return Ok(AudioEmbeddingResponse {
            accepted: true,
            operation: "embed".to_string(),
            model_id,
            runtime: AudioRuntime::ImportedPredictions,
            dimensions,
            embeddings: request.imported_embeddings,
        });
    }

    match request.model.fallback_policy {
        FallbackPolicy::FastFallback | FallbackPolicy::HeuristicFallback => {
            if request.features.is_empty() {
                return Err(DetectError::InvalidArgument(
                    "audio embedding request must include features or imported embeddings"
                        .to_string(),
                ));
            }
            let dimensions = request.dimensions.max(1);
            let embeddings = request
                .features
                .iter()
                .map(|features| spectral_embedding(features, dimensions, request.normalize))
                .collect::<Vec<_>>();
            Ok(AudioEmbeddingResponse {
                accepted: true,
                operation: "embed".to_string(),
                model_id,
                runtime: AudioRuntime::Spectral,
                dimensions,
                embeddings,
            })
        }
        FallbackPolicy::Error => unsupported_runtime(
            "native audio embedding requires imported embeddings or an explicit fallback policy",
        ),
    }
}

/// Handles ASR from imported transcript segments.
pub fn transcribe_audio(request: SpeechRecognitionRequest) -> Result<SpeechRecognitionResponse> {
    let model_id = request
        .model
        .model_id
        .clone()
        .unwrap_or_else(|| "whisper-tiny-en".to_string());

    if !request.imported_segments.is_empty() {
        let text = request
            .imported_segments
            .iter()
            .map(|segment| segment.text.trim())
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        return Ok(SpeechRecognitionResponse {
            accepted: true,
            operation: "transcribe".to_string(),
            model_id,
            runtime: AudioRuntime::ImportedPredictions,
            text,
            segments: request.imported_segments,
        });
    }

    unsupported_runtime(
        "native speech recognition requires text-transcripts native whisper.cpp execution or imported segments",
    )
}

/// Handles speaker diarization from imported segments or one-speaker fallback.
pub fn diarize_speakers(request: SpeakerDiarizationRequest) -> Result<SpeakerDiarizationResponse> {
    let model_id = request
        .model
        .model_id
        .clone()
        .unwrap_or_else(|| "pyannote-speaker-diarization-3.1".to_string());

    if !request.imported_segments.is_empty() {
        return Ok(SpeakerDiarizationResponse {
            accepted: true,
            operation: "diarize".to_string(),
            model_id,
            runtime: AudioRuntime::ImportedPredictions,
            segments: request.imported_segments,
        });
    }

    match request.model.fallback_policy {
        FallbackPolicy::FastFallback | FallbackPolicy::HeuristicFallback => {
            let duration = request.duration_seconds.unwrap_or(0.0).max(0.0);
            Ok(SpeakerDiarizationResponse {
                accepted: true,
                operation: "diarize".to_string(),
                model_id,
                runtime: AudioRuntime::Heuristic,
                segments: vec![SpeakerSegmentPrediction {
                    speaker: "speaker_0".to_string(),
                    start_seconds: 0.0,
                    end_seconds: duration,
                    score: Some(1.0),
                }],
            })
        }
        FallbackPolicy::Error => unsupported_runtime(
            "native speaker diarization requires imported segments or an explicit fallback policy",
        ),
    }
}

/// Handles source separation from imported stems or stem planning fallback.
pub fn separate_sources(request: SourceSeparationRequest) -> Result<SourceSeparationResponse> {
    let model_id = request
        .model
        .model_id
        .clone()
        .unwrap_or_else(|| "demucs-music-separation".to_string());

    if !request.imported_stems.is_empty() {
        return Ok(SourceSeparationResponse {
            accepted: true,
            operation: "separate".to_string(),
            model_id,
            runtime: AudioRuntime::ImportedPredictions,
            stems: request.imported_stems,
        });
    }

    match request.model.fallback_policy {
        FallbackPolicy::FastFallback | FallbackPolicy::HeuristicFallback => {
            let stem_names = if request.stems.is_empty() {
                vec!["vocals", "drums", "bass", "other"]
            } else {
                request.stems.iter().map(String::as_str).collect::<Vec<_>>()
            };
            Ok(SourceSeparationResponse {
                accepted: true,
                operation: "separate".to_string(),
                model_id,
                runtime: AudioRuntime::Heuristic,
                stems: stem_names
                    .into_iter()
                    .map(|stem| SeparatedStemPrediction {
                        stem: stem.to_string(),
                        uri: None,
                        score: None,
                    })
                    .collect(),
            })
        }
        FallbackPolicy::Error => unsupported_runtime(
            "native source separation requires audio-analysis-separation execution or an explicit fallback policy",
        ),
    }
}

/// Validates an audio generation request and returns a generation plan.
pub fn generate_audio(request: AudioGenerationRequest) -> Result<AudioGenerationResponse> {
    ensure_non_empty(&request.prompt, "prompt")?;
    let model_id = request
        .model
        .model_id
        .clone()
        .unwrap_or_else(|| "musicgen-small".to_string());
    Ok(AudioGenerationResponse {
        accepted: true,
        operation: "generate".to_string(),
        model_id,
        runtime: AudioRuntime::External,
        prompt: request.prompt.clone(),
        duration_seconds: request.duration_seconds.max(0.1),
        plan: format!(
            "Queue `{}` for external audio generation; requested duration {:.1}s.",
            request.prompt,
            request.duration_seconds.max(0.1)
        ),
    })
}

/// Parses an audio task path segment.
pub fn parse_task(input: &str) -> Option<AudioTask> {
    AudioTask::ALL
        .iter()
        .copied()
        .find(|task| {
            task.path_segment() == input || format!("{task:?}").eq_ignore_ascii_case(input)
        })
        .or_else(|| match input {
            "audio_classification" | "classification" => Some(AudioTask::AudioClassification),
            "audio_event_detection" | "event_detection" => Some(AudioTask::AudioEventDetection),
            "audio_embedding" | "embedding" => Some(AudioTask::AudioEmbedding),
            "speech_recognition" | "asr" => Some(AudioTask::SpeechRecognition),
            "speaker_diarization" | "speakers" => Some(AudioTask::SpeakerDiarization),
            "source_separation" | "separation" => Some(AudioTask::SourceSeparation),
            "audio_generation" | "synthesis" => Some(AudioTask::AudioGeneration),
            _ => None,
        })
}

/// Returns preset ids registered in video-analysis-models for audio tasks.
pub fn registered_audio_presets() -> Vec<String> {
    ModelPreset::ALL
        .iter()
        .map(|preset| preset.as_str().to_string())
        .filter(|preset| {
            preset.contains("audio")
                || preset.contains("ast")
                || preset.contains("clap")
                || preset.contains("whisper")
                || preset.contains("wav2vec")
                || preset.contains("pyannote")
                || preset.contains("demucs")
                || preset.contains("musicgen")
        })
        .collect()
}

/// Returns a compact schema summary for OpenAPI metadata.
pub fn schema_summary() -> serde_json::Value {
    serde_json::json!({
        "tasks": AudioTask::ALL.iter().map(|task| task.path_segment()).collect::<Vec<_>>(),
        "models": model_catalog(None),
        "registeredPresets": registered_audio_presets()
    })
}

fn metadata(
    id: &str,
    model_id: &str,
    task: AudioTask,
    runtime: AudioRuntime,
    supported: bool,
    fallback: Option<&str>,
    note: Option<&str>,
) -> AudioModelMetadata {
    AudioModelMetadata {
        id: id.to_string(),
        model_id: model_id.to_string(),
        task,
        runtime,
        supported,
        fallback: fallback.map(str::to_string),
        note: note.map(str::to_string),
    }
}

fn default_top_k() -> usize {
    3
}

fn default_event_threshold() -> f32 {
    0.2
}

fn default_embedding_dimensions() -> usize {
    128
}

fn default_generation_duration() -> f32 {
    8.0
}

fn default_true() -> bool {
    true
}

fn ensure_non_empty(value: &str, name: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(DetectError::InvalidArgument(format!(
            "request body must include a non-empty `{name}` string"
        )));
    }
    Ok(())
}

fn unsupported_runtime<T>(message: &str) -> Result<T> {
    Err(DetectError::InvalidArgument(format!(
        "unsupported_runtime: {message}"
    )))
}

fn normalize_predictions(
    predictions: Vec<ImportedAudioPrediction>,
    top_k: usize,
) -> Vec<AudioClassPrediction> {
    let mut predictions = predictions
        .into_iter()
        .map(|prediction| AudioClassPrediction {
            label: normalize_label(&prediction.label),
            score: prediction.score.clamp(0.0, 1.0),
        })
        .collect::<Vec<_>>();
    predictions.sort_by(|left, right| {
        right
            .score
            .total_cmp(&left.score)
            .then_with(|| left.label.cmp(&right.label))
    });
    predictions.truncate(top_k.max(1));
    predictions
}

fn spectral_label_scores(
    features: &AudioFeatureSummary,
    labels: &[String],
    top_k: usize,
) -> Vec<AudioClassPrediction> {
    let defaults = if features.rms < 0.02 && features.peak < 0.05 {
        vec![
            ("silence".to_string(), 0.92),
            ("background".to_string(), 0.35),
        ]
    } else if features.dominant_frequency_hz > 180.0 && features.spectral_centroid_hz > 1_200.0 {
        vec![
            ("speech".to_string(), 0.72),
            ("foreground".to_string(), 0.58),
        ]
    } else if features.rms > 0.25 || features.peak > 0.6 {
        vec![
            ("music".to_string(), 0.68),
            ("loud_event".to_string(), 0.62),
        ]
    } else {
        vec![
            ("ambient".to_string(), 0.57),
            ("background".to_string(), 0.42),
        ]
    };

    let mut predictions = if labels.is_empty() {
        defaults
            .into_iter()
            .map(|(label, score)| AudioClassPrediction { label, score })
            .collect::<Vec<_>>()
    } else {
        labels
            .iter()
            .map(|label| {
                let normalized = normalize_label(label);
                let score = match normalized.as_str() {
                    "silence" => 1.0 - features.rms.clamp(0.0, 1.0),
                    "speech" | "voice" => {
                        (features.spectral_centroid_hz / 3_000.0).clamp(0.0, 1.0) * 0.75
                    }
                    "music" => (features.peak + features.rms)
                        .mul_add(0.5, 0.1)
                        .clamp(0.0, 1.0),
                    "noise" | "background" => (features.zero_crossing_rate + 0.25).clamp(0.0, 1.0),
                    _ => 0.35,
                };
                AudioClassPrediction {
                    label: normalized,
                    score,
                }
            })
            .collect::<Vec<_>>()
    };
    predictions.sort_by(|left, right| right.score.total_cmp(&left.score));
    predictions.truncate(top_k.max(1));
    predictions
}

fn spectral_embedding(
    features: &AudioFeatureSummary,
    dimensions: usize,
    normalize: bool,
) -> Vec<f32> {
    let base = [
        features.duration_seconds / 60.0,
        features.sample_rate as f32 / 48_000.0,
        features.rms,
        features.peak,
        features.zero_crossing_rate,
        features.dominant_frequency_hz / 8_000.0,
        features.spectral_centroid_hz / 8_000.0,
    ];
    let mut values = (0..dimensions)
        .map(|index| {
            let seed = base[index % base.len()];
            let phase = (index as f32 + 1.0) * 0.173;
            (seed + phase.sin() * 0.05).clamp(-1.0, 1.0)
        })
        .collect::<Vec<_>>();
    if normalize {
        l2_normalize(&mut values);
    }
    values
}

fn l2_normalize(values: &mut [f32]) {
    let norm = values.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm > 0.0 {
        for value in values {
            *value /= norm;
        }
    }
}

fn normalize_label(label: &str) -> String {
    label.trim().to_ascii_lowercase().replace(' ', "_")
}

impl Default for AudioFeatureSummary {
    fn default() -> Self {
        Self {
            duration_seconds: 0.0,
            sample_rate: 0,
            rms: 0.0,
            peak: 0.0,
            zero_crossing_rate: 0.0,
            dominant_frequency_hz: 0.0,
            spectral_centroid_hz: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_catalog_lists_huggingface_audio_tasks() {
        let models = model_catalog(None);
        assert!(models.iter().any(|model| model.id == "ast-audioset"));
        assert!(models
            .iter()
            .any(|model| model.task == AudioTask::SpeechRecognition));
        assert!(registered_audio_presets()
            .iter()
            .any(|preset| preset.contains("whisper")));
    }

    #[test]
    fn classification_uses_spectral_fallback() {
        let response = classify_audio(AudioClassificationRequest {
            labels: vec!["speech".to_string(), "silence".to_string()],
            features: Some(AudioFeatureSummary {
                rms: 0.7,
                peak: 0.8,
                spectral_centroid_hz: 1_800.0,
                ..AudioFeatureSummary::default()
            }),
            model: AudioModelSelection {
                fallback_policy: FallbackPolicy::HeuristicFallback,
                ..AudioModelSelection::default()
            },
            source: None,
            top_k: 2,
            imported_predictions: Vec::new(),
        })
        .unwrap();

        assert_eq!(response.operation, "classify");
        assert_eq!(response.runtime, AudioRuntime::Spectral);
        assert_eq!(response.predictions[0].label, "speech");
    }

    #[test]
    fn embeddings_are_normalized() {
        let response = embed_audio(AudioEmbeddingRequest {
            features: vec![AudioFeatureSummary {
                duration_seconds: 3.0,
                sample_rate: 16_000,
                rms: 0.2,
                peak: 0.6,
                ..AudioFeatureSummary::default()
            }],
            dimensions: 8,
            normalize: true,
            model: AudioModelSelection {
                fallback_policy: FallbackPolicy::FastFallback,
                ..AudioModelSelection::default()
            },
            imported_embeddings: Vec::new(),
        })
        .unwrap();

        let norm = response.embeddings[0]
            .iter()
            .map(|value| value * value)
            .sum::<f32>()
            .sqrt();
        assert!((norm - 1.0).abs() < 0.0001);
    }

    #[test]
    fn transcript_imports_join_text() {
        let response = transcribe_audio(SpeechRecognitionRequest {
            source: None,
            language: Some("en".to_string()),
            model: AudioModelSelection::default(),
            imported_segments: vec![
                TranscriptSegmentPrediction {
                    index: 0,
                    start_seconds: Some(0.0),
                    end_seconds: Some(1.0),
                    text: "hello".to_string(),
                    score: Some(0.9),
                },
                TranscriptSegmentPrediction {
                    index: 1,
                    start_seconds: Some(1.0),
                    end_seconds: Some(2.0),
                    text: "audio".to_string(),
                    score: Some(0.8),
                },
            ],
        })
        .unwrap();

        assert_eq!(response.text, "hello audio");
        assert_eq!(response.runtime, AudioRuntime::ImportedPredictions);
    }
}
