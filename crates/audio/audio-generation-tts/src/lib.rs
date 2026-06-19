#![doc = include_str!("../README.md")]

pub mod surface;

use serde::{Deserialize, Serialize};
#[cfg(feature = "asr")]
use std::path::PathBuf;

/// In-memory PCM audio used by TTS inputs and outputs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PcmAudio {
    /// Samples per second.
    pub sample_rate_hz: u32,
    /// Channel count.
    pub channels: u16,
    /// Interleaved normalized PCM samples.
    pub samples: Vec<f32>,
}

impl PcmAudio {
    /// Validates this PCM buffer.
    pub fn validate(&self, field: &str) -> Result<(), String> {
        if self.sample_rate_hz == 0 {
            return Err(format!(
                "invalid request: `{field}.sampleRateHz` must be greater than zero"
            ));
        }
        if self.channels == 0 {
            return Err(format!(
                "invalid request: `{field}.channels` must be greater than zero"
            ));
        }
        if self.samples.is_empty() {
            return Err(format!(
                "invalid request: `{field}.samples` must not be empty"
            ));
        }
        if self.samples.iter().any(|sample| !sample.is_finite()) {
            return Err(format!(
                "invalid request: `{field}.samples` must contain only finite values"
            ));
        }
        Ok(())
    }
}

/// Reference audio source used by a Reference Voice Prompt.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", untagged)]
pub enum ReferenceVoicePromptAudio {
    /// In-memory PCM samples supplied by the caller.
    Samples(PcmAudio),
    /// Path to caller-managed reference audio.
    Path { path: String },
}

impl ReferenceVoicePromptAudio {
    /// Validates the reference audio source without opening files.
    pub fn validate(&self, field: &str) -> Result<(), String> {
        match self {
            Self::Samples(audio) => audio.validate(field),
            Self::Path { path } => {
                if path.trim().is_empty() {
                    return Err(format!(
                        "invalid request: `{field}.path` must not be empty when provided"
                    ));
                }
                Ok(())
            }
        }
    }

    /// Stable source-kind string for package-surface plans.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Samples(_) => "samples",
            Self::Path { .. } => "path",
        }
    }

    /// Converts this source into the transcription crate source contract.
    #[cfg(feature = "asr")]
    pub fn to_transcription_source(&self) -> audio_analysis_transcription::TranscriptionSource {
        match self {
            Self::Samples(audio) => audio_analysis_transcription::TranscriptionSource::Samples {
                samples: audio.samples.clone(),
                sample_rate: audio.sample_rate_hz,
                channels: audio.channels,
                source: None,
            },
            Self::Path { path } => audio_analysis_transcription::TranscriptionSource::Path {
                path: PathBuf::from(path),
            },
        }
    }
}

/// ASR fallback configuration for a transcript-missing Reference Voice Prompt.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReferencePromptAsrFallback {
    /// Planned transcription provider id from the transcription crate surface.
    #[serde(default = "default_reference_prompt_asr_provider")]
    pub provider_id: String,
    /// Optional model id to pass to the planned transcription provider.
    #[serde(default)]
    pub model_id: Option<String>,
    /// Optional language hint for ASR fallback.
    #[serde(default)]
    pub language: Option<String>,
}

impl Default for ReferencePromptAsrFallback {
    fn default() -> Self {
        Self {
            provider_id: default_reference_prompt_asr_provider(),
            model_id: None,
            language: None,
        }
    }
}

impl ReferencePromptAsrFallback {
    /// Validates ASR fallback planning fields.
    pub fn validate(&self) -> Result<(), String> {
        if self.provider_id.trim().is_empty() {
            return Err(
                "invalid request: `referenceVoicePrompt.asrFallback.providerId` must not be empty"
                    .to_string(),
            );
        }
        if self
            .model_id
            .as_deref()
            .is_some_and(|model_id| model_id.trim().is_empty())
        {
            return Err(
                "invalid request: `referenceVoicePrompt.asrFallback.modelId` must not be empty when provided"
                    .to_string(),
            );
        }
        if self
            .language
            .as_deref()
            .is_some_and(|language| language.trim().is_empty())
        {
            return Err(
                "invalid request: `referenceVoicePrompt.asrFallback.language` must not be empty when provided"
                    .to_string(),
            );
        }
        Ok(())
    }
}

