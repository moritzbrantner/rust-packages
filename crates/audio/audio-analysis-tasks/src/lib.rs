#![doc = include_str!("../README.md")]

pub mod surface;
use model_runtime::ModelPreset;
use serde::{Deserialize, Serialize};

pub use audio_analysis_recognition::{
    classify_audio, detect_audio_events, embed_audio, transcribe_audio, AudioClassPrediction,
    AudioClassificationRequest, AudioClassificationResponse, AudioEmbeddingRequest,
    AudioEmbeddingResponse, AudioEventDetectionRequest, AudioEventDetectionResponse,
    AudioEventPrediction, AudioFeatureFrame, AudioFeatureSummary, AudioRuntime,
    AudioRuntimeSelection, FallbackPolicy, ImportedAudioPrediction, SpeechRecognitionRequest,
    SpeechRecognitionResponse, TranscriptSegmentContract, TranscriptionContract,
};
pub use audio_analysis_separation::{
    separate_sources, SeparatedStemPrediction, SourceSeparationBackend, SourceSeparationRequest,
    SourceSeparationResponse,
};
pub use audio_analysis_speakers::{
    diarize_speakers, SpeakerDiarizationRequest, SpeakerDiarizationResponse,
    SpeakerSegmentPrediction,
};
pub use audio_analysis_synthesis::{
    generate_audio, AudioGenerationRequest, AudioGenerationResponse,
};

/// Audio task families exposed by the aggregate task layer.
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

/// Shared task runtime metadata for UI and CLI discovery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioTaskCatalogEntry {
    /// Stable local entry id.
    pub id: String,
    /// Upstream model id or strategy id.
    pub model_id: String,
    /// Task family.
    pub task: AudioTask,
    /// Preferred runtime.
    pub runtime: AudioRuntime,
    /// Whether the runtime is available in the default contributor build.
    pub supported: bool,
    /// Optional fallback entry id.
    pub fallback: Option<String>,
    /// Human-readable note for unsupported or fallback-only paths.
    pub note: Option<String>,
}

/// Compatibility alias for callers migrating from model-shaped metadata names.
pub type AudioModelMetadata = AudioTaskCatalogEntry;

/// Compatibility alias for callers migrating from model-shaped selection names.
#[deprecated(note = "use AudioRuntimeSelection")]
pub type AudioModelSelection = AudioRuntimeSelection;

