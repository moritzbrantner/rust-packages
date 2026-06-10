#![doc = include_str!("../README.md")]

pub mod surface;

#[cfg(feature = "alignment")]
mod ctc_alignment;
mod native_audio;
mod native_bundles;
mod native_device;
#[cfg(feature = "alignment")]
mod native_wav2vec2;
#[cfg(feature = "alignment")]
mod native_wav2vec2_model;
#[cfg(feature = "candle")]
mod native_whisper;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use text_transcripts::{
    normalize_transcription_contract, TranscriptWordContract, TranscriptionContract,
};
use video_analysis_core::{DetectError, Result};

#[cfg(feature = "diarization")]
pub use audio_analysis_speakers::{SpeakerDiarizationResponse, SpeakerSegmentPrediction};

#[cfg(not(feature = "diarization"))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeakerDiarizationResponse {
    pub accepted: bool,
    pub operation: String,
    pub model_id: String,
    pub runtime: String,
    pub segments: Vec<SpeakerSegmentPrediction>,
}

#[cfg(not(feature = "diarization"))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeakerSegmentPrediction {
    pub speaker: String,
    pub start_seconds: f32,
    pub end_seconds: f32,
    #[serde(default)]
    pub score: Option<f32>,
}

/// Request for an audio/video-to-text transcription pipeline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionPipelineRequest {
    pub source: TranscriptionSource,
    pub provider: TranscriptionProviderSelection,
    #[serde(default)]
    pub vad: VadOptions,
    #[serde(default)]
    pub alignment: AlignmentOptions,
    #[serde(default)]
    pub diarization: DiarizationOptions,
    #[serde(default)]
    pub output: TranscriptionOutputOptions,
}

/// Source accepted by transcription providers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", untagged)]
pub enum TranscriptionSource {
    Path {
        path: PathBuf,
    },
    Samples {
        samples: Vec<f32>,
        #[serde(rename = "sampleRate", alias = "sample_rate")]
        sample_rate: u32,
        channels: u16,
        #[serde(default)]
        source: Option<String>,
    },
}

impl TranscriptionSource {
    fn path(&self) -> Result<&Path> {
        match self {
            Self::Path { path } => Ok(path),
            Self::Samples { .. } => Err(invalid_request(
                "external command transcription requires a path source",
            )),
        }
    }
}

/// Provider selection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum TranscriptionProviderSelection {
    #[serde(rename = "candleWhisper", alias = "candle-whisper")]
    CandleWhisper(CandleWhisperOptions),
    #[serde(rename = "whisperCpp", alias = "whisper-cpp")]
    WhisperCpp(WhisperCppProviderOptions),
    #[serde(rename = "externalWhisperX", alias = "whisperx")]
    ExternalWhisperX(WhisperXCommandOptions),
}

impl TranscriptionProviderSelection {
    pub fn provider_id(&self) -> &'static str {
        match self {
            Self::CandleWhisper(_) => "candle-whisper",
            Self::WhisperCpp(_) => "whisper-cpp",
            Self::ExternalWhisperX(_) => "whisperx-command",
        }
    }

    pub fn model_id(&self) -> &str {
        match self {
            Self::CandleWhisper(options) => &options.model_id,
            Self::WhisperCpp(options) => &options.model_id,
            Self::ExternalWhisperX(options) => &options.model,
        }
    }
}

/// Options for the Candle Whisper native provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CandleWhisperOptions {
    #[serde(default = "default_candle_whisper_model")]
    pub model_id: String,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub device: NativeDevicePreference,
    #[serde(default)]
    pub model_bundle: Option<PathBuf>,
    #[serde(default)]
    pub batch_chunks: bool,
    #[serde(default)]
    pub max_batch_size: Option<usize>,
}

impl Default for CandleWhisperOptions {
    fn default() -> Self {
        Self {
            model_id: default_candle_whisper_model(),
            language: None,
            device: NativeDevicePreference::Auto,
            model_bundle: None,
            batch_chunks: true,
            max_batch_size: Some(4),
        }
    }
}

fn default_candle_whisper_model() -> String {
    "openai/whisper-large-v3-turbo".to_string()
}

/// Options for native whisper.cpp compatibility.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhisperCppProviderOptions {
    #[serde(default = "default_whisper_cpp_model")]
    pub model_id: String,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub model_path: Option<PathBuf>,
}

impl Default for WhisperCppProviderOptions {
    fn default() -> Self {
        Self {
            model_id: default_whisper_cpp_model(),
            language: None,
            model_path: None,
        }
    }
}

fn default_whisper_cpp_model() -> String {
    "large-v3-turbo".to_string()
}

/// Native execution device preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NativeDevicePreference {
    #[default]
    Auto,
    Cpu,
    Cuda,
}

/// Options for the external WhisperX command provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhisperXCommandOptions {
    pub command: PathBuf,
    pub model: String,
    #[serde(default)]
    pub language: Option<String>,
    pub device: WhisperXDevice,
    #[serde(default)]
    pub compute_type: Option<String>,
    #[serde(default)]
    pub batch_size: Option<usize>,
    #[serde(default)]
    pub align_model: Option<String>,
    #[serde(default)]
    pub diarize: bool,
    #[serde(default)]
    pub min_speakers: Option<usize>,
    #[serde(default)]
    pub max_speakers: Option<usize>,
    #[serde(default)]
    pub hf_token_env: Option<String>,
    #[serde(default)]
    pub output_dir: Option<PathBuf>,
    #[serde(default)]
    pub timeout_seconds: Option<u64>,
    #[serde(default)]
    pub extra_args: Vec<String>,
}

/// Backward-compatible name for the external WhisperX provider options.
pub type WhisperXOptions = WhisperXCommandOptions;

impl Default for WhisperXCommandOptions {
    fn default() -> Self {
        Self {
            command: PathBuf::from("whisperx"),
            model: "large-v2".to_string(),
            language: None,
            device: WhisperXDevice::Cpu,
            compute_type: None,
            batch_size: None,
            align_model: None,
            diarize: false,
            min_speakers: None,
            max_speakers: None,
            hf_token_env: None,
            output_dir: None,
            timeout_seconds: None,
            extra_args: Vec::new(),
        }
    }
}

/// WhisperX device selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WhisperXDevice {
    Cpu,
    Cuda,
}

impl WhisperXDevice {
    fn as_str(self) -> &'static str {
        match self {
            Self::Cpu => "cpu",
            Self::Cuda => "cuda",
        }
    }
}

/// VAD options used before ASR chunking.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VadOptions {
    #[serde(default = "default_vad_enabled")]
    pub enabled: bool,
    #[serde(default = "default_vad_threshold")]
    pub rms_threshold: f32,
    #[serde(default = "default_vad_frame_seconds")]
    pub frame_seconds: f64,
    #[serde(default = "default_vad_hop_seconds")]
    pub hop_seconds: f64,
    #[serde(default = "default_vad_min_speech_seconds")]
    pub min_speech_seconds: f64,
    #[serde(default = "default_vad_padding_seconds")]
    pub padding_seconds: f64,
    #[serde(default = "default_vad_merge_gap_seconds")]
    pub merge_gap_seconds: f64,
    #[serde(default = "default_vad_max_chunk_seconds")]
    pub max_chunk_seconds: f64,
}

impl Default for VadOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            rms_threshold: default_vad_threshold(),
            frame_seconds: default_vad_frame_seconds(),
            hop_seconds: default_vad_hop_seconds(),
            min_speech_seconds: default_vad_min_speech_seconds(),
            padding_seconds: default_vad_padding_seconds(),
            merge_gap_seconds: default_vad_merge_gap_seconds(),
            max_chunk_seconds: default_vad_max_chunk_seconds(),
        }
    }
}

fn default_vad_enabled() -> bool {
    true
}
fn default_vad_threshold() -> f32 {
    0.01
}
fn default_vad_frame_seconds() -> f64 {
    0.03
}
fn default_vad_hop_seconds() -> f64 {
    0.01
}
fn default_vad_min_speech_seconds() -> f64 {
    0.08
}
fn default_vad_padding_seconds() -> f64 {
    0.02
}
fn default_vad_merge_gap_seconds() -> f64 {
    0.05
}
fn default_vad_max_chunk_seconds() -> f64 {
    30.0
}

/// Forced-alignment options.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlignmentOptions {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_alignment_model")]
    pub model_id: String,
    #[serde(default)]
    pub model_bundle: Option<PathBuf>,
}

impl Default for AlignmentOptions {
    fn default() -> Self {
        Self {
            enabled: false,
            model_id: default_alignment_model(),
            model_bundle: None,
        }
    }
}

fn default_alignment_model() -> String {
    "facebook/wav2vec2-base-960h".to_string()
}

/// Diarization options.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiarizationOptions {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_diarization_model")]
    pub model_id: String,
    #[serde(default)]
    pub speaker_embedding_model_bundle: Option<PathBuf>,
    #[serde(default)]
    pub speaker_embedding_model_file: Option<String>,
    #[serde(default)]
    pub speaker_embedding_input_name: Option<String>,
    #[serde(default)]
    pub speaker_embedding_output_name: Option<String>,
    #[serde(default)]
    pub speaker_embedding_dimension: Option<usize>,
    #[serde(default)]
    pub speaker_embedding_sample_rate: Option<u32>,
    #[serde(default)]
    pub min_speakers: Option<usize>,
    #[serde(default)]
    pub max_speakers: Option<usize>,
    #[serde(default)]
    pub assignment_policy: SpeakerAssignmentPolicy,
}

impl Default for DiarizationOptions {
    fn default() -> Self {
        Self {
            enabled: false,
            model_id: default_diarization_model(),
            speaker_embedding_model_bundle: None,
            speaker_embedding_model_file: None,
            speaker_embedding_input_name: None,
            speaker_embedding_output_name: None,
            speaker_embedding_dimension: None,
            speaker_embedding_sample_rate: None,
            min_speakers: None,
            max_speakers: None,
            assignment_policy: SpeakerAssignmentPolicy::Majority,
        }
    }
}

fn default_diarization_model() -> String {
    "native-spectral-speaker-baseline".to_string()
}

/// Speaker assignment policy for transcript words and segments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SpeakerAssignmentPolicy {
    #[default]
    Majority,
    NearestStart,
    StrictContained,
}

/// Output preferences for transcription.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionOutputOptions {
    #[serde(default = "default_output_formats")]
    pub formats: Vec<String>,
}

impl Default for TranscriptionOutputOptions {
    fn default() -> Self {
        Self {
            formats: default_output_formats(),
        }
    }
}

fn default_output_formats() -> Vec<String> {
    vec!["json".to_string()]
}

/// Artifact produced or discovered by a provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionArtifact {
    pub kind: String,
    pub path: PathBuf,
}

/// Response from a transcription pipeline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionPipelineResponse {
    pub accepted: bool,
    pub operation: String,
    pub provider: String,
    pub model_id: String,
    pub transcript: TranscriptionContract,
    pub vad_segments: Vec<SpeechActivitySegment>,
    pub alignment: Option<AlignmentSummary>,
    pub diarization: Option<SpeakerDiarizationResponse>,
    pub artifacts: Vec<TranscriptionArtifact>,
    pub diagnostics: Vec<String>,
}

/// Metadata-only provider plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionProviderPlan {
    pub provider_id: String,
    pub external_runtime: bool,
    pub wasm_supported: bool,
    pub primary: bool,
    pub setup: Vec<String>,
    pub diagnostics: Vec<String>,
}

/// Loaded audio for native providers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadedAudio {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
    #[serde(default)]
    pub source: Option<String>,
}

impl LoadedAudio {
    pub fn mono_16khz_from_source(source: &TranscriptionSource) -> Result<Self> {
        native_audio::mono_16khz_from_source(source)
    }

    pub fn duration_seconds(&self) -> f64 {
        if self.sample_rate == 0 || self.channels == 0 {
            return 0.0;
        }
        self.samples.len() as f64 / self.channels as f64 / self.sample_rate as f64
    }
}

/// A speech activity span used for ASR chunking.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeechActivitySegment {
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub score: f32,
}

impl SpeechActivitySegment {
    pub fn new(start_seconds: f64, end_seconds: f64, score: f32) -> Result<Self> {
        let segment = Self {
            start_seconds,
            end_seconds,
            score,
        };
        segment.validate()?;
        Ok(segment)
    }

    pub fn validate(&self) -> Result<()> {
        if !self.start_seconds.is_finite()
            || !self.end_seconds.is_finite()
            || !self.score.is_finite()
        {
            return Err(invalid_request(
                "speech activity segment values must be finite",
            ));
        }
        if self.start_seconds < 0.0 || self.end_seconds <= self.start_seconds {
            return Err(invalid_request(
                "speech activity segment must have non-negative start and positive duration",
            ));
        }
        Ok(())
    }
}

/// ASR provider request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AsrRequest {
    pub audio: LoadedAudio,
    pub chunks: Vec<SpeechActivitySegment>,
    pub language: Option<String>,
    pub model_id: String,
}

/// ASR provider response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AsrResponse {
    pub model_id: String,
    pub language: Option<String>,
    pub transcript: TranscriptionContract,
    #[serde(default)]
    pub diagnostics: Vec<String>,
}

/// Forced-alignment request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlignmentRequest {
    pub audio: LoadedAudio,
    pub transcript: TranscriptionContract,
    pub language: Option<String>,
    pub model_id: String,
}

/// Forced-alignment response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlignmentResponse {
    pub model_id: String,
    pub words: Vec<AlignedWord>,
    #[serde(default)]
    pub diagnostics: Vec<String>,
}

/// One aligned word timing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlignedWord {
    pub segment_index: u64,
    pub word_index: usize,
    pub text: String,
    pub start_seconds: f64,
    pub end_seconds: f64,
    #[serde(default)]
    pub confidence: Option<f32>,
}

/// Alignment summary included in pipeline responses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlignmentSummary {
    pub provider: String,
    pub model_id: String,
    pub word_count: usize,
}

/// VAD request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VadRequest {
    pub audio: LoadedAudio,
    pub options: VadOptions,
}

/// VAD response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VadResponse {
    pub segments: Vec<SpeechActivitySegment>,
    #[serde(default)]
    pub diagnostics: Vec<String>,
}

/// Trait for audio transcription providers.
pub trait AudioTranscriptionProvider {
    fn provider_id(&self) -> &str;
    fn transcribe(&mut self, request: AsrRequest) -> Result<AsrResponse>;
}

/// Trait for forced alignment providers.
pub trait ForcedAlignmentProvider {
    fn provider_id(&self) -> &str;
    fn align(&mut self, request: AlignmentRequest) -> Result<AlignmentResponse>;
}

/// Trait for transcription VAD providers.
pub trait TranscriptionVadProvider {
    fn provider_id(&self) -> &str;
    fn detect_speech(&mut self, request: VadRequest) -> Result<VadResponse>;
}

/// Trait for transcript diarization and speaker assignment providers.
pub trait TranscriptDiarizationProvider {
    fn provider_id(&self) -> &str;
    fn diarize(
        &mut self,
        audio: LoadedAudio,
        transcript: &TranscriptionContract,
        options: &DiarizationOptions,
    ) -> Result<SpeakerDiarizationResponse>;
}

/// Native deterministic speaker diarization adapter.
#[cfg(feature = "diarization")]
#[derive(Debug, Clone, Default)]
pub struct NativeSpeakerDiarizationProvider;

#[cfg(feature = "diarization")]
impl TranscriptDiarizationProvider for NativeSpeakerDiarizationProvider {
    fn provider_id(&self) -> &str {
        "native-speaker-diarization"
    }

    fn diarize(
        &mut self,
        audio: LoadedAudio,
        transcript: &TranscriptionContract,
        options: &DiarizationOptions,
    ) -> Result<SpeakerDiarizationResponse> {
        native_audio::validate_loaded_audio(&audio)?;
        if options.speaker_embedding_model_bundle.is_some() {
            return diarize_with_onnx_speaker_embeddings(audio, transcript, options);
        }
        let spans = speech_spans_from_transcript(transcript, audio.duration_seconds())?;
        if !spans.is_empty() {
            let speaker_audio =
                audio_analysis_speakers::SpeakerAudio::mono(&audio.samples, audio.sample_rate)?;
            let embedder = audio_analysis_speakers::SpectralSpeakerEmbedder::default();
            let vad = TranscriptSpeechSpanVad { spans };
            let mut diarizer = audio_analysis_speakers::WindowedSpeakerDiarizer::new(embedder, vad)
                .cluster_threshold(0.95)?;
            let result =
                audio_analysis_speakers::SpeakerDiarizer::diarize(&mut diarizer, &speaker_audio)?;
            return Ok(SpeakerDiarizationResponse {
                accepted: true,
                operation: "audio.speakers.diarize".to_string(),
                model_id: options.model_id.clone(),
                runtime: audio_analysis_speakers::AudioRuntime::Heuristic,
                segments: stable_speaker_predictions_from_diarization(result.segments)?,
            });
        }

        let mut response = audio_analysis_speakers::diarize_speaker_audio_baseline(
            &audio.samples,
            audio.sample_rate,
        )?;
        response.model_id = options.model_id.clone();
        Ok(response)
    }
}