fn default_reference_prompt_asr_provider() -> String {
    "candle-whisper".to_string()
}

/// Reference Voice Prompt supplied by a package consumer for speaker conditioning.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceVoicePrompt {
    /// Caller-supplied reference audio.
    pub audio: ReferenceVoicePromptAudio,
    /// Optional transcript for the supplied reference audio.
    #[serde(default)]
    pub transcript: Option<String>,
    /// Optional BCP-47-ish language hint for transcript or ASR fallback planning.
    #[serde(default)]
    pub language: Option<String>,
    /// Optional ASR fallback used when the reference transcript is missing.
    #[serde(default)]
    pub asr_fallback: Option<ReferencePromptAsrFallback>,
    /// Optional caller metadata such as source id or preparation notes.
    #[serde(default)]
    pub metadata: serde_json::Value,
}

impl ReferenceVoicePrompt {
    /// Returns whether the prompt includes a non-empty transcript.
    pub fn has_transcript(&self) -> bool {
        self.transcript
            .as_deref()
            .is_some_and(|transcript| !transcript.trim().is_empty())
    }

    /// Validates source, transcript text, language hints, and fallback fields.
    pub fn validate_source_and_hints(&self) -> Result<(), String> {
        self.audio.validate("referenceVoicePrompt.audio")?;
        if self
            .transcript
            .as_deref()
            .is_some_and(|transcript| transcript.trim().is_empty())
        {
            return Err(
                "invalid request: `referenceVoicePrompt.transcript` must not be empty when provided"
                    .to_string(),
            );
        }
        if self
            .language
            .as_deref()
            .is_some_and(|language| language.trim().is_empty())
        {
            return Err(
                "invalid request: `referenceVoicePrompt.language` must not be empty when provided"
                    .to_string(),
            );
        }
        if let Some(fallback) = &self.asr_fallback {
            fallback.validate()?;
        }
        Ok(())
    }

    /// Validates this prompt for synthesis setup.
    pub fn validate(&self) -> Result<(), String> {
        self.validate_source_and_hints()?;
        if !self.has_transcript() && self.asr_fallback.is_none() {
            return Err(
                "setup_error: `referenceVoicePrompt.transcript` is required unless `referenceVoicePrompt.asrFallback` is configured"
                    .to_string(),
            );
        }
        Ok(())
    }
}

/// Native TTS execution device preference used for planning.
///
/// `Auto` is CUDA-preferred when CUDA support is built and a CUDA device is
/// available; otherwise providers should fall back to CPU.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NativeTtsDevicePreference {
    #[default]
    Auto,
    Cpu,
    Cuda,
}

impl NativeTtsDevicePreference {
    /// Stable protocol string used by package surfaces.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Cpu => "cpu",
            Self::Cuda => "cuda",
        }
    }
}

/// Model bundle behavior requested by a package consumer for native TTS planning.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TtsModelBundleSelection {
    /// Optional caller-provided local model bundle path.
    #[serde(default)]
    pub bundle_path: Option<String>,
    /// Whether a later native provider may materialize missing model files.
    #[serde(default)]
    pub auto_download: bool,
    /// Whether planning and execution must use already-cached files only.
    #[serde(default)]
    pub cache_only: bool,
}

impl TtsModelBundleSelection {
    /// Validates model-bundle planning options without touching the filesystem.
    pub fn validate(&self) -> Result<(), String> {
        if self
            .bundle_path
            .as_deref()
            .is_some_and(|path| path.trim().is_empty())
        {
            return Err(
                "invalid request: `provider.modelBundle.bundlePath` must not be empty when provided"
                    .to_string(),
            );
        }
        Ok(())
    }
}

/// Provider selection requested by the package consumer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SpeechSynthesisProviderSelection {
    /// Provider id, for example `generic`, `speaker-conditioned`, `f5`, or a downstream id.
    pub provider_id: String,
    /// Optional model id. Later slices add explicit model preset metadata.
    #[serde(default)]
    pub model_id: Option<String>,
    /// Whether native provider execution is requested.
    #[serde(default)]
    pub native: bool,
    /// Native execution device preference for planning.
    #[serde(default)]
    pub device: NativeTtsDevicePreference,
    /// Model-bundle resolution and download policy for native planning.
    #[serde(default)]
    pub model_bundle: TtsModelBundleSelection,
}