/// Returns the audio task catalog, optionally filtered by task.
pub fn audio_task_catalog(task: Option<AudioTask>) -> Vec<AudioTaskCatalogEntry> {
    let entries = vec![
        entry(
            "ast-audioset",
            "MIT/ast-finetuned-audioset-10-10-0.4593",
            AudioTask::AudioClassification,
            AudioRuntime::Onnx,
            false,
            Some("spectral-audio-classifier"),
            Some("Hugging Face preset metadata is registered; native AST execution is explicit/fallback-gated."),
        ),
        entry(
            "spectral-audio-classifier",
            "spectral_heuristic",
            AudioTask::AudioClassification,
            AudioRuntime::Spectral,
            true,
            None,
            Some("Uses feature summaries for deterministic local classification."),
        ),
        entry(
            "audioset-event-detector",
            "MIT/ast-finetuned-audioset-10-10-0.4593",
            AudioTask::AudioEventDetection,
            AudioRuntime::Onnx,
            false,
            Some("energy-event-detector"),
            Some("Windowed event schema is available; native model execution is not bundled."),
        ),
        entry(
            "energy-event-detector",
            "energy_threshold",
            AudioTask::AudioEventDetection,
            AudioRuntime::Heuristic,
            true,
            None,
            Some("Detects high-energy windows from supplied frame summaries."),
        ),
        entry(
            "clap-htsat-unfused",
            "laion/clap-htsat-unfused",
            AudioTask::AudioEmbedding,
            AudioRuntime::Onnx,
            false,
            Some("spectral-audio-embedding"),
            Some("Use imported embeddings or explicit fallback until CLAP execution is wired."),
        ),
        entry(
            "spectral-audio-embedding",
            "spectral_embedding",
            AudioTask::AudioEmbedding,
            AudioRuntime::Spectral,
            true,
            None,
            Some("Builds deterministic vectors from feature summaries."),
        ),
        entry(
            "whisper-tiny-en",
            "openai/whisper-tiny.en",
            AudioTask::SpeechRecognition,
            AudioRuntime::WhisperCpp,
            false,
            None,
            Some("Use text-transcripts native whisper.cpp support or imported segments for execution."),
        ),
        entry(
            "wav2vec2-base-960h",
            "facebook/wav2vec2-base-960h",
            AudioTask::SpeechRecognition,
            AudioRuntime::Onnx,
            false,
            Some("whisper-tiny-en"),
            Some("ASR schema and imported transcript support are available; native ONNX decoding is not wired."),
        ),
        entry(
            "pyannote-speaker-diarization-3.1",
            "pyannote/speaker-diarization-3.1",
            AudioTask::SpeakerDiarization,
            AudioRuntime::External,
            false,
            Some("single-speaker-heuristic"),
            Some("Gated external model; use imported segments or heuristic fallback."),
        ),
        entry(
            "single-speaker-heuristic",
            "single_speaker",
            AudioTask::SpeakerDiarization,
            AudioRuntime::Heuristic,
            true,
            None,
            Some("Creates one speaker segment over the provided duration."),
        ),
        entry(
            "demucs-music-separation",
            "facebook/demucs",
            AudioTask::SourceSeparation,
            AudioRuntime::Demucs,
            false,
            Some("demucs-stem-plan"),
            Some("Use audio-analysis-separation for command-backed Demucs execution."),
        ),
        entry(
            "demucs-stem-plan",
            "stem_plan",
            AudioTask::SourceSeparation,
            AudioRuntime::Heuristic,
            true,
            None,
            Some("Returns requested stem descriptors without writing audio files."),
        ),
        entry(
            "musicgen-small",
            "facebook/musicgen-small",
            AudioTask::AudioGeneration,
            AudioRuntime::External,
            false,
            None,
            Some("Prompt schema is available; waveform generation is outside the default package app."),
        ),
    ];

    entries
        .into_iter()
        .filter(|entry| task.map(|task| task == entry.task).unwrap_or(true))
        .collect()
}

/// Compatibility function name for routes that still expose `/api/models`.
#[deprecated(note = "use audio_task_catalog")]
pub fn model_catalog(task: Option<AudioTask>) -> Vec<AudioTaskCatalogEntry> {
    audio_task_catalog(task)
}

/// Parses an audio task path segment.
pub fn parse_task(input: &str) -> Option<AudioTask> {
    AudioTask::ALL
        .iter()
        .copied()
        .find(|task| {
            task.path_segment() == input || format!("{task:?}").eq_ignore_ascii_case(input)
        })
        .or(match input {
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

/// Returns preset ids registered in model-runtime for audio tasks.
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
        "runtimes": audio_task_catalog(None),
        "models": audio_task_catalog(None),
        "registeredPresets": registered_audio_presets()
    })
}

fn entry(
    id: &str,
    model_id: &str,
    task: AudioTask,
    runtime: AudioRuntime,
    supported: bool,
    fallback: Option<&str>,
    note: Option<&str>,
) -> AudioTaskCatalogEntry {
    AudioTaskCatalogEntry {
        id: id.to_string(),
        model_id: model_id.to_string(),
        task,
        runtime,
        supported,
        fallback: fallback.map(str::to_string),
        note: note.map(str::to_string),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_lists_audio_tasks() {
        let entries = audio_task_catalog(None);
        assert!(entries.iter().any(|entry| entry.id == "ast-audioset"));
        assert!(entries
            .iter()
            .any(|entry| entry.task == AudioTask::SpeechRecognition));
        assert!(registered_audio_presets()
            .iter()
            .any(|preset| preset.contains("whisper")));
    }

    #[test]
    fn deprecated_model_catalog_delegates_to_task_catalog() {
        #[allow(deprecated)]
        let models = model_catalog(Some(AudioTask::AudioEmbedding));
        assert_eq!(models, audio_task_catalog(Some(AudioTask::AudioEmbedding)));
    }
}