#[cfg(feature = "diarization")]
fn diarize_with_onnx_speaker_embeddings(
    audio: LoadedAudio,
    transcript: &TranscriptionContract,
    options: &DiarizationOptions,
) -> Result<SpeakerDiarizationResponse> {
    let config = onnx_speaker_embedding_config(options)?;
    let speaker_audio =
        audio_analysis_speakers::SpeakerAudio::mono(&audio.samples, audio.sample_rate)?;
    let embedder = audio_analysis_speakers::OnnxSpeakerEmbedder::from_config(config)?;
    let spans = speech_spans_from_transcript(transcript, audio.duration_seconds())?;
    let result = if spans.is_empty() {
        let vad = audio_analysis_speakers::EnergyVoiceActivityDetector::default();
        let mut diarizer = audio_analysis_speakers::WindowedSpeakerDiarizer::new(embedder, vad)
            .cluster_threshold(0.95)?;
        audio_analysis_speakers::SpeakerDiarizer::diarize(&mut diarizer, &speaker_audio)?
    } else {
        let vad = TranscriptSpeechSpanVad { spans };
        let mut diarizer = audio_analysis_speakers::WindowedSpeakerDiarizer::new(embedder, vad)
            .cluster_threshold(0.95)?;
        audio_analysis_speakers::SpeakerDiarizer::diarize(&mut diarizer, &speaker_audio)?
    };
    Ok(SpeakerDiarizationResponse {
        accepted: true,
        operation: "audio.speakers.diarize".to_string(),
        model_id: options.model_id.clone(),
        runtime: audio_analysis_speakers::AudioRuntime::Onnx,
        segments: stable_speaker_predictions_from_diarization(result.segments)?,
    })
}

/// Default pure-Rust energy VAD provider.
#[derive(Debug, Clone, Default)]
pub struct EnergyVadTranscriptionProvider;

impl TranscriptionVadProvider for EnergyVadTranscriptionProvider {
    fn provider_id(&self) -> &str {
        "energy-vad"
    }

    fn detect_speech(&mut self, request: VadRequest) -> Result<VadResponse> {
        let segments = energy_vad_segments(&request.audio, &request.options)?;
        Ok(VadResponse {
            segments,
            diagnostics: vec!["deterministic energy VAD completed".to_string()],
        })
    }
}

/// Feature-gated Candle Whisper provider.
#[derive(Debug, Clone, Default)]
pub struct CandleWhisperTranscriber {
    pub options: CandleWhisperOptions,
}

impl CandleWhisperTranscriber {
    pub fn new(options: CandleWhisperOptions) -> Self {
        Self { options }
    }
}

impl AudioTranscriptionProvider for CandleWhisperTranscriber {
    fn provider_id(&self) -> &str {
        "candle-whisper"
    }

    fn transcribe(&mut self, request: AsrRequest) -> Result<AsrResponse> {
        validate_asr_request(&request)?;
        validate_candle_setup(&self.options)?;
        #[cfg(feature = "candle")]
        {
            let chunk_count = request.chunks.len();
            let mut response = native_whisper::transcribe(&self.options, request)?;
            response.diagnostics.extend(candle_batch_diagnostics(
                &self.options,
                chunk_count,
                "candle-whisper-sequential",
            ));
            Ok(response)
        }
        #[cfg(not(feature = "candle"))]
        {
            Err(unsupported_runtime(format!(
                "Candle Whisper requested for `{}` but the binary lacks the `candle` feature",
                request.model_id
            )))
        }
    }
}

/// Native whisper.cpp compatibility provider.
#[derive(Debug, Clone, Default)]
pub struct WhisperCppTranscriber {
    pub options: WhisperCppProviderOptions,
}

impl AudioTranscriptionProvider for WhisperCppTranscriber {
    fn provider_id(&self) -> &str {
        "whisper-cpp"
    }

    fn transcribe(&mut self, _request: AsrRequest) -> Result<AsrResponse> {
        let Some(model_path) = &self.options.model_path else {
            return Err(setup_error("required whisper.cpp model path is missing"));
        };
        if !model_path.exists() {
            return Err(setup_error(format!(
                "required whisper.cpp model `{}` is missing",
                model_path.display()
            )));
        }
        Err(unsupported_runtime(
            "whisper.cpp compatibility provider is not the primary transcription path",
        ))
    }
}

/// Feature-gated CTC forced aligner.
#[derive(Debug, Clone, Default)]
pub struct CtcForcedAligner {
    pub options: AlignmentOptions,
}

impl ForcedAlignmentProvider for CtcForcedAligner {
    fn provider_id(&self) -> &str {
        "ctc-forced-aligner"
    }

    fn align(&mut self, request: AlignmentRequest) -> Result<AlignmentResponse> {
        #[cfg(feature = "alignment")]
        {
            ctc_alignment::align(&self.options, request)
        }
        #[cfg(not(feature = "alignment"))]
        {
            validate_alignment_setup(&self.options)?;
            Err(unsupported_runtime(format!(
            "CTC alignment execution for `{}` is planned behind the alignment provider; default tests use mock alignment providers",
            request.model_id
        )))
        }
    }
}

/// External command provider for Python WhisperX.
#[derive(Debug, Clone, Default)]
pub struct WhisperXCommandTranscriber;

impl WhisperXCommandTranscriber {
    pub fn transcribe_pipeline(
        &mut self,
        request: TranscriptionPipelineRequest,
    ) -> Result<TranscriptionPipelineResponse> {
        match request.provider {
            TranscriptionProviderSelection::ExternalWhisperX(options) => {
                run_whisperx_command(request.source.path()?, options)
            }
            other => Err(invalid_request(format!(
                "whisperx-command cannot run provider `{}`",
                other.provider_id()
            ))),
        }
    }
}

fn run_native_transcription_pipeline(
    request: TranscriptionPipelineRequest,
    vad: &mut dyn TranscriptionVadProvider,
    asr: &mut dyn AudioTranscriptionProvider,
    diarization_provider: Option<&mut dyn TranscriptDiarizationProvider>,
) -> Result<TranscriptionPipelineResponse> {
    if !request.alignment.enabled {
        return run_transcription_pipeline(request, vad, asr, None, diarization_provider);
    }

    let mut aligner = CtcForcedAligner {
        options: request.alignment.clone(),
    };
    run_transcription_pipeline(
        request,
        vad,
        asr,
        Some(&mut aligner as &mut dyn ForcedAlignmentProvider),
        diarization_provider,
    )
}

/// Runs a transcription request with the selected primary provider.
pub fn transcribe(request: TranscriptionPipelineRequest) -> Result<TranscriptionPipelineResponse> {
    match &request.provider {
        TranscriptionProviderSelection::ExternalWhisperX(_) => {
            let mut provider = WhisperXCommandTranscriber;
            provider.transcribe_pipeline(request)
        }
        TranscriptionProviderSelection::CandleWhisper(options) => {
            let mut vad = EnergyVadTranscriptionProvider;
            let mut asr = CandleWhisperTranscriber::new(options.clone());
            #[cfg(feature = "diarization")]
            {
                let mut diarizer = NativeSpeakerDiarizationProvider;
                let diarization_provider = request
                    .diarization
                    .enabled
                    .then_some(&mut diarizer as &mut dyn TranscriptDiarizationProvider);
                run_native_transcription_pipeline(request, &mut vad, &mut asr, diarization_provider)
            }
            #[cfg(not(feature = "diarization"))]
            {
                run_native_transcription_pipeline(request, &mut vad, &mut asr, None)
            }
        }
        TranscriptionProviderSelection::WhisperCpp(options) => {
            let mut vad = EnergyVadTranscriptionProvider;
            let mut asr = WhisperCppTranscriber {
                options: options.clone(),
            };
            #[cfg(feature = "diarization")]
            {
                let mut diarizer = NativeSpeakerDiarizationProvider;
                let diarization_provider = request
                    .diarization
                    .enabled
                    .then_some(&mut diarizer as &mut dyn TranscriptDiarizationProvider);
                run_native_transcription_pipeline(request, &mut vad, &mut asr, diarization_provider)
            }
            #[cfg(not(feature = "diarization"))]
            {
                run_native_transcription_pipeline(request, &mut vad, &mut asr, None)
            }
        }
    }
}

/// Runs the provider-agnostic native transcription pipeline.
pub fn run_transcription_pipeline(
    request: TranscriptionPipelineRequest,
    vad_provider: &mut dyn TranscriptionVadProvider,
    asr_provider: &mut dyn AudioTranscriptionProvider,
    alignment_provider: Option<&mut dyn ForcedAlignmentProvider>,
    diarization_provider: Option<&mut dyn TranscriptDiarizationProvider>,
) -> Result<TranscriptionPipelineResponse> {
    validate_batch_options_for_provider(&request.provider)?;
    let provider = request.provider.provider_id().to_string();
    let model_id = request.provider.model_id().to_string();
    let audio = LoadedAudio::mono_16khz_from_source(&request.source)?;
    let vad_response = if request.vad.enabled {
        vad_provider.detect_speech(VadRequest {
            audio: audio.clone(),
            options: request.vad.clone(),
        })?
    } else {
        VadResponse {
            segments: vec![SpeechActivitySegment::new(
                0.0,
                audio.duration_seconds().max(1.0 / audio.sample_rate as f64),
                1.0,
            )?],
            diagnostics: vec!["VAD disabled; using full source as one ASR chunk".to_string()],
        }
    };

    let language = provider_language(&request.provider);
    let mut asr_response = asr_provider.transcribe(AsrRequest {
        audio: audio.clone(),
        chunks: vad_response.segments.clone(),
        language: language.clone(),
        model_id: model_id.clone(),
    })?;
    if !asr_response
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.starts_with("batchChunks="))
    {
        if let TranscriptionProviderSelection::CandleWhisper(options) = &request.provider {
            asr_response.diagnostics.extend(candle_batch_diagnostics(
                options,
                vad_response.segments.len(),
                "candle-whisper-sequential",
            ));
        }
    }
    let mut transcript = normalize_transcription_contract(asr_response.transcript)
        .map_err(|error| model_output_mismatch(error.to_string()))?;
    offset_chunk_local_segments(&mut transcript, &vad_response.segments)?;

    let mut diagnostics = vad_response.diagnostics;
    diagnostics.extend(asr_response.diagnostics);
    let mut alignment_summary = None;
    if request.alignment.enabled {
        let provider = alignment_provider.ok_or_else(|| {
            setup_error("alignment requested but no alignment provider is available")
        })?;
        let alignment_response = provider.align(AlignmentRequest {
            audio: audio.clone(),
            transcript: transcript.clone(),
            language: language.clone(),
            model_id: request.alignment.model_id.clone(),
        })?;
        apply_alignment_words(&mut transcript, &alignment_response.words)?;
        alignment_summary = Some(AlignmentSummary {
            provider: provider.provider_id().to_string(),
            model_id: alignment_response.model_id,
            word_count: alignment_response.words.len(),
        });
        diagnostics.extend(alignment_response.diagnostics);
    }

    let mut diarization = None;
    if request.diarization.enabled {
        validate_diarization_options(&request.diarization)?;
        let provider = diarization_provider.ok_or_else(|| {
            setup_error("diarization requested but no diarization provider is available")
        })?;
        let response = provider.diarize(audio, &transcript, &request.diarization)?;
        diagnostics.extend(diarization_diagnostics(
            provider.provider_id(),
            &response,
            &request.diarization,
        ));
        assign_speakers_from_diarization(
            &mut transcript,
            &response,
            request.diarization.assignment_policy,
        )?;
        diarization = Some(response);
    }

    transcript = normalize_transcription_contract(transcript)
        .map_err(|error| model_output_mismatch(error.to_string()))?;
    transcript
        .validate_strict()
        .map_err(|error| model_output_mismatch(error.to_string()))?;

    Ok(TranscriptionPipelineResponse {
        accepted: true,
        operation: "audio.transcription.transcribe".to_string(),
        provider,
        model_id,
        transcript,
        vad_segments: vad_response.segments,
        alignment: alignment_summary,
        diarization,
        artifacts: Vec::new(),
        diagnostics,
    })
}

/// Parses existing WhisperX JSON without running external tools.
pub fn import_whisperx_json(bytes: &[u8]) -> Result<TranscriptionContract> {
    text_transcripts::parse_whisperx_json(bytes)
        .map_err(|error| DetectError::InvalidArgument(error.to_string()))
}

/// Returns provider plans.
pub fn transcription_provider_plans() -> Vec<TranscriptionProviderPlan> {
    vec![
        candle_whisper_provider_plan(),
        whisper_cpp_provider_plan(),
        whisperx_provider_plan(),
    ]
}

/// Returns the primary Candle Whisper provider plan.
pub fn candle_whisper_provider_plan() -> TranscriptionProviderPlan {
    TranscriptionProviderPlan {
        provider_id: "candle-whisper".to_string(),
        external_runtime: false,
        wasm_supported: false,
        primary: true,
        setup: vec![
            "Provide an offline model bundle with config.json, generation_config.json, tokenizer.json, preprocessor_config.json, and model.safetensors.".to_string(),
            "Build with feature `candle`; add `cuda` for CUDA device execution.".to_string(),
        ],
        diagnostics: vec![
            "Candle Whisper is the primary planned Rust-native ASR provider.".to_string(),
            "Default tests do not download models or require CUDA.".to_string(),
        ],
    }
}

/// Returns the whisper.cpp compatibility provider plan.
pub fn whisper_cpp_provider_plan() -> TranscriptionProviderPlan {
    TranscriptionProviderPlan {
        provider_id: "whisper-cpp".to_string(),
        external_runtime: false,
        wasm_supported: false,
        primary: false,
        setup: vec!["Provide a local whisper.cpp model path explicitly.".to_string()],
        diagnostics: vec![
            "whisper.cpp is retained as a native compatibility provider, not the primary ASR path."
                .to_string(),
        ],
    }
}

/// Returns the external WhisperX provider plan.
pub fn whisperx_provider_plan() -> TranscriptionProviderPlan {
    TranscriptionProviderPlan {
        provider_id: "whisperx-command".to_string(),
        external_runtime: true,
        wasm_supported: false,
        primary: false,
        setup: vec![
            "Install whisperx in the active Python environment.".to_string(),
            "Ensure ffmpeg is available on PATH.".to_string(),
            "Set HF_TOKEN before diarization requests.".to_string(),
        ],
        diagnostics: vec![
            "WhisperX execution is opt-in and never required by default tests.".to_string(),
            "Transcript normalization and WhisperX JSON import are delegated to text-transcripts."
                .to_string(),
        ],
    }
}

fn provider_language(provider: &TranscriptionProviderSelection) -> Option<String> {
    match provider {
        TranscriptionProviderSelection::CandleWhisper(options) => options.language.clone(),
        TranscriptionProviderSelection::WhisperCpp(options) => options.language.clone(),
        TranscriptionProviderSelection::ExternalWhisperX(options) => options.language.clone(),
    }
}

fn validate_batch_options_for_provider(provider: &TranscriptionProviderSelection) -> Result<()> {
    if let TranscriptionProviderSelection::CandleWhisper(options) = provider {
        validate_candle_batch_options(options)?;
    }
    Ok(())
}

pub(crate) fn validate_candle_batch_options(options: &CandleWhisperOptions) -> Result<()> {
    if options.max_batch_size == Some(0) {
        return Err(invalid_request(
            "Candle Whisper max_batch_size must be greater than zero",
        ));
    }
    Ok(())
}

pub(crate) fn candle_batch_count(options: &CandleWhisperOptions, chunk_count: usize) -> usize {
    if chunk_count == 0 {
        return 0;
    }
    if !options.batch_chunks {
        return chunk_count;
    }
    match options.max_batch_size {
        Some(max_batch_size) => chunk_count.div_ceil(max_batch_size),
        None => 1,
    }
}