impl Default for SpeechSynthesisProviderSelection {
    fn default() -> Self {
        Self {
            provider_id: "generic".to_string(),
            model_id: None,
            native: false,
            device: NativeTtsDevicePreference::Auto,
            model_bundle: TtsModelBundleSelection::default(),
        }
    }
}

impl SpeechSynthesisProviderSelection {
    /// Validates this provider selection.
    pub fn validate(&self) -> Result<(), String> {
        if self.provider_id.trim().is_empty() {
            return Err("invalid request: `provider.providerId` must not be empty".to_string());
        }
        if self
            .model_id
            .as_deref()
            .is_some_and(|model_id| model_id.trim().is_empty())
        {
            return Err(
                "invalid request: `provider.modelId` must not be empty when provided".to_string(),
            );
        }
        self.model_bundle.validate()?;
        Ok(())
    }
}

/// Inference controls that package consumers can pass through to TTS providers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SpeechSynthesisOptions {
    /// Desired output sample rate.
    #[serde(default = "default_sample_rate_hz")]
    pub sample_rate_hz: u32,
    /// Desired output channel count.
    #[serde(default = "default_channels")]
    pub channels: u16,
    /// Optional deterministic seed.
    #[serde(default)]
    pub seed: Option<u64>,
    /// Optional native sampling step count.
    #[serde(default)]
    pub steps: Option<u32>,
    /// Optional classifier-free guidance strength.
    #[serde(default)]
    pub cfg_strength: Option<f32>,
    /// Optional speech speed multiplier.
    #[serde(default)]
    pub speed: Option<f32>,
    /// Optional maximum generated duration.
    #[serde(default)]
    pub max_duration_seconds: Option<f32>,
    /// Whether downstream native providers should remove generated silence.
    #[serde(default)]
    pub remove_silence: bool,
}

impl Default for SpeechSynthesisOptions {
    fn default() -> Self {
        Self {
            sample_rate_hz: default_sample_rate_hz(),
            channels: default_channels(),
            seed: None,
            steps: None,
            cfg_strength: None,
            speed: None,
            max_duration_seconds: None,
            remove_silence: false,
        }
    }
}

impl SpeechSynthesisOptions {
    /// Validates inference options.
    pub fn validate(&self) -> Result<(), String> {
        if self.sample_rate_hz == 0 {
            return Err(
                "invalid request: `options.sampleRateHz` must be greater than zero".to_string(),
            );
        }
        if self.channels == 0 {
            return Err(
                "invalid request: `options.channels` must be greater than zero".to_string(),
            );
        }
        if self.steps.is_some_and(|steps| steps == 0) {
            return Err(
                "invalid request: `options.steps` must be greater than zero when provided"
                    .to_string(),
            );
        }
        for (field, value) in [
            ("options.cfgStrength", self.cfg_strength),
            ("options.speed", self.speed),
            ("options.maxDurationSeconds", self.max_duration_seconds),
        ] {
            if value.is_some_and(|value| !value.is_finite() || value <= 0.0) {
                return Err(format!(
                    "invalid request: `{field}` must be finite and greater than zero when provided"
                ));
            }
        }
        Ok(())
    }
}

fn default_sample_rate_hz() -> u32 {
    24_000
}

fn default_channels() -> u16 {
    1
}

/// Text-to-speech synthesis request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SpeechSynthesisRequest {
    /// Target text to synthesize.
    pub text: String,
    /// Optional Reference Voice Prompt for speaker-conditioned synthesis.
    #[serde(default)]
    pub reference_voice_prompt: Option<ReferenceVoicePrompt>,
    /// Provider selection.
    #[serde(default)]
    pub provider: SpeechSynthesisProviderSelection,
    /// Inference options.
    #[serde(default)]
    pub options: SpeechSynthesisOptions,
}

impl SpeechSynthesisRequest {
    /// Validates the request through the public contract.
    pub fn validate(&self) -> Result<(), String> {
        if self.text.trim().is_empty() {
            return Err("invalid request: `text` must not be empty".to_string());
        }
        self.provider.validate()?;
        self.options.validate()?;
        if let Some(prompt) = &self.reference_voice_prompt {
            prompt.validate()?;
        }
        Ok(())
    }

    /// Returns whether the request is speaker-conditioned.
    pub fn is_speaker_conditioned(&self) -> bool {
        self.reference_voice_prompt.is_some()
            || self.provider.provider_id == "speaker-conditioned"
            || self.provider.provider_id == "f5"
            || self.provider.provider_id == "e2"
    }
}

/// Status returned by synthesis operations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SpeechSynthesisStatus {
    Ready,
    SetupRequired,
    UnsupportedRuntime,
}

/// A single setup or runtime diagnostic.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SpeechSynthesisDiagnostic {
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub help: Option<String>,
}

/// Result of a synthesis request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SpeechSynthesisOutput {
    pub status: SpeechSynthesisStatus,
    pub provider: SpeechSynthesisProviderSelection,
    #[serde(default)]
    pub audio: Option<PcmAudio>,
    pub diagnostics: Vec<SpeechSynthesisDiagnostic>,
}

/// Trait implemented by concrete TTS providers.
pub trait SpeechSynthesisProvider {
    /// Stable provider id.
    fn provider_id(&self) -> &'static str;

    /// Validates and synthesizes speech or returns explicit setup diagnostics.
    fn synthesize(&self, request: &SpeechSynthesisRequest)
        -> Result<SpeechSynthesisOutput, String>;
}

/// Side-effect-free provider used until native providers are implemented.
#[derive(Debug, Clone, Copy, Default)]
pub struct UnsupportedSpeechSynthesisProvider;

impl SpeechSynthesisProvider for UnsupportedSpeechSynthesisProvider {
    fn provider_id(&self) -> &'static str {
        "unsupported-runtime"
    }

    fn synthesize(
        &self,
        request: &SpeechSynthesisRequest,
    ) -> Result<SpeechSynthesisOutput, String> {
        request.validate()?;
        let status = if request.provider.native || reference_prompt_asr_unavailable(request) {
            SpeechSynthesisStatus::SetupRequired
        } else {
            SpeechSynthesisStatus::UnsupportedRuntime
        };
        Ok(SpeechSynthesisOutput {
            status,
            provider: request.provider.clone(),
            audio: None,
            diagnostics: unsupported_provider_diagnostics(request),
        })
    }
}

/// Validates a request and returns the current default synthesis response.
pub fn synthesize(request: &SpeechSynthesisRequest) -> Result<SpeechSynthesisOutput, String> {
    UnsupportedSpeechSynthesisProvider.synthesize(request)
}

fn unsupported_provider_diagnostics(
    request: &SpeechSynthesisRequest,
) -> Vec<SpeechSynthesisDiagnostic> {
    let mut diagnostics = vec![SpeechSynthesisDiagnostic {
        code: "tts_provider_not_available".to_string(),
        message: "No native TTS provider is available in this build.".to_string(),
        help: Some(
            "Later slices add explicit model presets and native provider setup.".to_string(),
        ),
    }];
    if request.is_speaker_conditioned() && request.reference_voice_prompt.is_none() {
        diagnostics.push(SpeechSynthesisDiagnostic {
            code: "reference_voice_prompt_missing".to_string(),
            message: "Speaker-conditioned TTS requires a Reference Voice Prompt.".to_string(),
            help: Some(
                "Provide referenceVoicePrompt audio and, when available, a transcript.".to_string(),
            ),
        });
    }
    if reference_prompt_asr_unavailable(request) {
        diagnostics.push(SpeechSynthesisDiagnostic {
            code: "reference_prompt_asr_unavailable".to_string(),
            message: "Reference Voice Prompt ASR fallback is configured but unavailable in this build."
                .to_string(),
            help: Some(
                "Build audio-generation-tts with the `asr` feature to plan fallback through audio-analysis-transcription."
                    .to_string(),
            ),
        });
    }
    diagnostics
}