pub(crate) fn candle_batch_diagnostics(
    options: &CandleWhisperOptions,
    chunk_count: usize,
    execution: &str,
) -> Vec<String> {
    vec![
        format!("chunkCount={chunk_count}"),
        format!("batchChunks={}", options.batch_chunks),
        format!(
            "maxBatchSize={}",
            options
                .max_batch_size
                .map(|value| value.to_string())
                .unwrap_or_else(|| "unbounded".to_string())
        ),
        format!("batchCount={}", candle_batch_count(options, chunk_count)),
        format!("batchExecution={execution}"),
    ]
}

pub(crate) fn validate_asr_request(request: &AsrRequest) -> Result<()> {
    native_audio::validate_loaded_audio(&request.audio)?;
    if request.chunks.is_empty() {
        return Err(invalid_request(
            "ASR request must contain at least one speech chunk",
        ));
    }
    let duration = request.audio.duration_seconds();
    let tolerance = 1.0 / request.audio.sample_rate as f64;
    for chunk in &request.chunks {
        chunk.validate()?;
        if chunk.end_seconds > duration + tolerance {
            return Err(invalid_request(format!(
                "speech chunk end {:.6} exceeds audio duration {:.6}",
                chunk.end_seconds, duration
            )));
        }
    }
    Ok(())
}

fn validate_diarization_options(options: &DiarizationOptions) -> Result<()> {
    if options.min_speakers == Some(0) {
        return Err(invalid_request(
            "diarization min_speakers must be greater than zero",
        ));
    }
    if options.max_speakers == Some(0) {
        return Err(invalid_request(
            "diarization max_speakers must be greater than zero",
        ));
    }
    if let (Some(min), Some(max)) = (options.min_speakers, options.max_speakers) {
        if min > max {
            return Err(invalid_request(
                "diarization min_speakers must be less than or equal to max_speakers",
            ));
        }
    }
    if options.speaker_embedding_dimension == Some(0) {
        return Err(invalid_request(
            "diarization speaker_embedding_dimension must be greater than zero",
        ));
    }
    if options.speaker_embedding_sample_rate == Some(0) {
        return Err(invalid_request(
            "diarization speaker_embedding_sample_rate must be greater than zero",
        ));
    }
    Ok(())
}

#[cfg(feature = "diarization")]
fn onnx_speaker_embedding_config(
    options: &DiarizationOptions,
) -> Result<audio_analysis_speakers::OnnxSpeakerEmbeddingConfig> {
    let bundle_path = options
        .speaker_embedding_model_bundle
        .clone()
        .ok_or_else(|| setup_error("ONNX speaker embedding model bundle is required"))?;
    let embedding_dimension = options.speaker_embedding_dimension.ok_or_else(|| {
        invalid_request("ONNX speaker embedding dimension must be configured explicitly")
    })?;
    Ok(audio_analysis_speakers::OnnxSpeakerEmbeddingConfig {
        bundle_path,
        model_file: options.speaker_embedding_model_file.clone(),
        input_name: options.speaker_embedding_input_name.clone(),
        output_name: options.speaker_embedding_output_name.clone(),
        embedding_dimension,
        sample_rate: options.speaker_embedding_sample_rate.unwrap_or(16_000),
    })
}

#[cfg(feature = "diarization")]
fn speech_spans_from_transcript(
    transcript: &TranscriptionContract,
    audio_duration_seconds: f64,
) -> Result<Vec<audio_analysis_speakers::SpeechSpan>> {
    const AUDIO_DURATION_EPSILON: f64 = 1e-6;

    let has_timed_words = transcript.segments.iter().any(|segment| {
        segment.words.iter().any(|word| {
            !word.text.trim().is_empty()
                && word.start_seconds.is_some()
                && word.end_seconds.is_some()
        })
    });

    let mut spans = Vec::new();
    if has_timed_words {
        for word in transcript
            .segments
            .iter()
            .flat_map(|segment| &segment.words)
        {
            if word.text.trim().is_empty() {
                continue;
            }
            let Some((start, end)) = word.start_seconds.zip(word.end_seconds) else {
                continue;
            };
            spans.push(transcript_timing_span(
                start,
                end,
                audio_duration_seconds,
                AUDIO_DURATION_EPSILON,
            )?);
        }
    } else {
        for segment in &transcript.segments {
            if segment.text.trim().is_empty() {
                continue;
            }
            let Some((start, end)) = segment.start_seconds.zip(segment.end_seconds) else {
                continue;
            };
            spans.push(transcript_timing_span(
                start,
                end,
                audio_duration_seconds,
                AUDIO_DURATION_EPSILON,
            )?);
        }
    }

    merge_transcript_speech_spans(
        spans,
        audio_analysis_speakers::EnergyVadConfig::default().merge_gap_seconds,
    )
}

#[cfg(feature = "diarization")]
fn transcript_timing_span(
    start_seconds: f64,
    end_seconds: f64,
    audio_duration_seconds: f64,
    audio_duration_epsilon: f64,
) -> Result<audio_analysis_speakers::SpeechSpan> {
    if !start_seconds.is_finite() || !end_seconds.is_finite() || !audio_duration_seconds.is_finite()
    {
        return Err(invalid_request(
            "transcript diarization timing values must be finite",
        ));
    }
    if start_seconds < 0.0 || end_seconds <= start_seconds {
        return Err(invalid_request(
            "transcript diarization timing must be non-negative with positive duration",
        ));
    }
    if start_seconds > audio_duration_seconds + audio_duration_epsilon
        || end_seconds > audio_duration_seconds + audio_duration_epsilon
    {
        return Err(invalid_request(format!(
            "transcript diarization timing end {:.6} exceeds audio duration {:.6}",
            end_seconds, audio_duration_seconds
        )));
    }
    let end_seconds = if end_seconds > audio_duration_seconds {
        audio_duration_seconds
    } else {
        end_seconds
    };
    audio_analysis_speakers::SpeechSpan::new(start_seconds, end_seconds, 1.0)
}

#[cfg(feature = "diarization")]
fn merge_transcript_speech_spans(
    mut spans: Vec<audio_analysis_speakers::SpeechSpan>,
    merge_gap_seconds: f64,
) -> Result<Vec<audio_analysis_speakers::SpeechSpan>> {
    spans.sort_by(|left, right| left.start_seconds.total_cmp(&right.start_seconds));
    let mut merged: Vec<audio_analysis_speakers::SpeechSpan> = Vec::new();
    for span in spans {
        if let Some(last) = merged.last_mut() {
            if span.start_seconds - last.end_seconds <= merge_gap_seconds {
                let last_duration = last.duration_seconds();
                let span_duration = span.duration_seconds();
                let total_duration = last_duration + span_duration;
                last.end_seconds = last.end_seconds.max(span.end_seconds);
                last.score = if total_duration > f64::EPSILON {
                    (((last.score as f64 * last_duration) + (span.score as f64 * span_duration))
                        / total_duration) as f32
                } else {
                    last.score.max(span.score)
                };
                continue;
            }
        }
        merged.push(span);
    }
    Ok(merged)
}

#[cfg(feature = "diarization")]
#[derive(Debug, Clone)]
struct TranscriptSpeechSpanVad {
    spans: Vec<audio_analysis_speakers::SpeechSpan>,
}

#[cfg(feature = "diarization")]
impl audio_analysis_speakers::VoiceActivityDetector for TranscriptSpeechSpanVad {
    fn detect_speech(
        &mut self,
        _audio: &audio_analysis_speakers::SpeakerAudio<'_>,
    ) -> Result<Vec<audio_analysis_speakers::SpeechSpan>> {
        Ok(self.spans.clone())
    }
}

#[cfg(feature = "diarization")]
fn stable_speaker_predictions_from_diarization(
    segments: Vec<audio_analysis_speakers::DiarizationSegment>,
) -> Result<Vec<SpeakerSegmentPrediction>> {
    let mut unknown_labels: Vec<(String, String)> = Vec::new();
    let mut predictions = Vec::new();
    for segment in segments {
        let speaker = match segment.speaker {
            audio_analysis_speakers::DiarizedSpeaker::Known(id) => id.as_str().to_string(),
            audio_analysis_speakers::DiarizedSpeaker::Unknown(label) => {
                if let Some((_, stable)) = unknown_labels
                    .iter()
                    .find(|(existing, _)| existing == &label)
                {
                    stable.clone()
                } else {
                    let stable = format!("speaker_{}", unknown_labels.len());
                    unknown_labels.push((label, stable.clone()));
                    stable
                }
            }
        };
        predictions.push(normalize_speaker_prediction(SpeakerSegmentPrediction {
            speaker,
            start_seconds: segment.start_seconds as f32,
            end_seconds: segment.end_seconds as f32,
            score: Some(segment.score),
        })?);
    }
    merge_speaker_predictions(
        predictions,
        audio_analysis_speakers::EnergyVadConfig::default().merge_gap_seconds as f32,
    )
}

#[cfg(feature = "diarization")]
fn normalize_speaker_prediction(
    mut segment: SpeakerSegmentPrediction,
) -> Result<SpeakerSegmentPrediction> {
    segment.speaker = segment.speaker.trim().to_string();
    if segment.speaker.is_empty() {
        return Err(invalid_request("speaker label must not be empty"));
    }
    if !segment.start_seconds.is_finite() || !segment.end_seconds.is_finite() {
        return Err(invalid_request("speaker segment timestamps must be finite"));
    }
    if segment.end_seconds < segment.start_seconds {
        return Err(invalid_request(
            "speaker segment end_seconds must be greater than or equal to start_seconds",
        ));
    }
    segment.score = segment
        .score
        .and_then(|score| score.is_finite().then(|| score.clamp(0.0, 1.0)));
    Ok(segment)
}

#[cfg(feature = "diarization")]
fn merge_speaker_predictions(
    segments: Vec<SpeakerSegmentPrediction>,
    merge_gap_seconds: f32,
) -> Result<Vec<SpeakerSegmentPrediction>> {
    let mut merged: Vec<SpeakerSegmentPrediction> = Vec::new();
    for segment in segments {
        if let Some(last) = merged.last_mut() {
            if last.speaker == segment.speaker
                && segment.start_seconds - last.end_seconds <= merge_gap_seconds
            {
                let last_duration = (last.end_seconds - last.start_seconds).max(0.0);
                let segment_duration = (segment.end_seconds - segment.start_seconds).max(0.0);
                let total_duration = last_duration + segment_duration;
                last.end_seconds = segment.end_seconds;
                last.score = match (last.score, segment.score) {
                    (Some(left), Some(right)) if total_duration > f32::EPSILON => {
                        Some(((left * last_duration) + (right * segment_duration)) / total_duration)
                    }
                    (Some(left), Some(right)) => Some(left.max(right)),
                    (Some(left), None) => Some(left),
                    (None, Some(right)) => Some(right),
                    (None, None) => None,
                };
                continue;
            }
        }
        merged.push(segment);
    }
    Ok(merged)
}

fn diarization_speaker_count(response: &SpeakerDiarizationResponse) -> usize {
    response
        .segments
        .iter()
        .map(|segment| segment.speaker.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .len()
}

fn diarization_diagnostics(
    provider_id: &str,
    response: &SpeakerDiarizationResponse,
    options: &DiarizationOptions,
) -> Vec<String> {
    let speaker_count = diarization_speaker_count(response);
    let mut diagnostics = vec![
        format!("diarizationProvider={provider_id}"),
        format!("diarizationRuntime={}", diarization_runtime_value(response)),
        format!("diarizationModelId={}", response.model_id),
        format!("diarizationSegmentCount={}", response.segments.len()),
        format!("diarizationSpeakerCount={speaker_count}"),
        format!(
            "diarizationAssignmentPolicy={}",
            speaker_assignment_policy_value(options.assignment_policy)
        ),
    ];
    if let Some(min) = options.min_speakers {
        diagnostics.push(format!("diarizationMinSpeakers={min}"));
        if speaker_count < min {
            diagnostics.push(format!(
                "diarizationSpeakerCountBelowRequestedMin={speaker_count}/{min}"
            ));
        }
    }
    if let Some(max) = options.max_speakers {
        diagnostics.push(format!("diarizationMaxSpeakers={max}"));
        if speaker_count > max {
            diagnostics.push(format!(
                "diarizationSpeakerCountAboveRequestedMax={speaker_count}/{max}"
            ));
        }
    }
    if diarization_runtime_is_heuristic(response) {
        diagnostics.push("diarizationBaseline=heuristic-native".to_string());
    } else if diarization_runtime_value(response) == "onnx" {
        diagnostics.push("speakerEmbeddingProvider=onnx".to_string());
        if let Some(dimension) = options.speaker_embedding_dimension {
            diagnostics.push(format!("speakerEmbeddingDimension={dimension}"));
        }
        diagnostics.push("diarizationBaseline=false".to_string());
    }
    diagnostics
}

#[cfg(feature = "diarization")]
fn diarization_runtime_value(response: &SpeakerDiarizationResponse) -> &'static str {
    match response.runtime {
        audio_analysis_speakers::AudioRuntime::Onnx => "onnx",
        audio_analysis_speakers::AudioRuntime::Candle => "candle",
        audio_analysis_speakers::AudioRuntime::WhisperCpp => "whisper_cpp",
        audio_analysis_speakers::AudioRuntime::Demucs => "demucs",
        audio_analysis_speakers::AudioRuntime::External => "external",
        audio_analysis_speakers::AudioRuntime::Spectral => "spectral",
        audio_analysis_speakers::AudioRuntime::Heuristic => "heuristic",
        audio_analysis_speakers::AudioRuntime::Imported => "imported",
    }
}

#[cfg(not(feature = "diarization"))]
fn diarization_runtime_value(response: &SpeakerDiarizationResponse) -> &str {
    response.runtime.as_str()
}

#[cfg(feature = "diarization")]
fn diarization_runtime_is_heuristic(response: &SpeakerDiarizationResponse) -> bool {
    response.runtime == audio_analysis_speakers::AudioRuntime::Heuristic
}

#[cfg(not(feature = "diarization"))]
fn diarization_runtime_is_heuristic(response: &SpeakerDiarizationResponse) -> bool {
    response.runtime == "heuristic"
}

fn speaker_assignment_policy_value(policy: SpeakerAssignmentPolicy) -> &'static str {
    match policy {
        SpeakerAssignmentPolicy::Majority => "majority",
        SpeakerAssignmentPolicy::NearestStart => "nearestStart",
        SpeakerAssignmentPolicy::StrictContained => "strictContained",
    }
}

pub(crate) fn normalize_samples_source(
    samples: &[f32],
    sample_rate: u32,
    channels: u16,
    source: Option<String>,
) -> Result<LoadedAudio> {
    if sample_rate == 0 || channels == 0 {
        return Err(DetectError::InvalidAudioFormat {
            sample_rate,
            channels,
        });
    }
    if samples.is_empty() {
        return Err(invalid_request("empty audio"));
    }
    if !samples.len().is_multiple_of(channels as usize) {
        return Err(invalid_request(
            "sample count must contain complete interleaved frames",
        ));
    }
    if samples.iter().any(|sample| !sample.is_finite()) {
        return Err(invalid_request("audio samples must be finite"));
    }
    let mono = if channels == 1 {
        samples.to_vec()
    } else {
        samples
            .chunks_exact(channels as usize)
            .map(|frame| frame.iter().sum::<f32>() / channels as f32)
            .collect::<Vec<_>>()
    };
    let samples = if sample_rate == 16_000 {
        mono
    } else {
        resample_linear(&mono, sample_rate, 16_000)
    };
    Ok(LoadedAudio {
        samples,
        sample_rate: 16_000,
        channels: 1,
        source,
    })
}

pub(crate) fn resample_linear(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if samples.is_empty() || from_rate == to_rate {
        return samples.to_vec();
    }
    let output_len = ((samples.len() as f64 * to_rate as f64) / from_rate as f64)
        .round()
        .max(1.0) as usize;
    (0..output_len)
        .map(|index| {
            let position = index as f64 * from_rate as f64 / to_rate as f64;
            let left = position.floor() as usize;
            let right = (left + 1).min(samples.len() - 1);
            let frac = (position - left as f64) as f32;
            samples[left] * (1.0 - frac) + samples[right] * frac
        })
        .collect()
}

fn energy_vad_segments(
    audio: &LoadedAudio,
    options: &VadOptions,
) -> Result<Vec<SpeechActivitySegment>> {
    validate_vad_options(options)?;
    let frame_size = seconds_to_samples(options.frame_seconds, audio.sample_rate)?;
    let hop_size = seconds_to_samples(options.hop_seconds, audio.sample_rate)?;
    let mut active = Vec::new();
    let mut start = 0;
    while start < audio.samples.len() {
        let end = (start + frame_size).min(audio.samples.len());
        let score = rms(&audio.samples[start..end]);
        if score >= options.rms_threshold {
            active.push(SpeechActivitySegment::new(
                start as f64 / audio.sample_rate as f64,
                end as f64 / audio.sample_rate as f64,
                score,
            )?);
        }
        if start + hop_size >= audio.samples.len() {
            break;
        }
        start += hop_size;
    }
    let duration = audio.duration_seconds();
    let mut merged = merge_speech_segments(active, options.merge_gap_seconds)?;
    for segment in &mut merged {
        segment.start_seconds = (segment.start_seconds - options.padding_seconds).max(0.0);
        segment.end_seconds = (segment.end_seconds + options.padding_seconds).min(duration);
    }
    merged = merge_speech_segments(merged, options.merge_gap_seconds)?;
    let filtered = merged
        .into_iter()
        .filter(|segment| segment.end_seconds - segment.start_seconds >= options.min_speech_seconds)
        .flat_map(|segment| split_max_chunk(segment, options.max_chunk_seconds))
        .collect::<Vec<_>>();
    if filtered.is_empty() {
        return Ok(vec![SpeechActivitySegment::new(
            0.0,
            duration.max(1.0 / audio.sample_rate as f64),
            0.0,
        )?]);
    }
    Ok(filtered)
}

fn validate_vad_options(options: &VadOptions) -> Result<()> {
    if !options.rms_threshold.is_finite() || options.rms_threshold < 0.0 {
        return Err(invalid_request(
            "VAD RMS threshold must be finite and non-negative",
        ));
    }
    for (name, value, positive) in [
        ("frameSeconds", options.frame_seconds, true),
        ("hopSeconds", options.hop_seconds, true),
        ("minSpeechSeconds", options.min_speech_seconds, false),
        ("paddingSeconds", options.padding_seconds, false),
        ("mergeGapSeconds", options.merge_gap_seconds, false),
        ("maxChunkSeconds", options.max_chunk_seconds, true),
    ] {
        if !value.is_finite() || (positive && value <= 0.0) || (!positive && value < 0.0) {
            return Err(invalid_request(format!(
                "VAD option `{name}` has an invalid value"
            )));
        }
    }
    Ok(())
}

fn seconds_to_samples(seconds: f64, sample_rate: u32) -> Result<usize> {
    if !seconds.is_finite() || seconds <= 0.0 || sample_rate == 0 {
        return Err(invalid_request("invalid sample duration"));
    }
    Ok((seconds * sample_rate as f64).round().max(1.0) as usize)
}

fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    (samples.iter().map(|sample| sample * sample).sum::<f32>() / samples.len() as f32).sqrt()
}

fn merge_speech_segments(
    mut segments: Vec<SpeechActivitySegment>,
    merge_gap_seconds: f64,
) -> Result<Vec<SpeechActivitySegment>> {
    segments.sort_by(|left, right| left.start_seconds.total_cmp(&right.start_seconds));
    let mut merged: Vec<SpeechActivitySegment> = Vec::new();
    for segment in segments {
        if let Some(last) = merged.last_mut() {
            if segment.start_seconds - last.end_seconds <= merge_gap_seconds {
                last.end_seconds = last.end_seconds.max(segment.end_seconds);
                last.score = last.score.max(segment.score);
                continue;
            }
        }
        merged.push(segment);
    }
    for segment in &merged {
        segment.validate()?;
    }
    Ok(merged)
}

fn split_max_chunk(
    segment: SpeechActivitySegment,
    max_chunk_seconds: f64,
) -> Vec<SpeechActivitySegment> {
    if segment.end_seconds - segment.start_seconds <= max_chunk_seconds {
        return vec![segment];
    }
    let mut chunks = Vec::new();
    let mut start = segment.start_seconds;
    while start < segment.end_seconds {
        let end = (start + max_chunk_seconds).min(segment.end_seconds);
        if end > start {
            chunks.push(SpeechActivitySegment {
                start_seconds: start,
                end_seconds: end,
                score: segment.score,
            });
        }
        start = end;
    }
    chunks
}

fn offset_chunk_local_segments(
    transcript: &mut TranscriptionContract,
    chunks: &[SpeechActivitySegment],
) -> Result<()> {
    if chunks.is_empty() || transcript.segments.len() != chunks.len() {
        return Ok(());
    }
    for (segment, chunk) in transcript.segments.iter_mut().zip(chunks) {
        if segment
            .attributes
            .get("timing")
            .is_some_and(|value| value == "global")
        {
            continue;
        }
        if let Some(start) = &mut segment.start_seconds {
            *start += chunk.start_seconds;
        }
        if let Some(end) = &mut segment.end_seconds {
            *end += chunk.start_seconds;
        }
        for word in &mut segment.words {
            if let Some(start) = &mut word.start_seconds {
                *start += chunk.start_seconds;
            }
            if let Some(end) = &mut word.end_seconds {
                *end += chunk.start_seconds;
            }
        }
        segment
            .attributes
            .insert("timing".to_string(), "global".to_string());
    }
    Ok(())
}

fn apply_alignment_words(
    transcript: &mut TranscriptionContract,
    words: &[AlignedWord],
) -> Result<()> {
    for aligned in words {
        if !aligned.start_seconds.is_finite()
            || !aligned.end_seconds.is_finite()
            || aligned.end_seconds < aligned.start_seconds
        {
            return Err(model_output_mismatch(
                "alignment output contains invalid word timing",
            ));
        }
        let segment = transcript
            .segments
            .iter_mut()
            .find(|segment| segment.index == aligned.segment_index)
            .ok_or_else(|| model_output_mismatch("alignment output references unknown segment"))?;
        while segment.words.len() <= aligned.word_index {
            segment.words.push(TranscriptWordContract {
                text: String::new(),
                start_seconds: None,
                end_seconds: None,
                confidence: None,
                speaker: None,
                attributes: BTreeMap::new(),
            });
        }
        let word = &mut segment.words[aligned.word_index];
        if word.text.trim().is_empty() {
            word.text = aligned.text.clone();
        }
        word.start_seconds = Some(aligned.start_seconds);
        word.end_seconds = Some(aligned.end_seconds);
        word.confidence = aligned.confidence;
    }
    Ok(())
}

fn assign_speakers_from_diarization(
    transcript: &mut TranscriptionContract,
    diarization: &SpeakerDiarizationResponse,
    policy: SpeakerAssignmentPolicy,
) -> Result<()> {
    for segment in &mut transcript.segments {
        for word in &mut segment.words {
            let Some((start, end)) = word.start_seconds.zip(word.end_seconds) else {
                continue;
            };
            word.speaker = speaker_for_range(start, end, diarization, policy);
        }
        segment.speaker = if segment.speaker.is_some() {
            segment.speaker.clone()
        } else if !segment.words.is_empty() {
            majority_word_speaker(&segment.words)
        } else if let Some((start, end)) = segment.start_seconds.zip(segment.end_seconds) {
            speaker_for_range(start, end, diarization, policy)
        } else {
            None
        };
    }
    Ok(())
}

fn majority_word_speaker(words: &[TranscriptWordContract]) -> Option<String> {
    let mut counts = BTreeMap::<String, usize>::new();
    for speaker in words.iter().filter_map(|word| word.speaker.as_ref()) {
        *counts.entry(speaker.clone()).or_default() += 1;
    }
    counts
        .into_iter()
        .max_by(|left, right| left.1.cmp(&right.1).then_with(|| right.0.cmp(&left.0)))
        .map(|(speaker, _)| speaker)
}

fn speaker_for_range(
    start: f64,
    end: f64,
    diarization: &SpeakerDiarizationResponse,
    policy: SpeakerAssignmentPolicy,
) -> Option<String> {
    #[cfg(feature = "diarization")]
    let segments = diarization
        .segments
        .iter()
        .map(|segment| {
            (
                segment.speaker.clone(),
                segment.start_seconds as f64,
                segment.end_seconds as f64,
            )
        })
        .collect::<Vec<_>>();
    #[cfg(not(feature = "diarization"))]
    let segments = diarization
        .segments
        .iter()
        .map(|segment| {
            (
                segment.speaker.clone(),
                segment.start_seconds as f64,
                segment.end_seconds as f64,
            )
        })
        .collect::<Vec<_>>();

    match policy {
        SpeakerAssignmentPolicy::Majority => segments
            .iter()
            .filter_map(|(speaker, diar_start, diar_end)| {
                let overlap = (end.min(*diar_end) - start.max(*diar_start)).max(0.0);
                (overlap > 0.0).then_some((speaker.clone(), overlap))
            })
            .max_by(|left, right| {
                left.1
                    .total_cmp(&right.1)
                    .then_with(|| right.0.cmp(&left.0))
            })
            .map(|(speaker, _)| speaker),
        SpeakerAssignmentPolicy::NearestStart => segments
            .iter()
            .map(|(speaker, diar_start, _)| (speaker.clone(), (*diar_start - start).abs()))
            .min_by(|left, right| {
                left.1
                    .total_cmp(&right.1)
                    .then_with(|| left.0.cmp(&right.0))
            })
            .map(|(speaker, _)| speaker),
        SpeakerAssignmentPolicy::StrictContained => {
            segments.iter().find_map(|(speaker, diar_start, diar_end)| {
                (*diar_start <= start && *diar_end >= end).then_some(speaker.clone())
            })
        }
    }
}

fn validate_candle_setup(options: &CandleWhisperOptions) -> Result<()> {
    validate_candle_batch_options(options)?;
    native_device::resolve_native_device(options.device)?;
    if !cfg!(feature = "candle") {
        return Err(unsupported_runtime(
            "Candle Whisper requested but the binary lacks the `candle` feature",
        ));
    }
    if let Some(bundle) = &options.model_bundle {
        validate_model_bundle_files(
            bundle,
            &[
                "config.json",
                "generation_config.json",
                "tokenizer.json",
                "preprocessor_config.json",
                "model.safetensors",
            ],
        )
    } else {
        Err(setup_error(
            "required Candle Whisper model bundle is missing",
        ))
    }
}

#[cfg(not(feature = "alignment"))]
fn validate_alignment_setup(options: &AlignmentOptions) -> Result<()> {
    if !cfg!(feature = "alignment") {
        return Err(unsupported_runtime(
            "CTC alignment requested but the binary lacks the `alignment` feature",
        ));
    }
    if let Some(bundle) = &options.model_bundle {
        validate_model_bundle_files(
            bundle,
            &[
                "config.json",
                "tokenizer.json",
                "preprocessor_config.json",
                "model.safetensors",
            ],
        )
    } else {
        Err(setup_error(
            "required CTC alignment model bundle is missing",
        ))
    }
}

fn validate_model_bundle_files(bundle: &Path, files: &[&str]) -> Result<()> {
    native_bundles::validate_required_bundle_files(bundle, files)
}