fn reference_prompt_asr_unavailable(request: &SpeechSynthesisRequest) -> bool {
    request
        .reference_voice_prompt
        .as_ref()
        .is_some_and(|prompt| {
            !prompt.has_transcript()
                && prompt.asr_fallback.is_some()
                && !reference_prompt_asr_available()
        })
}

fn reference_prompt_asr_available() -> bool {
    cfg!(feature = "asr")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthesis_rejects_empty_text() {
        let request = SpeechSynthesisRequest {
            text: String::new(),
            reference_voice_prompt: None,
            provider: SpeechSynthesisProviderSelection::default(),
            options: SpeechSynthesisOptions::default(),
        };

        let error = synthesize(&request).expect_err("empty text");
        assert!(error.contains("text"));
    }

    #[test]
    fn synthesis_returns_explicit_unsupported_runtime_without_audio() {
        let request = SpeechSynthesisRequest {
            text: "Hello from TTS".to_string(),
            reference_voice_prompt: None,
            provider: SpeechSynthesisProviderSelection::default(),
            options: SpeechSynthesisOptions::default(),
        };

        let output = synthesize(&request).expect("unsupported provider response");
        assert_eq!(output.status, SpeechSynthesisStatus::UnsupportedRuntime);
        assert!(output.audio.is_none());
        assert!(output
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "tts_provider_not_available"));
    }

    #[test]
    fn speaker_conditioned_synthesis_requires_transcript_without_asr_fallback() {
        let request = SpeechSynthesisRequest {
            text: "Match this speaker.".to_string(),
            reference_voice_prompt: Some(ReferenceVoicePrompt {
                audio: ReferenceVoicePromptAudio::Samples(PcmAudio {
                    sample_rate_hz: 24_000,
                    channels: 1,
                    samples: vec![0.0, 0.01, -0.01, 0.0],
                }),
                transcript: None,
                language: Some("en".to_string()),
                asr_fallback: None,
                metadata: serde_json::json!({}),
            }),
            provider: SpeechSynthesisProviderSelection {
                provider_id: "speaker-conditioned".to_string(),
                ..SpeechSynthesisProviderSelection::default()
            },
            options: SpeechSynthesisOptions::default(),
        };

        let error = synthesize(&request).expect_err("missing transcript");
        assert!(error.contains("setup_error"));
        assert!(error.contains("referenceVoicePrompt.transcript"));
        assert!(error.contains("referenceVoicePrompt.asrFallback"));
    }

    #[test]
    fn speaker_conditioned_synthesis_reports_asr_unavailable_when_fallback_is_configured() {
        let request = SpeechSynthesisRequest {
            text: "Match this speaker.".to_string(),
            reference_voice_prompt: Some(ReferenceVoicePrompt {
                audio: ReferenceVoicePromptAudio::Samples(PcmAudio {
                    sample_rate_hz: 24_000,
                    channels: 1,
                    samples: vec![0.0, 0.01, -0.01, 0.0],
                }),
                transcript: None,
                language: Some("en".to_string()),
                asr_fallback: Some(ReferencePromptAsrFallback {
                    provider_id: "candle-whisper".to_string(),
                    model_id: Some("openai/whisper-large-v3-turbo".to_string()),
                    language: None,
                }),
                metadata: serde_json::json!({}),
            }),
            provider: SpeechSynthesisProviderSelection {
                provider_id: "speaker-conditioned".to_string(),
                ..SpeechSynthesisProviderSelection::default()
            },
            options: SpeechSynthesisOptions::default(),
        };

        let output = synthesize(&request).expect("asr fallback setup response");
        assert_eq!(output.status, SpeechSynthesisStatus::SetupRequired);
        assert!(output.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "reference_prompt_asr_unavailable"
                && diagnostic.message.contains("ASR fallback")
        }));
    }

    #[test]
    fn provider_selection_deserializes_native_device_preferences() {
        for (device, expected) in [
            ("auto", NativeTtsDevicePreference::Auto),
            ("cpu", NativeTtsDevicePreference::Cpu),
            ("cuda", NativeTtsDevicePreference::Cuda),
        ] {
            let selection: SpeechSynthesisProviderSelection = serde_json::from_value(
                serde_json::json!({"providerId":"f5","native":true,"device": device}),
            )
            .expect("provider selection");

            assert_eq!(selection.device, expected);
        }
    }
}