fn run_whisperx_command(
    source_path: &Path,
    options: WhisperXCommandOptions,
) -> Result<TranscriptionPipelineResponse> {
    let output_dir = options
        .output_dir
        .clone()
        .unwrap_or_else(default_whisperx_output_dir);
    fs::create_dir_all(&output_dir)?;

    let hf_token = if options.diarize {
        let env_name = options
            .hf_token_env
            .clone()
            .unwrap_or_else(|| "HF_TOKEN".to_string());
        Some(std::env::var(&env_name).map_err(|_| {
            setup_error(format!(
                "diarization requires `{env_name}` to be set before running WhisperX"
            ))
        })?)
    } else {
        None
    };

    let args = whisperx_args(source_path, &output_dir, &options, hf_token.as_deref());
    let child = Command::new(&options.command)
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                setup_error(format!(
                    "WhisperX command `{}` was not found; install whisperx or pass provider.command",
                    options.command.display()
                ))
            } else {
                DetectError::Io(error)
            }
        })?;
    let output = wait_with_optional_timeout(child, &options.command, options.timeout_seconds)?;
    if !output.status.success() {
        return Err(setup_error(format!(
            "WhisperX command `{}` failed: {}",
            options.command.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }

    let (transcript_path, transcript_bytes) = whisperx_json_bytes(&output_dir, &output.stdout)
        .ok_or_else(|| {
            model_output_mismatch(format!(
                "WhisperX completed but no JSON transcript was found in `{}`",
                output_dir.display()
            ))
        })?;
    let mut transcript = import_whisperx_json(&transcript_bytes)?;
    if transcript.source.is_none() {
        transcript.source = Some(source_path.to_string_lossy().into_owned());
    }
    let artifacts = transcript_path
        .map(|path| TranscriptionArtifact {
            kind: "whisperx-json".to_string(),
            path,
        })
        .into_iter()
        .collect();
    Ok(TranscriptionPipelineResponse {
        accepted: true,
        operation: "audio.transcription.transcribe".to_string(),
        provider: "whisperx-command".to_string(),
        model_id: options.model,
        transcript,
        vad_segments: Vec::new(),
        alignment: None,
        diarization: None,
        artifacts,
        diagnostics: vec![
            format!("ran WhisperX output in `{}`", output_dir.display()),
            "parsed WhisperX JSON through text-transcripts".to_string(),
        ],
    })
}

fn whisperx_args(
    source_path: &Path,
    output_dir: &Path,
    options: &WhisperXCommandOptions,
    hf_token: Option<&str>,
) -> Vec<String> {
    let mut args = vec![
        source_path.to_string_lossy().into_owned(),
        "--model".to_string(),
        options.model.clone(),
        "--device".to_string(),
        options.device.as_str().to_string(),
        "--output_format".to_string(),
        "json".to_string(),
        "--output_dir".to_string(),
        output_dir.to_string_lossy().into_owned(),
    ];
    if let Some(language) = &options.language {
        args.extend(["--language".to_string(), language.clone()]);
    }
    if let Some(compute_type) = &options.compute_type {
        args.extend(["--compute_type".to_string(), compute_type.clone()]);
    }
    if let Some(batch_size) = options.batch_size {
        args.extend(["--batch_size".to_string(), batch_size.to_string()]);
    }
    if let Some(align_model) = &options.align_model {
        args.extend(["--align_model".to_string(), align_model.clone()]);
    }
    if options.diarize {
        args.push("--diarize".to_string());
        if let Some(min_speakers) = options.min_speakers {
            args.extend(["--min_speakers".to_string(), min_speakers.to_string()]);
        }
        if let Some(max_speakers) = options.max_speakers {
            args.extend(["--max_speakers".to_string(), max_speakers.to_string()]);
        }
        if let Some(hf_token) = hf_token {
            args.extend(["--hf_token".to_string(), hf_token.to_string()]);
        }
    }
    args.extend(options.extra_args.clone());
    args
}

fn whisperx_json_bytes(output_dir: &Path, stdout: &[u8]) -> Option<(Option<PathBuf>, Vec<u8>)> {
    let path = find_json_artifact(output_dir);
    if let Some(path) = path {
        return fs::read(&path).ok().map(|bytes| (Some(path), bytes));
    }
    serde_json::from_slice::<serde_json::Value>(stdout)
        .ok()
        .map(|_| (None, stdout.to_vec()))
}

fn find_json_artifact(output_dir: &Path) -> Option<PathBuf> {
    let mut candidates = fs::read_dir(output_dir)
        .ok()?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("json"))
        .collect::<Vec<_>>();
    candidates.sort();
    candidates.into_iter().next()
}

fn default_whisperx_output_dir() -> PathBuf {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    std::env::temp_dir().join(format!("video-analysis-whisperx-{millis}"))
}

fn wait_with_optional_timeout(
    mut child: Child,
    command: &Path,
    timeout_seconds: Option<u64>,
) -> Result<Output> {
    let Some(seconds) = timeout_seconds else {
        return child.wait_with_output().map_err(DetectError::Io);
    };
    let started = Instant::now();
    loop {
        if child.try_wait()?.is_some() {
            return child.wait_with_output().map_err(DetectError::Io);
        }
        if started.elapsed() >= Duration::from_secs(seconds) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(timeout_error(format!(
                "WhisperX command `{}` timed out after {seconds} seconds",
                command.display()
            )));
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}

pub(crate) fn setup_error(message: impl Into<String>) -> DetectError {
    DetectError::InvalidArgument(format!("setup_error: {}", message.into()))
}

pub(crate) fn invalid_request(message: impl Into<String>) -> DetectError {
    DetectError::InvalidArgument(format!("invalid_request: {}", message.into()))
}

pub(crate) fn model_output_mismatch(message: impl Into<String>) -> DetectError {
    DetectError::InvalidArgument(format!("model_output_mismatch: {}", message.into()))
}

fn timeout_error(message: impl Into<String>) -> DetectError {
    DetectError::InvalidArgument(format!("timeout: {}", message.into()))
}

pub(crate) fn unsupported_runtime(message: impl Into<String>) -> DetectError {
    DetectError::InvalidArgument(format!("unsupported_runtime: {}", message.into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use text_transcripts::TranscriptSegmentContract;

    #[derive(Default)]
    struct MockAsrProvider;

    impl AudioTranscriptionProvider for MockAsrProvider {
        fn provider_id(&self) -> &str {
            "mock-asr"
        }

        fn transcribe(&mut self, request: AsrRequest) -> Result<AsrResponse> {
            let segments = request
                .chunks
                .iter()
                .enumerate()
                .map(|(index, chunk)| {
                    let mut segment = TranscriptSegmentContract::new(index as u64, " hello ");
                    segment.start_seconds = Some(0.0);
                    segment.end_seconds = Some(chunk.end_seconds - chunk.start_seconds);
                    segment
                })
                .collect::<Vec<_>>();
            Ok(AsrResponse {
                model_id: request.model_id,
                language: request.language,
                transcript: TranscriptionContract::from_segments(
                    request.audio.source,
                    Some("en".to_string()),
                    segments,
                )
                .map_err(|error| DetectError::InvalidArgument(error.to_string()))?,
                diagnostics: vec!["mock ASR completed".to_string()],
            })
        }
    }

    #[derive(Clone)]
    struct FixedVadProvider {
        segments: Vec<SpeechActivitySegment>,
    }

    impl TranscriptionVadProvider for FixedVadProvider {
        fn provider_id(&self) -> &str {
            "fixed-vad"
        }

        fn detect_speech(&mut self, _request: VadRequest) -> Result<VadResponse> {
            Ok(VadResponse {
                segments: self.segments.clone(),
                diagnostics: vec!["fixed VAD completed".to_string()],
            })
        }
    }

    #[derive(Default)]
    struct MockAlignmentProvider;

    impl ForcedAlignmentProvider for MockAlignmentProvider {
        fn provider_id(&self) -> &str {
            "mock-aligner"
        }

        fn align(&mut self, request: AlignmentRequest) -> Result<AlignmentResponse> {
            Ok(AlignmentResponse {
                model_id: request.model_id,
                words: vec![AlignedWord {
                    segment_index: 0,
                    word_index: 0,
                    text: "hello".to_string(),
                    start_seconds: 0.05,
                    end_seconds: 0.35,
                    confidence: Some(0.91),
                }],
                diagnostics: vec!["mock alignment completed".to_string()],
            })
        }
    }

    struct MockDiarizationProvider;

    impl TranscriptDiarizationProvider for MockDiarizationProvider {
        fn provider_id(&self) -> &str {
            "mock-diarization"
        }

        fn diarize(
            &mut self,
            _audio: LoadedAudio,
            _transcript: &TranscriptionContract,
            options: &DiarizationOptions,
        ) -> Result<SpeakerDiarizationResponse> {
            #[cfg(not(feature = "diarization"))]
            {
                Ok(SpeakerDiarizationResponse {
                    accepted: true,
                    operation: "audio.speakers.diarize".to_string(),
                    model_id: options.model_id.clone(),
                    runtime: "mock".to_string(),
                    segments: vec![SpeakerSegmentPrediction {
                        speaker: "SPEAKER_00".to_string(),
                        start_seconds: 0.0,
                        end_seconds: 1.0,
                        score: Some(0.9),
                    }],
                })
            }
            #[cfg(feature = "diarization")]
            {
                Ok(SpeakerDiarizationResponse {
                    accepted: true,
                    operation: "audio.speakers.diarize".to_string(),
                    model_id: options.model_id.clone(),
                    runtime: audio_analysis_speakers::AudioRuntime::Imported,
                    segments: vec![audio_analysis_speakers::SpeakerSegmentPrediction {
                        speaker: "SPEAKER_00".to_string(),
                        start_seconds: 0.0,
                        end_seconds: 1.0,
                        score: Some(0.9),
                    }],
                })
            }
        }
    }

    struct PanickingDiarizationProvider {
        called: bool,
    }

    #[cfg(feature = "diarization")]
    struct MockOnnxDiarizationProvider;

    #[cfg(feature = "diarization")]
    impl TranscriptDiarizationProvider for MockOnnxDiarizationProvider {
        fn provider_id(&self) -> &str {
            "mock-onnx-diarization"
        }

        fn diarize(
            &mut self,
            _audio: LoadedAudio,
            _transcript: &TranscriptionContract,
            options: &DiarizationOptions,
        ) -> Result<SpeakerDiarizationResponse> {
            Ok(SpeakerDiarizationResponse {
                accepted: true,
                operation: "audio.speakers.diarize".to_string(),
                model_id: options.model_id.clone(),
                runtime: audio_analysis_speakers::AudioRuntime::Onnx,
                segments: vec![audio_analysis_speakers::SpeakerSegmentPrediction {
                    speaker: "SPEAKER_ONNX".to_string(),
                    start_seconds: 0.0,
                    end_seconds: 1.0,
                    score: Some(0.9),
                }],
            })
        }
    }

    impl TranscriptDiarizationProvider for PanickingDiarizationProvider {
        fn provider_id(&self) -> &str {
            "panicking-diarization"
        }

        fn diarize(
            &mut self,
            _audio: LoadedAudio,
            _transcript: &TranscriptionContract,
            _options: &DiarizationOptions,
        ) -> Result<SpeakerDiarizationResponse> {
            self.called = true;
            panic!("diarization provider should not be called for invalid options");
        }
    }

    fn sample_request() -> TranscriptionPipelineRequest {
        let mut samples = vec![0.0; 16_000];
        for sample in &mut samples[1_000..5_000] {
            *sample = 0.1;
        }
        TranscriptionPipelineRequest {
            source: TranscriptionSource::Samples {
                samples,
                sample_rate: 16_000,
                channels: 1,
                source: Some("synthetic".to_string()),
            },
            provider: TranscriptionProviderSelection::CandleWhisper(CandleWhisperOptions::default()),
            vad: VadOptions {
                min_speech_seconds: 0.01,
                ..VadOptions::default()
            },
            alignment: AlignmentOptions::default(),
            diarization: DiarizationOptions::default(),
            output: TranscriptionOutputOptions::default(),
        }
    }

    fn batch_test_chunks() -> Vec<SpeechActivitySegment> {
        vec![
            SpeechActivitySegment::new(0.0, 0.20, 1.0).unwrap(),
            SpeechActivitySegment::new(0.20, 0.40, 1.0).unwrap(),
            SpeechActivitySegment::new(0.40, 0.60, 1.0).unwrap(),
        ]
    }

    fn batch_test_request(options: CandleWhisperOptions) -> TranscriptionPipelineRequest {
        TranscriptionPipelineRequest {
            provider: TranscriptionProviderSelection::CandleWhisper(options),
            ..sample_request()
        }
    }

    fn diarization_response_for_tests(
        segments: Vec<SpeakerSegmentPrediction>,
    ) -> SpeakerDiarizationResponse {
        #[cfg(not(feature = "diarization"))]
        {
            SpeakerDiarizationResponse {
                accepted: true,
                operation: "audio.speakers.diarize".to_string(),
                model_id: "test-speakers".to_string(),
                runtime: "mock".to_string(),
                segments,
            }
        }
        #[cfg(feature = "diarization")]
        {
            SpeakerDiarizationResponse {
                accepted: true,
                operation: "audio.speakers.diarize".to_string(),
                model_id: "test-speakers".to_string(),
                runtime: audio_analysis_speakers::AudioRuntime::Imported,
                segments,
            }
        }
    }

    fn transcript_with_words(
        words: Vec<(&str, f64, f64)>,
    ) -> std::result::Result<TranscriptionContract, Box<dyn std::error::Error>> {
        let mut segment = TranscriptSegmentContract::new(
            0,
            words
                .iter()
                .map(|(word, _, _)| *word)
                .collect::<Vec<_>>()
                .join(" "),
        );
        segment.start_seconds = Some(0.0);
        segment.end_seconds = Some(2.0);
        segment.words = words
            .into_iter()
            .map(
                |(text, start_seconds, end_seconds)| TranscriptWordContract {
                    text: text.to_string(),
                    start_seconds: Some(start_seconds),
                    end_seconds: Some(end_seconds),
                    confidence: None,
                    speaker: None,
                    attributes: BTreeMap::new(),
                },
            )
            .collect();
        Ok(TranscriptionContract::from_segments(
            None,
            Some("en".to_string()),
            vec![segment],
        )?)
    }

    #[cfg(all(feature = "diarization", feature = "onnx"))]
    fn local_16khz_wav_samples(
        path: &Path,
    ) -> std::result::Result<Vec<f32>, Box<dyn std::error::Error>> {
        let mut reader = hound::WavReader::open(path)?;
        let spec = reader.spec();
        if spec.sample_rate != 16_000 {
            return Err(format!(
                "DIARIZATION_AUDIO_PATH must be 16 kHz WAV, got {} Hz",
                spec.sample_rate
            )
            .into());
        }
        if spec.channels == 0 {
            return Err("DIARIZATION_AUDIO_PATH WAV channel count must be non-zero".into());
        }
        let interleaved = match spec.sample_format {
            hound::SampleFormat::Float => {
                if spec.bits_per_sample != 32 {
                    return Err("float DIARIZATION_AUDIO_PATH WAV must be 32-bit".into());
                }
                reader
                    .samples::<f32>()
                    .collect::<std::result::Result<Vec<_>, _>>()?
            }
            hound::SampleFormat::Int if spec.bits_per_sample <= 16 => reader
                .samples::<i16>()
                .map(|sample| sample.map(|value| value as f32 / 32_768.0))
                .collect::<std::result::Result<Vec<_>, _>>()?,
            hound::SampleFormat::Int => {
                let scale = 2_f32.powi(spec.bits_per_sample as i32 - 1);
                reader
                    .samples::<i32>()
                    .map(|sample| sample.map(|value| value as f32 / scale))
                    .collect::<std::result::Result<Vec<_>, _>>()?
            }
        };
        let channels = spec.channels as usize;
        let samples = if channels == 1 {
            interleaved
        } else {
            interleaved
                .chunks_exact(channels)
                .map(|frame| frame.iter().copied().sum::<f32>() / channels as f32)
                .collect()
        };
        if samples.is_empty() {
            return Err("DIARIZATION_AUDIO_PATH WAV must contain samples".into());
        }
        if !samples.iter().all(|sample| sample.is_finite()) {
            return Err("DIARIZATION_AUDIO_PATH WAV samples must be finite".into());
        }
        Ok(samples)
    }

    #[test]
    fn diarization_options_reject_invalid_speaker_bounds() {
        let mut options = DiarizationOptions {
            min_speakers: Some(0),
            ..DiarizationOptions::default()
        };
        assert!(validate_diarization_options(&options)
            .unwrap_err()
            .to_string()
            .contains("invalid_request"));

        options = DiarizationOptions {
            max_speakers: Some(0),
            ..DiarizationOptions::default()
        };
        assert!(validate_diarization_options(&options)
            .unwrap_err()
            .to_string()
            .contains("invalid_request"));

        options = DiarizationOptions {
            min_speakers: Some(3),
            max_speakers: Some(2),
            ..DiarizationOptions::default()
        };
        assert!(validate_diarization_options(&options)
            .unwrap_err()
            .to_string()
            .contains("invalid_request"));

        options = DiarizationOptions {
            min_speakers: Some(1),
            max_speakers: Some(2),
            ..DiarizationOptions::default()
        };
        validate_diarization_options(&options).unwrap();
    }

    #[test]
    fn batch_options_reject_zero_max_batch_size() {
        let mut vad = FixedVadProvider {
            segments: batch_test_chunks(),
        };
        let mut asr = MockAsrProvider;
        let result = run_transcription_pipeline(
            batch_test_request(CandleWhisperOptions {
                max_batch_size: Some(0),
                ..CandleWhisperOptions::default()
            }),
            &mut vad,
            &mut asr,
            None,
            None,
        );

        let error = result.unwrap_err().to_string();
        assert!(error.contains("invalid_request"));
        assert!(error.contains("max_batch_size"));
    }

    #[test]
    fn batch_chunking_preserves_transcript_order() {
        let mut vad = FixedVadProvider {
            segments: batch_test_chunks(),
        };
        let mut asr = MockAsrProvider;
        let response = run_transcription_pipeline(
            batch_test_request(CandleWhisperOptions {
                batch_chunks: true,
                max_batch_size: Some(2),
                ..CandleWhisperOptions::default()
            }),
            &mut vad,
            &mut asr,
            None,
            None,
        )
        .unwrap();

        let starts = response
            .transcript
            .segments
            .iter()
            .map(|segment| segment.start_seconds.unwrap())
            .collect::<Vec<_>>();
        assert_eq!(starts, vec![0.0, 0.20, 0.40]);
    }

    #[test]
    fn batch_chunking_reports_batch_diagnostics() {
        let mut vad = FixedVadProvider {
            segments: batch_test_chunks(),
        };
        let mut asr = MockAsrProvider;
        let response = run_transcription_pipeline(
            batch_test_request(CandleWhisperOptions {
                batch_chunks: true,
                max_batch_size: Some(2),
                ..CandleWhisperOptions::default()
            }),
            &mut vad,
            &mut asr,
            None,
            None,
        )
        .unwrap();

        assert!(response
            .diagnostics
            .iter()
            .any(|item| item == "chunkCount=3"));
        assert!(response
            .diagnostics
            .iter()
            .any(|item| item == "batchChunks=true"));
        assert!(response
            .diagnostics
            .iter()
            .any(|item| item == "maxBatchSize=2"));
        assert!(response
            .diagnostics
            .iter()
            .any(|item| item == "batchCount=2"));
        assert!(response
            .diagnostics
            .iter()
            .any(|item| item == "batchExecution=candle-whisper-sequential"));
    }

    #[test]
    fn batch_chunking_reports_unbounded_batch_diagnostics() {
        let mut vad = FixedVadProvider {
            segments: batch_test_chunks(),
        };
        let mut asr = MockAsrProvider;
        let response = run_transcription_pipeline(
            batch_test_request(CandleWhisperOptions {
                batch_chunks: true,
                max_batch_size: None,
                ..CandleWhisperOptions::default()
            }),
            &mut vad,
            &mut asr,
            None,
            None,
        )
        .unwrap();

        assert!(response
            .diagnostics
            .iter()
            .any(|item| item == "maxBatchSize=unbounded"));
        assert!(response
            .diagnostics
            .iter()
            .any(|item| item == "batchCount=1"));
    }

    #[test]
    fn batch_disabled_reports_sequential_diagnostics() {
        let mut vad = FixedVadProvider {
            segments: batch_test_chunks(),
        };
        let mut asr = MockAsrProvider;
        let response = run_transcription_pipeline(
            batch_test_request(CandleWhisperOptions {
                batch_chunks: false,
                max_batch_size: Some(2),
                ..CandleWhisperOptions::default()
            }),
            &mut vad,
            &mut asr,
            None,
            None,
        )
        .unwrap();

        assert!(response
            .diagnostics
            .iter()
            .any(|item| item == "batchChunks=false"));
        assert!(response
            .diagnostics
            .iter()
            .any(|item| item == "batchCount=3"));
        assert!(response
            .diagnostics
            .iter()
            .any(|item| item == "batchExecution=candle-whisper-sequential"));
    }

    #[test]
    #[cfg(feature = "diarization")]
    fn speech_spans_from_transcript_prefers_aligned_words(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let transcript = transcript_with_words(vec![("hello", 0.2, 0.5), ("world", 1.0, 1.4)])?;

        let spans = speech_spans_from_transcript(&transcript, 2.0)?;

        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].start_seconds, 0.2);
        assert_eq!(spans[0].end_seconds, 0.5);
        assert_eq!(spans[1].start_seconds, 1.0);
        assert_eq!(spans[1].end_seconds, 1.4);
        assert!(spans[0].start_seconds < spans[1].start_seconds);
        assert!(spans
            .iter()
            .all(|span| span.score.is_finite() && span.score > 0.0));
        Ok(())
    }

    #[test]
    #[cfg(feature = "diarization")]
    fn speech_spans_from_transcript_falls_back_to_segments(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut first = TranscriptSegmentContract::new(0, "hello");
        first.start_seconds = Some(0.2);
        first.end_seconds = Some(0.5);
        let mut second = TranscriptSegmentContract::new(1, "world");
        second.start_seconds = Some(1.0);
        second.end_seconds = Some(1.4);
        let transcript = TranscriptionContract::from_segments(
            None,
            Some("en".to_string()),
            vec![second, first],
        )?;

        let spans = speech_spans_from_transcript(&transcript, 2.0)?;

        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].start_seconds, 0.2);
        assert_eq!(spans[0].end_seconds, 0.5);
        assert_eq!(spans[1].start_seconds, 1.0);
        assert_eq!(spans[1].end_seconds, 1.4);
        Ok(())
    }

    #[test]
    #[cfg(feature = "diarization")]
    fn speech_spans_from_transcript_rejects_out_of_range_timing(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let transcript = transcript_with_words(vec![("hello", 0.2, 2.000_01)])?;

        let error = speech_spans_from_transcript(&transcript, 2.0)
            .unwrap_err()
            .to_string();

        assert!(error.contains("invalid_request"));
        Ok(())
    }

    #[cfg(feature = "alignment")]
    struct AlignmentAwareDiarizationProvider {
        saw_aligned_words: bool,
    }

    #[cfg(feature = "alignment")]
    impl TranscriptDiarizationProvider for AlignmentAwareDiarizationProvider {
        fn provider_id(&self) -> &str {
            "alignment-aware-diarization"
        }

        fn diarize(
            &mut self,
            _audio: LoadedAudio,
            transcript: &TranscriptionContract,
            options: &DiarizationOptions,
        ) -> Result<SpeakerDiarizationResponse> {
            self.saw_aligned_words = transcript.segments.iter().any(|segment| {
                segment.words.iter().any(|word| {
                    word.start_seconds.is_some()
                        && word.end_seconds.is_some()
                        && word.confidence.is_some()
                })
            });
            assert!(
                self.saw_aligned_words,
                "diarization should receive transcript word timings from alignment"
            );
            let mut response = diarization_response_for_tests(vec![SpeakerSegmentPrediction {
                speaker: "SPEAKER_ALIGNED".to_string(),
                start_seconds: 0.0,
                end_seconds: 1.0,
                score: Some(0.95),
            }]);
            response.model_id = options.model_id.clone();
            Ok(response)
        }
    }

    #[cfg(all(feature = "alignment", feature = "candle"))]
    fn write_tiny_wav2vec2_bundle(root: &Path) {
        use candle_core::{Device, Tensor};
        use std::collections::HashMap;

        fs::write(
            root.join("config.json"),
            serde_json::json!({
                "model_type": "wav2vec2",
                "architectures": ["Wav2Vec2ForCTC"],
                "vocab_size": 10,
                "word_delimiter_token": "|",
                "hidden_size": 1,
                "num_hidden_layers": 0,
                "num_attention_heads": 1,
                "intermediate_size": 1,
                "hidden_act": "gelu",
                "layer_norm_eps": 1e-5,
                "feat_extract_activation": "gelu",
                "conv_dim": [1],
                "conv_stride": [1],
                "conv_kernel": [1],
                "conv_bias": false,
                "num_conv_pos_embeddings": 0,
                "num_conv_pos_embedding_groups": 1
            })
            .to_string(),
        )
        .unwrap();
        fs::write(
            root.join("tokenizer.json"),
            serde_json::json!({
                "version": "1.0",
                "word_delimiter_token": "|",
                "model": {
                    "type": "WordLevel",
                    "vocab": {
                        "[PAD]": 0,
                        "H": 1,
                        "E": 2,
                        "L": 3,
                        "O": 4,
                        "|": 5,
                        "W": 6,
                        "R": 7,
                        "D": 8,
                        "<unk>": 9
                    },
                    "unk_token": "<unk>"
                }
            })
            .to_string(),
        )
        .unwrap();
        fs::write(
            root.join("preprocessor_config.json"),
            serde_json::json!({
                "sampling_rate": 16000,
                "do_normalize": false,
                "return_attention_mask": false
            })
            .to_string(),
        )
        .unwrap();

        let device = Device::Cpu;
        let mut tensors = HashMap::new();
        tensors.insert(
            "wav2vec2.feature_extractor.conv_layers.0.conv.weight".to_string(),
            Tensor::new(&[1.0f32], &device)
                .unwrap()
                .reshape((1, 1, 1))
                .unwrap(),
        );
        tensors.insert(
            "wav2vec2.feature_projection.layer_norm.weight".to_string(),
            Tensor::new(&[1.0f32], &device).unwrap(),
        );
        tensors.insert(
            "wav2vec2.feature_projection.layer_norm.bias".to_string(),
            Tensor::new(&[0.0f32], &device).unwrap(),
        );
        tensors.insert(
            "wav2vec2.feature_projection.projection.weight".to_string(),
            Tensor::new(&[1.0f32], &device)
                .unwrap()
                .reshape((1, 1))
                .unwrap(),
        );
        tensors.insert(
            "wav2vec2.feature_projection.projection.bias".to_string(),
            Tensor::new(&[0.0f32], &device).unwrap(),
        );
        tensors.insert(
            "lm_head.weight".to_string(),
            Tensor::new(&[0.0f32; 10], &device)
                .unwrap()
                .reshape((10, 1))
                .unwrap(),
        );
        tensors.insert(
            "lm_head.bias".to_string(),
            Tensor::new(&[0.0f32; 10], &device).unwrap(),
        );
        candle_core::safetensors::save(&tensors, root.join("model.safetensors")).unwrap();
    }

    #[test]
    fn majority_overlap_assigns_word_and_segment_speaker(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut transcript = transcript_with_words(vec![("hello", 0.1, 0.7), ("world", 0.8, 1.2)])?;
        let diarization = diarization_response_for_tests(vec![
            SpeakerSegmentPrediction {
                speaker: "speaker_0".to_string(),
                start_seconds: 0.0,
                end_seconds: 0.75,
                score: Some(0.9),
            },
            SpeakerSegmentPrediction {
                speaker: "speaker_1".to_string(),
                start_seconds: 0.75,
                end_seconds: 1.5,
                score: Some(0.8),
            },
        ]);

        assign_speakers_from_diarization(
            &mut transcript,
            &diarization,
            SpeakerAssignmentPolicy::Majority,
        )?;

        assert_eq!(
            transcript.segments[0].words[0].speaker.as_deref(),
            Some("speaker_0")
        );
        assert_eq!(
            transcript.segments[0].words[1].speaker.as_deref(),
            Some("speaker_1")
        );
        assert_eq!(transcript.segments[0].speaker.as_deref(), Some("speaker_0"));
        Ok(())
    }

    #[test]
    fn nearest_start_policy_assigns_nearest_speaker(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut transcript = transcript_with_words(vec![("hello", 0.72, 0.9)])?;
        let diarization = diarization_response_for_tests(vec![
            SpeakerSegmentPrediction {
                speaker: "speaker_0".to_string(),
                start_seconds: 0.0,
                end_seconds: 0.4,
                score: Some(0.9),
            },
            SpeakerSegmentPrediction {
                speaker: "speaker_1".to_string(),
                start_seconds: 0.7,
                end_seconds: 1.0,
                score: Some(0.9),
            },
        ]);

        assign_speakers_from_diarization(
            &mut transcript,
            &diarization,
            SpeakerAssignmentPolicy::NearestStart,
        )?;

        assert_eq!(
            transcript.segments[0].words[0].speaker.as_deref(),
            Some("speaker_1")
        );
        Ok(())
    }

    #[test]
    fn strict_contained_policy_leaves_uncontained_words_unassigned(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut transcript = transcript_with_words(vec![("hello", 0.2, 0.8)])?;
        let diarization = diarization_response_for_tests(vec![SpeakerSegmentPrediction {
            speaker: "speaker_0".to_string(),
            start_seconds: 0.3,
            end_seconds: 0.7,
            score: Some(0.9),
        }]);

        assign_speakers_from_diarization(
            &mut transcript,
            &diarization,
            SpeakerAssignmentPolicy::StrictContained,
        )?;

        assert!(transcript.segments[0].words[0].speaker.is_none());
        assert!(transcript.segments[0].speaker.is_none());
        Ok(())
    }

    #[test]
    fn existing_segment_speaker_is_not_overwritten(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut transcript = transcript_with_words(vec![("hello", 0.1, 0.4)])?;
        transcript.segments[0].speaker = Some("manual".to_string());
        let diarization = diarization_response_for_tests(vec![SpeakerSegmentPrediction {
            speaker: "speaker_0".to_string(),
            start_seconds: 0.0,
            end_seconds: 1.0,
            score: Some(0.9),
        }]);

        assign_speakers_from_diarization(
            &mut transcript,
            &diarization,
            SpeakerAssignmentPolicy::Majority,
        )?;

        assert_eq!(transcript.segments[0].speaker.as_deref(), Some("manual"));
        assert_eq!(
            transcript.segments[0].words[0].speaker.as_deref(),
            Some("speaker_0")
        );
        Ok(())
    }

    #[test]
    fn segment_speaker_uses_majority_word_speaker_when_words_are_present(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut transcript = transcript_with_words(vec![
            ("one", 0.0, 0.3),
            ("two", 0.3, 0.6),
            ("three", 0.6, 0.9),
        ])?;
        let diarization = diarization_response_for_tests(vec![
            SpeakerSegmentPrediction {
                speaker: "speaker_0".to_string(),
                start_seconds: 0.0,
                end_seconds: 0.65,
                score: Some(0.9),
            },
            SpeakerSegmentPrediction {
                speaker: "speaker_1".to_string(),
                start_seconds: 0.65,
                end_seconds: 1.0,
                score: Some(0.9),
            },
        ]);

        assign_speakers_from_diarization(
            &mut transcript,
            &diarization,
            SpeakerAssignmentPolicy::Majority,
        )?;

        assert_eq!(transcript.segments[0].speaker.as_deref(), Some("speaker_0"));
        Ok(())
    }

    #[test]
    fn segment_without_words_uses_policy_fallback(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut segment = TranscriptSegmentContract::new(0, "hello");
        segment.start_seconds = Some(0.5);
        segment.end_seconds = Some(0.8);
        let mut transcript =
            TranscriptionContract::from_segments(None, Some("en".to_string()), vec![segment])?;
        let diarization = diarization_response_for_tests(vec![
            SpeakerSegmentPrediction {
                speaker: "speaker_0".to_string(),
                start_seconds: 0.0,
                end_seconds: 0.2,
                score: Some(0.9),
            },
            SpeakerSegmentPrediction {
                speaker: "speaker_1".to_string(),
                start_seconds: 0.45,
                end_seconds: 1.0,
                score: Some(0.9),
            },
        ]);

        assign_speakers_from_diarization(
            &mut transcript,
            &diarization,
            SpeakerAssignmentPolicy::StrictContained,
        )?;

        assert_eq!(transcript.segments[0].speaker.as_deref(), Some("speaker_1"));
        Ok(())
    }

    #[test]
    fn offset_chunk_local_segments_skips_global_timing() -> Result<()> {
        let mut segment = TranscriptSegmentContract::new(0, "hello");
        segment.start_seconds = Some(10.0);
        segment.end_seconds = Some(10.5);
        segment
            .attributes
            .insert("timing".to_string(), "global".to_string());
        segment.words.push(TranscriptWordContract {
            text: "hello".to_string(),
            start_seconds: Some(10.0),
            end_seconds: Some(10.5),
            confidence: None,
            speaker: None,
            attributes: BTreeMap::new(),
        });
        let mut transcript =
            TranscriptionContract::from_segments(None, Some("en".to_string()), vec![segment])
                .map_err(|error| DetectError::InvalidArgument(error.to_string()))?;
        let chunks = vec![SpeechActivitySegment::new(5.0, 6.0, 0.8)?];

        offset_chunk_local_segments(&mut transcript, &chunks)?;

        assert_eq!(transcript.segments[0].start_seconds, Some(10.0));
        assert_eq!(transcript.segments[0].end_seconds, Some(10.5));
        assert_eq!(transcript.segments[0].words[0].start_seconds, Some(10.0));
        assert_eq!(transcript.segments[0].words[0].end_seconds, Some(10.5));
        Ok(())
    }

    #[test]
    fn offset_chunk_local_segments_offsets_local_timing() -> Result<()> {
        let mut segment = TranscriptSegmentContract::new(0, "hello");
        segment.start_seconds = Some(0.0);
        segment.end_seconds = Some(0.5);
        let mut transcript =
            TranscriptionContract::from_segments(None, Some("en".to_string()), vec![segment])
                .map_err(|error| DetectError::InvalidArgument(error.to_string()))?;
        let chunks = vec![SpeechActivitySegment::new(5.0, 6.0, 0.8)?];

        offset_chunk_local_segments(&mut transcript, &chunks)?;

        assert_eq!(transcript.segments[0].start_seconds, Some(5.0));
        assert_eq!(transcript.segments[0].end_seconds, Some(5.5));
        assert_eq!(
            transcript.segments[0]
                .attributes
                .get("timing")
                .map(String::as_str),
            Some("global")
        );
        Ok(())
    }

    #[test]
    fn alignment_overwrites_projected_word_timings() -> Result<()> {
        let mut segment = TranscriptSegmentContract::new(0, "hello");
        segment.start_seconds = Some(0.0);
        segment.end_seconds = Some(1.0);
        segment.words.push(TranscriptWordContract {
            text: "hello".to_string(),
            start_seconds: Some(0.0),
            end_seconds: Some(0.5),
            confidence: None,
            speaker: None,
            attributes: BTreeMap::from([(
                "timing".to_string(),
                "whisperTimestampProjection".to_string(),
            )]),
        });
        let mut transcript =
            TranscriptionContract::from_segments(None, Some("en".to_string()), vec![segment])
                .map_err(|error| DetectError::InvalidArgument(error.to_string()))?;

        apply_alignment_words(
            &mut transcript,
            &[AlignedWord {
                segment_index: 0,
                word_index: 0,
                text: "hello".to_string(),
                start_seconds: 0.1,
                end_seconds: 0.4,
                confidence: Some(0.9),
            }],
        )?;

        let word = &transcript.segments[0].words[0];
        assert_eq!(word.text, "hello");
        assert_eq!(word.start_seconds, Some(0.1));
        assert_eq!(word.end_seconds, Some(0.4));
        assert_eq!(word.confidence, Some(0.9));
        assert_eq!(
            word.attributes.get("timing").map(String::as_str),
            Some("whisperTimestampProjection")
        );
        Ok(())
    }

    #[test]
    fn provider_plan_reports_candle_primary_native_provider() {
        let plans = transcription_provider_plans();
        let candle = plans
            .iter()
            .find(|plan| plan.provider_id == "candle-whisper")
            .unwrap();
        assert!(candle.primary);
        assert!(!candle.external_runtime);
        assert!(plans
            .iter()
            .any(|plan| plan.provider_id == "whisperx-command" && !plan.primary));
    }

    #[test]
    fn cuda_request_without_feature_returns_setup_error() {
        let mut provider = CandleWhisperTranscriber::new(CandleWhisperOptions {
            device: NativeDevicePreference::Cuda,
            ..CandleWhisperOptions::default()
        });
        let result = provider.transcribe(AsrRequest {
            audio: LoadedAudio {
                samples: vec![0.0; 16],
                sample_rate: 16_000,
                channels: 1,
                source: None,
            },
            chunks: vec![SpeechActivitySegment::new(0.0, 0.001, 0.0).unwrap()],
            language: None,
            model_id: "openai/whisper-large-v3".to_string(),
        });
        let error = result.unwrap_err().to_string();
        assert!(error.contains("setup_error") || cfg!(feature = "cuda"));
    }

    #[test]
    fn missing_model_bundle_returns_setup_error() {
        let mut provider = CandleWhisperTranscriber::default();
        let result = provider.transcribe(AsrRequest {
            audio: LoadedAudio {
                samples: vec![0.0; 16],
                sample_rate: 16_000,
                channels: 1,
                source: None,
            },
            chunks: vec![SpeechActivitySegment::new(0.0, 0.001, 0.0).unwrap()],
            language: None,
            model_id: "openai/whisper-large-v3".to_string(),
        });
        let error = result.unwrap_err().to_string();
        assert!(error.contains("setup_error") || error.contains("unsupported_runtime"));
    }

    #[test]
    fn empty_audio_returns_invalid_request() {
        let mut provider = CandleWhisperTranscriber::default();
        let result = provider.transcribe(AsrRequest {
            audio: LoadedAudio {
                samples: Vec::new(),
                sample_rate: 16_000,
                channels: 1,
                source: None,
            },
            chunks: vec![SpeechActivitySegment::new(0.0, 0.001, 0.0).unwrap()],
            language: None,
            model_id: "openai/whisper-large-v3".to_string(),
        });
        let error = result.unwrap_err().to_string();
        assert!(error.contains("invalid_request"));
        assert!(error.contains("empty audio"));
    }

    #[test]
    fn non_finite_audio_returns_invalid_request() {
        let mut provider = CandleWhisperTranscriber::default();
        let result = provider.transcribe(AsrRequest {
            audio: LoadedAudio {
                samples: vec![0.0, f32::NAN],
                sample_rate: 16_000,
                channels: 1,
                source: None,
            },
            chunks: vec![SpeechActivitySegment::new(0.0, 0.001, 0.0).unwrap()],
            language: None,
            model_id: "openai/whisper-large-v3".to_string(),
        });
        let error = result.unwrap_err().to_string();
        assert!(error.contains("invalid_request"));
        assert!(error.contains("finite"));
    }

    #[cfg(not(feature = "audio-io"))]
    #[test]
    fn path_non_wav_returns_unsupported_runtime() {
        let result = LoadedAudio::mono_16khz_from_source(&TranscriptionSource::Path {
            path: PathBuf::from("clip.mp4"),
        });
        let error = result.unwrap_err().to_string();
        assert!(error.contains("unsupported_runtime"));
        assert!(error.contains("WAV"));
    }

    #[test]
    fn wav_path_decodes_to_mono_16khz() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("stereo-8khz.wav");
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 8_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&path, spec)?;
        for _ in 0..8_000 {
            writer.write_sample::<i16>(16_384)?;
            writer.write_sample::<i16>(0)?;
        }
        writer.finalize()?;

        let audio = LoadedAudio::mono_16khz_from_source(&TranscriptionSource::Path { path })?;
        assert_eq!(audio.sample_rate, 16_000);
        assert_eq!(audio.channels, 1);
        assert_eq!(audio.samples.len(), 16_000);
        assert!(audio.samples.iter().all(|sample| sample.is_finite()));
        assert!(audio.samples.iter().any(|sample| *sample > 0.20));
        Ok(())
    }

    #[test]
    fn decode_diagnostics_report_direct_samples_route() {
        let (audio, diagnostics) =
            native_audio::mono_16khz_from_source_with_diagnostics(&TranscriptionSource::Samples {
                samples: vec![0.0, 1.0],
                sample_rate: 8_000,
                channels: 1,
                source: Some("inline".to_string()),
            })
            .unwrap();

        assert_eq!(diagnostics.decode_route, "direct-samples");
        assert_eq!(diagnostics.source_path_extension, None);
        assert_eq!(diagnostics.input_sample_rate, Some(8_000));
        assert_eq!(diagnostics.output_sample_rate, 16_000);
        assert_eq!(diagnostics.output_channels, 1);
        assert_eq!(audio.sample_rate, 16_000);
        assert_eq!(audio.channels, 1);
    }

    #[test]
    fn decode_diagnostics_report_wav_route() -> std::result::Result<(), Box<dyn std::error::Error>>
    {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("diagnostic.wav");
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 8_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&path, spec)?;
        writer.write_sample::<i16>(16_384)?;
        writer.finalize()?;

        let (_audio, diagnostics) =
            native_audio::mono_16khz_from_source_with_diagnostics(&TranscriptionSource::Path {
                path,
            })?;

        assert_eq!(diagnostics.decode_route, "native-wav-reader");
        assert_eq!(diagnostics.source_path_extension.as_deref(), Some("wav"));
        assert_eq!(diagnostics.input_sample_rate, Some(8_000));
        assert_eq!(diagnostics.output_sample_rate, 16_000);
        assert_eq!(diagnostics.output_channels, 1);
        Ok(())
    }

    #[test]
    fn wav_path_still_uses_native_reader_without_audio_io(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("native-reader.wav");
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 16_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&path, spec)?;
        writer.write_sample::<i16>(16_384)?;
        writer.write_sample::<i16>(-16_384)?;
        writer.finalize()?;

        let audio =
            LoadedAudio::mono_16khz_from_source(&TranscriptionSource::Path { path: path.clone() })?;

        assert_eq!(audio.sample_rate, 16_000);
        assert_eq!(audio.channels, 1);
        assert_eq!(
            audio.source.as_deref(),
            Some(path.to_string_lossy().as_ref())
        );
        assert_eq!(audio.samples.len(), 2);
        assert!(audio.samples[0] > 0.49);
        assert!(audio.samples[1] < -0.49);
        Ok(())
    }

    #[cfg(feature = "audio-io")]
    #[test]
    #[ignore = "requires RUN_NATIVE_MEDIA_DECODE_TESTS=1 and a local FFmpeg-decodable media file"]
    fn native_media_decode_when_requested() -> std::result::Result<(), Box<dyn std::error::Error>> {
        if std::env::var("RUN_NATIVE_MEDIA_DECODE_TESTS")
            .ok()
            .as_deref()
            != Some("1")
        {
            eprintln!("set RUN_NATIVE_MEDIA_DECODE_TESTS=1 to run native media decode smoke");
            return Ok(());
        }
        let path = std::env::var_os("TRANSCRIPTION_MEDIA_PATH")
            .map(PathBuf::from)
            .map(resolve_smoke_path)
            .ok_or("TRANSCRIPTION_MEDIA_PATH must point to a local media file")?;

        let (audio, diagnostics) =
            native_audio::mono_16khz_from_source_with_diagnostics(&TranscriptionSource::Path {
                path: path.clone(),
            })?;

        assert_eq!(audio.sample_rate, 16_000);
        assert_eq!(audio.channels, 1);
        assert!(!audio.samples.is_empty());
        assert!(audio.samples.iter().all(|sample| sample.is_finite()));
        assert_eq!(diagnostics.decode_route, "audio-io-media-decode");
        assert_eq!(diagnostics.output_sample_rate, 16_000);
        assert_eq!(diagnostics.output_channels, 1);
        assert!(diagnostics.input_sample_rate.is_some());
        assert_eq!(
            audio.source.as_deref(),
            Some(path.to_string_lossy().as_ref())
        );
        Ok(())
    }

    #[cfg(feature = "audio-io")]
    fn resolve_smoke_path(path: PathBuf) -> PathBuf {
        if path.is_absolute() || path.exists() {
            return path;
        }
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let Some(workspace_root) = manifest_dir.ancestors().nth(3) else {
            return path;
        };
        let workspace_path = workspace_root.join(&path);
        if workspace_path.exists() {
            workspace_path
        } else {
            path
        }
    }

    #[test]
    fn vad_splits_deterministic_synthetic_speech() {
        let request = sample_request();
        let audio = LoadedAudio::mono_16khz_from_source(&request.source).unwrap();
        let mut vad = EnergyVadTranscriptionProvider;
        let response = vad
            .detect_speech(VadRequest {
                audio,
                options: request.vad,
            })
            .unwrap();
        assert_eq!(response.segments.len(), 1);
        assert!(response.segments[0].start_seconds < 0.07);
        assert!(response.segments[0].end_seconds > 0.30);
    }

    #[test]
    fn mock_pipeline_normalizes_offsets_alignment_and_diarization() {
        let mut request = sample_request();
        request.alignment.enabled = true;
        request.diarization.enabled = true;
        let mut vad = EnergyVadTranscriptionProvider;
        let mut asr = MockAsrProvider;
        let mut aligner = MockAlignmentProvider;
        let mut diarizer = MockDiarizationProvider;
        let response = run_transcription_pipeline(
            request,
            &mut vad,
            &mut asr,
            Some(&mut aligner),
            Some(&mut diarizer),
        )
        .unwrap();
        assert!(response.accepted);
        assert_eq!(response.provider, "candle-whisper");
        assert_eq!(response.alignment.as_ref().unwrap().word_count, 1);
        assert_eq!(
            response.transcript.segments[0].words[0].speaker.as_deref(),
            Some("SPEAKER_00")
        );
        assert_eq!(
            response.transcript.segments[0].speaker.as_deref(),
            Some("SPEAKER_00")
        );
    }

    #[test]
    fn native_pipeline_reports_diarization_diagnostics() {
        let mut request = sample_request();
        request.diarization = DiarizationOptions {
            enabled: true,
            min_speakers: Some(2),
            max_speakers: Some(3),
            ..DiarizationOptions::default()
        };
        let mut vad = EnergyVadTranscriptionProvider;
        let mut asr = MockAsrProvider;
        let mut diarizer = MockDiarizationProvider;

        let response =
            run_transcription_pipeline(request, &mut vad, &mut asr, None, Some(&mut diarizer))
                .unwrap();

        assert!(response
            .diagnostics
            .iter()
            .any(|item| item == "diarizationProvider=mock-diarization"));
        assert!(response
            .diagnostics
            .iter()
            .any(|item| item == "diarizationSegmentCount=1"));
        assert!(response
            .diagnostics
            .iter()
            .any(|item| item == "diarizationSpeakerCount=1"));
        assert!(response
            .diagnostics
            .iter()
            .any(|item| item == "diarizationMinSpeakers=2"));
        assert!(response
            .diagnostics
            .iter()
            .any(|item| item == "diarizationSpeakerCountBelowRequestedMin=1/2"));
    }

    #[cfg(feature = "diarization")]
    #[test]
    fn native_pipeline_reports_onnx_diarization_diagnostics() {
        let mut request = sample_request();
        request.diarization = DiarizationOptions {
            enabled: true,
            speaker_embedding_model_bundle: Some(PathBuf::from("speaker-model")),
            speaker_embedding_dimension: Some(2),
            ..DiarizationOptions::default()
        };
        let mut vad = EnergyVadTranscriptionProvider;
        let mut asr = MockAsrProvider;
        let mut diarizer = MockOnnxDiarizationProvider;

        let response =
            run_transcription_pipeline(request, &mut vad, &mut asr, None, Some(&mut diarizer))
                .unwrap();

        assert!(response
            .diagnostics
            .iter()
            .any(|item| item == "diarizationRuntime=onnx"));
        assert!(response
            .diagnostics
            .iter()
            .any(|item| item == "speakerEmbeddingProvider=onnx"));
        assert!(response
            .diagnostics
            .iter()
            .any(|item| item == "speakerEmbeddingDimension=2"));
        assert!(response
            .diagnostics
            .iter()
            .any(|item| item == "diarizationBaseline=false"));
        assert!(!response
            .diagnostics
            .iter()
            .any(|item| item == "diarizationBaseline=heuristic-native"));
        assert_eq!(
            response.transcript.segments[0].speaker.as_deref(),
            Some("SPEAKER_ONNX")
        );
    }

    #[cfg(feature = "diarization")]
    #[test]
    fn native_onnx_diarization_missing_bundle_returns_setup_error() {
        let mut request = sample_request();
        request.diarization = DiarizationOptions {
            enabled: true,
            speaker_embedding_model_bundle: Some(PathBuf::from(
                "/definitely/missing/onnx-speaker-model",
            )),
            speaker_embedding_dimension: Some(2),
            ..DiarizationOptions::default()
        };
        let mut vad = EnergyVadTranscriptionProvider;
        let mut asr = MockAsrProvider;
        let mut diarizer = NativeSpeakerDiarizationProvider;

        let error =
            run_native_transcription_pipeline(request, &mut vad, &mut asr, Some(&mut diarizer))
                .unwrap_err()
                .to_string();

        assert!(error.contains("setup_error"));
        assert!(error.contains("ONNX speaker embedding"));
    }

    #[cfg(all(feature = "diarization", feature = "onnx"))]
    #[test]
    #[ignore = "requires RUN_NATIVE_SPEAKER_MODEL_TESTS=1 and caller-owned local ONNX speaker model/audio"]
    fn native_onnx_diarization_smoke_when_requested(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        if std::env::var("RUN_NATIVE_SPEAKER_MODEL_TESTS").as_deref() != Ok("1") {
            eprintln!(
                "skipping native ONNX diarization smoke; set RUN_NATIVE_SPEAKER_MODEL_TESTS=1"
            );
            return Ok(());
        }

        let bundle_path = std::env::var_os("SPEAKER_EMBEDDING_MODEL_BUNDLE")
            .map(PathBuf::from)
            .ok_or("SPEAKER_EMBEDDING_MODEL_BUNDLE is required")?;
        let audio_path = std::env::var_os("DIARIZATION_AUDIO_PATH")
            .map(PathBuf::from)
            .ok_or("DIARIZATION_AUDIO_PATH is required")?;
        let embedding_dimension = std::env::var("SPEAKER_EMBEDDING_DIMENSION")
            .ok()
            .map(|value| value.parse::<usize>())
            .transpose()?
            .unwrap_or(192);
        let samples = local_16khz_wav_samples(&audio_path)?;
        let duration_seconds = samples.len() as f64 / 16_000.0;
        let midpoint = (duration_seconds / 2.0).clamp(1.0 / 16_000.0, duration_seconds);
        let first_end = midpoint.min(duration_seconds);
        let second_start = first_end;
        let second_end = duration_seconds;

        let mut request = TranscriptionPipelineRequest {
            source: TranscriptionSource::Samples {
                samples,
                sample_rate: 16_000,
                channels: 1,
                source: Some(audio_path.to_string_lossy().into_owned()),
            },
            provider: TranscriptionProviderSelection::CandleWhisper(CandleWhisperOptions::default()),
            vad: VadOptions::default(),
            alignment: AlignmentOptions::default(),
            diarization: DiarizationOptions {
                enabled: true,
                speaker_embedding_model_bundle: Some(bundle_path),
                speaker_embedding_dimension: Some(embedding_dimension),
                speaker_embedding_sample_rate: Some(16_000),
                assignment_policy: SpeakerAssignmentPolicy::StrictContained,
                ..DiarizationOptions::default()
            },
            output: TranscriptionOutputOptions::default(),
        };
        if second_end > second_start {
            request.vad.enabled = true;
        }

        let segments = if second_end > second_start {
            vec![
                SpeechActivitySegment::new(0.0, first_end, 1.0)?,
                SpeechActivitySegment::new(second_start, second_end, 1.0)?,
            ]
        } else {
            vec![SpeechActivitySegment::new(0.0, first_end, 1.0)?]
        };
        let mut vad = FixedVadProvider { segments };
        let mut asr = MockAsrProvider;
        let mut diarizer = NativeSpeakerDiarizationProvider;

        let response =
            run_native_transcription_pipeline(request, &mut vad, &mut asr, Some(&mut diarizer))?;
        eprintln!("{}", response.diagnostics.join("\n"));

        assert!(response.accepted);
        assert!(response
            .diagnostics
            .iter()
            .any(|item| item == "diarizationRuntime=onnx"));
        assert!(response
            .diagnostics
            .iter()
            .any(|item| item == "speakerEmbeddingProvider=onnx"));
        assert!(response
            .diagnostics
            .iter()
            .any(|item| item == &format!("speakerEmbeddingDimension={embedding_dimension}")));
        assert!(response
            .diagnostics
            .iter()
            .any(|item| item == "diarizationBaseline=false"));
        assert!(response.transcript.segments.iter().all(|segment| segment
            .speaker
            .as_deref()
            .is_some_and(|speaker| { !speaker.trim().is_empty() })));
        normalize_transcription_contract(response.transcript.clone())
            .map_err(|error| format!("transcript speaker assignment must validate: {error}"))?;
        Ok(())
    }

    #[test]
    fn native_pipeline_diarization_invalid_bounds_errors_before_provider() {
        let mut request = sample_request();
        request.diarization = DiarizationOptions {
            enabled: true,
            min_speakers: Some(3),
            max_speakers: Some(2),
            ..DiarizationOptions::default()
        };
        let mut vad = EnergyVadTranscriptionProvider;
        let mut asr = MockAsrProvider;
        let mut diarizer = PanickingDiarizationProvider { called: false };

        let error =
            run_transcription_pipeline(request, &mut vad, &mut asr, None, Some(&mut diarizer))
                .unwrap_err()
                .to_string();

        assert!(error.contains("invalid_request"));
        assert!(!diarizer.called);
    }

    #[test]
    #[cfg(not(feature = "diarization"))]
    fn native_diarization_without_feature_still_reports_no_provider() {
        let mut request = sample_request();
        request.diarization.enabled = true;
        let mut vad = EnergyVadTranscriptionProvider;
        let mut asr = MockAsrProvider;

        let error = run_transcription_pipeline(request, &mut vad, &mut asr, None, None)
            .unwrap_err()
            .to_string();

        assert!(error.contains("setup_error"));
        assert!(error.contains("no diarization provider is available"));
    }

    #[test]
    #[cfg(feature = "diarization")]
    fn native_speaker_diarization_provider_uses_transcript_spans_when_available(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let samples = (0..32_000)
            .map(|index| if index % 80 < 40 { 0.2 } else { -0.2 })
            .collect::<Vec<_>>();
        let audio = LoadedAudio {
            samples,
            sample_rate: 16_000,
            channels: 1,
            source: Some("synthetic".to_string()),
        };
        let transcript = transcript_with_words(vec![("hello", 0.20, 0.50), ("world", 1.00, 1.40)])?;
        let options = DiarizationOptions {
            enabled: true,
            model_id: "requested-native-speakers".to_string(),
            ..DiarizationOptions::default()
        };
        let mut provider = NativeSpeakerDiarizationProvider;

        let response = provider.diarize(audio, &transcript, &options)?;

        assert_eq!(response.model_id, "requested-native-speakers");
        assert_eq!(
            response.runtime,
            audio_analysis_speakers::AudioRuntime::Heuristic
        );
        assert_eq!(response.segments.len(), 2);
        assert!((response.segments[0].start_seconds - 0.20).abs() < 0.001);
        assert!((response.segments[0].end_seconds - 0.50).abs() < 0.001);
        assert!((response.segments[1].start_seconds - 1.00).abs() < 0.001);
        assert!((response.segments[1].end_seconds - 1.40).abs() < 0.001);
        assert!(response
            .segments
            .iter()
            .all(|segment| segment.speaker.starts_with("speaker_")));
        Ok(())
    }

    #[test]
    #[cfg(feature = "diarization")]
    fn native_speaker_diarization_provider_falls_back_without_transcript_timing(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let samples = (0..16_000)
            .map(|index| {
                if (1_000..8_000).contains(&index) {
                    0.2
                } else {
                    0.0
                }
            })
            .collect::<Vec<_>>();
        let audio = LoadedAudio {
            samples,
            sample_rate: 16_000,
            channels: 1,
            source: Some("synthetic".to_string()),
        };
        let segment = TranscriptSegmentContract::new(0, "hello without timing");
        let transcript =
            TranscriptionContract::from_segments(None, Some("en".to_string()), vec![segment])?;
        let options = DiarizationOptions {
            enabled: true,
            model_id: "fallback-native-speakers".to_string(),
            ..DiarizationOptions::default()
        };
        let mut provider = NativeSpeakerDiarizationProvider;

        let response = provider.diarize(audio, &transcript, &options)?;

        assert_eq!(response.model_id, "fallback-native-speakers");
        assert_eq!(response.operation, "audio.speakers.diarize");
        assert_eq!(
            response.runtime,
            audio_analysis_speakers::AudioRuntime::Heuristic
        );
        assert!(!response.segments.is_empty());
        Ok(())
    }

    #[test]
    #[cfg(feature = "alignment")]
    fn native_pipeline_supplies_alignment_provider_when_enabled() {
        let mut request = sample_request();
        request.alignment = AlignmentOptions {
            enabled: true,
            model_bundle: None,
            ..AlignmentOptions::default()
        };
        let mut vad = EnergyVadTranscriptionProvider;
        let mut asr = MockAsrProvider;

        let response =
            run_native_transcription_pipeline(request, &mut vad, &mut asr, None).unwrap();

        let alignment = response.alignment.as_ref().unwrap();
        assert_eq!(alignment.provider, "ctc-forced-aligner");
        assert_eq!(alignment.model_id, default_alignment_model());
        assert_eq!(alignment.word_count, 1);
        let word = &response.transcript.segments[0].words[0];
        assert_eq!(word.text, "hello");
        assert!(word.start_seconds.is_some());
        assert!(word.end_seconds.is_some());
        assert_eq!(word.confidence, Some(1.0));
        assert!(response
            .diagnostics
            .iter()
            .any(|item| item == "deterministic transcript timing alignment completed"));
        response
            .transcript
            .validate_strict()
            .expect("native pipeline response should be strictly valid");
    }

    #[test]
    fn native_pipeline_leaves_alignment_absent_when_disabled() {
        let request = sample_request();
        let mut vad = EnergyVadTranscriptionProvider;
        let mut asr = MockAsrProvider;

        let response =
            run_native_transcription_pipeline(request, &mut vad, &mut asr, None).unwrap();

        assert!(response.alignment.is_none());
        assert!(response
            .diagnostics
            .iter()
            .all(|item| !item.to_lowercase().contains("alignment")));
        response
            .transcript
            .validate_strict()
            .expect("native pipeline response should be strictly valid");
    }

    #[test]
    #[cfg(feature = "alignment")]
    fn native_pipeline_runs_alignment_before_diarization() {
        let mut request = sample_request();
        request.alignment.enabled = true;
        request.diarization.enabled = true;
        let mut vad = EnergyVadTranscriptionProvider;
        let mut asr = MockAsrProvider;
        let mut diarizer = AlignmentAwareDiarizationProvider {
            saw_aligned_words: false,
        };

        let response =
            run_native_transcription_pipeline(request, &mut vad, &mut asr, Some(&mut diarizer))
                .unwrap();

        assert!(diarizer.saw_aligned_words);
        assert!(response.diarization.is_some());
        let word = &response.transcript.segments[0].words[0];
        assert!(word.start_seconds.is_some());
        assert!(word.end_seconds.is_some());
        assert_eq!(word.speaker.as_deref(), Some("SPEAKER_ALIGNED"));
        assert_eq!(
            response.transcript.segments[0].speaker.as_deref(),
            Some("SPEAKER_ALIGNED")
        );
    }

    #[test]
    #[cfg(not(feature = "alignment"))]
    fn native_pipeline_alignment_without_feature_reports_alignment_unsupported_runtime() {
        let mut request = sample_request();
        request.alignment.enabled = true;
        let mut vad = EnergyVadTranscriptionProvider;
        let mut asr = MockAsrProvider;

        let error = run_native_transcription_pipeline(request, &mut vad, &mut asr, None)
            .unwrap_err()
            .to_string();

        assert!(error.contains("unsupported_runtime"));
        assert!(error.contains("CTC alignment"));
        assert!(error.contains("alignment"));
        assert!(!error.contains("no alignment provider is available"));
    }

    #[test]
    #[cfg(all(feature = "alignment", feature = "candle"))]
    fn native_pipeline_with_tiny_wav2vec2_bundle_runs_model_backed_alignment(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        write_tiny_wav2vec2_bundle(temp.path());
        let mut request = sample_request();
        request.alignment = AlignmentOptions {
            enabled: true,
            model_bundle: Some(temp.path().to_path_buf()),
            ..AlignmentOptions::default()
        };
        let mut vad = EnergyVadTranscriptionProvider;
        let mut asr = MockAsrProvider;

        let response = run_native_transcription_pipeline(request, &mut vad, &mut asr, None)?;

        let alignment = response.alignment.as_ref().unwrap();
        assert_eq!(alignment.provider, "ctc-forced-aligner");
        assert_eq!(alignment.word_count, 1);
        assert!(response
            .diagnostics
            .iter()
            .any(|item| item == "alignmentModelExecution=candle-wav2vec2"));
        let word = &response.transcript.segments[0].words[0];
        assert_eq!(word.text, "hello");
        assert!(word.start_seconds.is_some());
        assert!(word.end_seconds.is_some());
        assert!(word.confidence.is_some());
        response.transcript.validate_strict()?;
        Ok(())
    }

    #[test]
    fn missing_command_returns_setup_error() {
        let result = transcribe(TranscriptionPipelineRequest {
            source: TranscriptionSource::Path {
                path: PathBuf::from("missing.wav"),
            },
            provider: TranscriptionProviderSelection::ExternalWhisperX(WhisperXCommandOptions {
                command: PathBuf::from("definitely-missing-whisperx-command"),
                ..WhisperXCommandOptions::default()
            }),
            vad: VadOptions::default(),
            alignment: AlignmentOptions::default(),
            diarization: DiarizationOptions::default(),
            output: TranscriptionOutputOptions::default(),
        });

        let error = result.unwrap_err().to_string();
        assert!(error.contains("setup_error"));
        assert!(error.contains("not found"));
    }

    #[test]
    fn diarization_requires_token_before_spawn() {
        let result = transcribe(TranscriptionPipelineRequest {
            source: TranscriptionSource::Path {
                path: PathBuf::from("missing.wav"),
            },
            provider: TranscriptionProviderSelection::ExternalWhisperX(WhisperXCommandOptions {
                command: PathBuf::from("definitely-missing-whisperx-command"),
                diarize: true,
                hf_token_env: Some("VIDEO_ANALYSIS_TEST_MISSING_HF_TOKEN".to_string()),
                ..WhisperXCommandOptions::default()
            }),
            vad: VadOptions::default(),
            alignment: AlignmentOptions::default(),
            diarization: DiarizationOptions::default(),
            output: TranscriptionOutputOptions::default(),
        });

        let error = result.unwrap_err().to_string();
        assert!(error.contains("diarization requires"));
        assert!(!error.contains("not found"));
    }

    #[test]
    fn mock_command_output_round_trips() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let command = temp.path().join("mock-whisperx.sh");
        let output_dir = temp.path().join("out");
        fs::write(
            &command,
            format!(
                "#!/usr/bin/env bash\nmkdir -p \"{}\"\ncat > \"{}/sample.json\" <<'JSON'\n{{\"segments\":[{{\"start\":0.0,\"end\":1.0,\"text\":\" hello \",\"speaker\":\"SPEAKER_00\",\"words\":[{{\"word\":\"hello\",\"start\":0.0,\"end\":0.8,\"score\":0.9,\"speaker\":\"SPEAKER_00\"}}]}}]}}\nJSON\n",
                output_dir.display(),
                output_dir.display()
            ),
        )?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(&command)?.permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&command, permissions)?;
        }

        let response = transcribe(TranscriptionPipelineRequest {
            source: TranscriptionSource::Path {
                path: PathBuf::from("speech.wav"),
            },
            provider: TranscriptionProviderSelection::ExternalWhisperX(WhisperXCommandOptions {
                command,
                output_dir: Some(output_dir),
                ..WhisperXCommandOptions::default()
            }),
            vad: VadOptions::default(),
            alignment: AlignmentOptions::default(),
            diarization: DiarizationOptions::default(),
            output: TranscriptionOutputOptions::default(),
        })?;

        assert!(response.accepted);
        assert_eq!(response.transcript.text.as_deref(), Some("hello"));
        assert_eq!(
            response.transcript.segments[0].words[0].speaker.as_deref(),
            Some("SPEAKER_00")
        );
        Ok(())
    }

    #[test]
    fn timeout_returns_typed_error() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let command = temp.path().join("slow-whisperx.sh");
        fs::write(&command, "#!/usr/bin/env bash\nsleep 2\n")?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(&command)?.permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&command, permissions)?;
        }

        let result = transcribe(TranscriptionPipelineRequest {
            source: TranscriptionSource::Path {
                path: PathBuf::from("speech.wav"),
            },
            provider: TranscriptionProviderSelection::ExternalWhisperX(WhisperXCommandOptions {
                command,
                timeout_seconds: Some(1),
                ..WhisperXCommandOptions::default()
            }),
            vad: VadOptions::default(),
            alignment: AlignmentOptions::default(),
            diarization: DiarizationOptions::default(),
            output: TranscriptionOutputOptions::default(),
        });
        assert!(result.unwrap_err().to_string().contains("timeout"));
        Ok(())
    }

    #[test]
    #[ignore]
    fn candle_whisper_cuda_smoke_when_requested() {
        if std::env::var("RUN_NATIVE_TRANSCRIPTION_TESTS").as_deref() != Ok("1") {
            eprintln!("skipping native transcription smoke; set RUN_NATIVE_TRANSCRIPTION_TESTS=1");
            return;
        }
        #[cfg(not(all(feature = "candle", feature = "cuda", feature = "model-bundles")))]
        panic!("native transcription smoke requires candle,cuda,model-bundles features");

        #[cfg(all(feature = "candle", feature = "cuda", feature = "model-bundles"))]
        {
            let bundle = std::env::var_os("TRANSCRIPTION_MODEL_BUNDLE")
                .map(PathBuf::from)
                .expect("TRANSCRIPTION_MODEL_BUNDLE is required");
            let audio_path = std::env::var_os("TRANSCRIPTION_AUDIO_PATH")
                .map(PathBuf::from)
                .expect("TRANSCRIPTION_AUDIO_PATH is required");
            let response = transcribe(TranscriptionPipelineRequest {
                source: TranscriptionSource::Path { path: audio_path },
                provider: TranscriptionProviderSelection::CandleWhisper(CandleWhisperOptions {
                    model_id: "openai/whisper-tiny".to_string(),
                    device: NativeDevicePreference::Cuda,
                    language: Some("en".to_string()),
                    model_bundle: Some(bundle),
                    ..CandleWhisperOptions::default()
                }),
                vad: VadOptions::default(),
                alignment: AlignmentOptions::default(),
                diarization: DiarizationOptions::default(),
                output: TranscriptionOutputOptions::default(),
            })
            .expect("native Candle Whisper CUDA transcription should run");
            eprintln!("{}", response.diagnostics.join("\n"));
            assert!(response.accepted);
            assert!(response
                .diagnostics
                .iter()
                .any(|item| item == "provider=candle-whisper"));
            assert!(response
                .diagnostics
                .iter()
                .any(|item| item == "device=cuda:0"));
            assert!(response
                .diagnostics
                .iter()
                .any(|item| item.starts_with("modelId=")));
            assert!(response
                .diagnostics
                .iter()
                .any(|item| item.starts_with("bundle=")));
            assert!(response
                .diagnostics
                .iter()
                .any(|item| item.starts_with("chunkCount=")));
            assert!(response.diagnostics.iter().any(|item| item == "cuda=true"));
            assert!(
                response
                    .transcript
                    .text
                    .as_deref()
                    .is_some_and(|text| !text.trim().is_empty())
                    || !response.transcript.segments.is_empty()
            );
        }
    }

    #[test]
    #[ignore]
    fn ctc_alignment_wav2vec2_smoke_when_requested() {
        if std::env::var("RUN_NATIVE_ALIGNMENT_TESTS").as_deref() != Ok("1") {
            eprintln!("skipping native alignment smoke; set RUN_NATIVE_ALIGNMENT_TESTS=1");
            return;
        }
        #[cfg(not(all(feature = "candle", feature = "alignment", feature = "model-bundles")))]
        panic!("native alignment smoke requires candle,alignment,model-bundles features");

        #[cfg(all(feature = "candle", feature = "alignment", feature = "model-bundles"))]
        {
            let bundle = std::env::var_os("ALIGNMENT_MODEL_BUNDLE")
                .map(PathBuf::from)
                .expect("ALIGNMENT_MODEL_BUNDLE is required");
            let audio_path = std::env::var_os("TRANSCRIPTION_AUDIO_PATH")
                .map(PathBuf::from)
                .expect("TRANSCRIPTION_AUDIO_PATH is required");
            let layout = native_wav2vec2::inspect_wav2vec2_bundle_layout(&bundle)
                .expect("alignment smoke should inspect wav2vec2 bundle layout");
            eprintln!("wav2vec2 layout report: {layout:#?}");
            let transcript_text = std::env::var("ALIGNMENT_TRANSCRIPT_TEXT")
                .unwrap_or_else(|_| "hello world".to_string());
            let audio = LoadedAudio::mono_16khz_from_source(&TranscriptionSource::Path {
                path: audio_path,
            })
            .expect("alignment smoke requires readable WAV audio");
            let mut segment = TranscriptSegmentContract::new(0, transcript_text.clone());
            segment.start_seconds = Some(0.0);
            segment.end_seconds = Some(audio.duration_seconds().clamp(1.0 / 16_000.0, 1.0));
            let transcript =
                TranscriptionContract::from_segments(None, Some("en".to_string()), vec![segment])
                    .expect("alignment smoke transcript should validate");
            let request = AlignmentRequest {
                audio,
                transcript,
                language: Some("en".to_string()),
                model_id: default_alignment_model(),
            };
            let mut aligner = CtcForcedAligner {
                options: AlignmentOptions {
                    enabled: true,
                    model_bundle: Some(bundle),
                    ..AlignmentOptions::default()
                },
            };
            let response = aligner
                .align(request)
                .expect("native wav2vec2/CTC alignment should run");
            assert!(!response.words.is_empty());
            assert!(response
                .words
                .iter()
                .all(|word| word.end_seconds >= word.start_seconds));
            assert!(response.words.iter().all(|word| word.confidence.is_some()));
            assert!(response
                .diagnostics
                .iter()
                .any(|item| item == "alignmentModelExecution=candle-wav2vec2"));
        }
    }
}
