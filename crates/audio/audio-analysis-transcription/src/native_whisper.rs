use std::path::{Path, PathBuf};

use candle_core::{DType, Device, IndexOp, Tensor, D};
use candle_nn::VarBuilder;
use candle_transformers::models::whisper::{self, model::Whisper};
use serde::Deserialize;
use text_transcripts::{TranscriptSegmentContract, TranscriptionContract};
use tokenizers::Tokenizer;
use video_analysis_core::Result;

use crate::native_device::{resolve_native_device, ResolvedNativeDevice};
use crate::{
    invalid_request, model_output_mismatch, setup_error, validate_asr_request, AsrRequest,
    AsrResponse, CandleWhisperOptions, SpeechActivitySegment,
};

#[derive(Debug, Clone)]
pub(crate) struct WhisperBundlePaths {
    pub root: PathBuf,
    pub config_json: PathBuf,
    pub generation_config_json: PathBuf,
    pub tokenizer_json: PathBuf,
    pub preprocessor_config_json: PathBuf,
    pub model_safetensors: PathBuf,
}

#[derive(Debug, Clone)]
struct WhisperRunSetup {
    model_id: String,
    language: Option<String>,
    bundle: WhisperBundlePaths,
    resolved_device: ResolvedNativeDevice,
}

#[derive(Debug, Clone, Deserialize)]
struct GenerationConfig {
    #[serde(default)]
    decoder_start_token_id: Option<u32>,
    #[serde(default)]
    eos_token_id: Option<u32>,
    #[serde(default)]
    forced_decoder_ids: Option<Vec<(usize, Option<u32>)>>,
    #[serde(default)]
    max_length: Option<usize>,
    #[serde(default)]
    lang_to_id: std::collections::BTreeMap<String, u32>,
    #[serde(default)]
    task_to_id: std::collections::BTreeMap<String, u32>,
}

pub(crate) fn transcribe(
    options: &CandleWhisperOptions,
    request: AsrRequest,
) -> Result<AsrResponse> {
    let setup = WhisperRunSetup::from_options_and_request(options, &request)?;
    let mut session = CandleWhisperSession::load(setup)?;
    session.transcribe_chunks(request)
}

impl WhisperRunSetup {
    fn from_options_and_request(
        options: &CandleWhisperOptions,
        request: &AsrRequest,
    ) -> Result<Self> {
        validate_asr_request(request)?;
        let bundle = options
            .model_bundle
            .as_ref()
            .ok_or_else(|| setup_error("required Candle Whisper model bundle is missing"))
            .and_then(|bundle| resolve_whisper_bundle_paths(bundle))?;
        let resolved_device = resolve_native_device(options.device)?;
        Ok(Self {
            model_id: request.model_id.clone(),
            language: request
                .language
                .clone()
                .or_else(|| options.language.clone()),
            bundle,
            resolved_device,
        })
    }
}

pub(crate) fn resolve_whisper_bundle_paths(bundle: &Path) -> Result<WhisperBundlePaths> {
    if !bundle.exists() {
        return Err(setup_error(format!(
            "required Candle Whisper model bundle `{}` is missing",
            bundle.display()
        )));
    }
    Ok(WhisperBundlePaths {
        root: bundle.to_path_buf(),
        config_json: crate::native_bundles::resolve_required_bundle_file(bundle, "config.json")?,
        generation_config_json: crate::native_bundles::resolve_required_bundle_file(
            bundle,
            "generation_config.json",
        )?,
        tokenizer_json: crate::native_bundles::resolve_required_bundle_file(
            bundle,
            "tokenizer.json",
        )?,
        preprocessor_config_json: crate::native_bundles::resolve_required_bundle_file(
            bundle,
            "preprocessor_config.json",
        )?,
        model_safetensors: crate::native_bundles::resolve_required_bundle_file(
            bundle,
            "model.safetensors",
        )?,
    })
}

struct CandleWhisperSession {
    setup: WhisperRunSetup,
    device: Device,
    model: Whisper,
    tokenizer: Tokenizer,
    generation: GenerationConfig,
    mel_filters: Vec<f32>,
}

impl CandleWhisperSession {
    fn load(setup: WhisperRunSetup) -> Result<Self> {
        let device = candle_device(&setup.resolved_device)?;
        let config: whisper::Config = read_json(&setup.bundle.config_json, "config.json")?;
        let generation: GenerationConfig = read_json(
            &setup.bundle.generation_config_json,
            "generation_config.json",
        )?;
        let _preprocessor: serde_json::Value = read_json(
            &setup.bundle.preprocessor_config_json,
            "preprocessor_config.json",
        )?;
        let tokenizer = Tokenizer::from_file(&setup.bundle.tokenizer_json).map_err(|error| {
            invalid_request(format!(
                "failed to load tokenizer `{}`: {error}",
                setup.bundle.tokenizer_json.display()
            ))
        })?;
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(
                &[setup.bundle.model_safetensors.as_path()],
                whisper::DTYPE,
                &device,
            )
        }
        .map_err(|error| {
            setup_error(format!(
                "failed to load Candle Whisper weights `{}`: {error}",
                setup.bundle.model_safetensors.display()
            ))
        })?;
        let model = Whisper::load(&vb, config.clone()).map_err(|error| {
            setup_error(format!(
                "failed to construct Candle Whisper model from `{}`: {error}",
                setup.bundle.root.display()
            ))
        })?;
        let mel_filters =
            mel_filter_bank(config.num_mel_bins, whisper::N_FFT, whisper::SAMPLE_RATE);
        Ok(Self {
            setup,
            device,
            model,
            tokenizer,
            generation,
            mel_filters,
        })
    }

    fn transcribe_chunks(&mut self, request: AsrRequest) -> Result<AsrResponse> {
        let mut segments = Vec::new();
        let mut next_index = 0_u64;
        for chunk in &request.chunks {
            for window in chunk_windows(&request.audio.samples, request.audio.sample_rate, chunk)? {
                let text = self.decode_window(&window.samples)?;
                let mut segment = TranscriptSegmentContract::new(next_index, text);
                segment.start_seconds = Some(window.local_start_seconds);
                segment.end_seconds = Some(window.local_end_seconds);
                segment.language = self.setup.language.clone();
                segment
                    .attributes
                    .insert("provider".to_string(), "candle-whisper".to_string());
                segment
                    .attributes
                    .insert("timing".to_string(), "chunkLocal".to_string());
                segments.push(segment);
                next_index += 1;
            }
        }
        let transcript = TranscriptionContract::from_segments(
            request.audio.source,
            request.language.clone(),
            segments,
        )
        .map_err(|error| model_output_mismatch(error.to_string()))?;
        let device_label = device_label(&self.setup.resolved_device);
        let mut diagnostics = vec![
            "provider=candle-whisper".to_string(),
            format!("device={device_label}"),
            format!("modelId={}", self.setup.model_id),
            format!("bundle={}", self.setup.bundle.root.display()),
            format!("chunkCount={}", request.chunks.len()),
            format!("cuda={}", device_is_cuda(&self.setup.resolved_device)),
            "timing=chunk/window".to_string(),
        ];
        if let Some(language) = &self.setup.language {
            diagnostics.push(format!("language={language}"));
        }
        Ok(AsrResponse {
            model_id: request.model_id,
            language: self.setup.language.clone(),
            transcript,
            diagnostics,
        })
    }

    fn decode_window(&mut self, samples: &[f32]) -> Result<String> {
        let mel = whisper::audio::pcm_to_mel(&self.model.config, samples, &self.mel_filters);
        let n_mel = self.model.config.num_mel_bins;
        let mel_frames = mel.len() / n_mel;
        let mut features = Vec::with_capacity(n_mel * whisper::N_FRAMES);
        for mel_index in 0..n_mel {
            let row_start = mel_index * mel_frames;
            let available = mel_frames.min(whisper::N_FRAMES);
            features.extend_from_slice(&mel[row_start..row_start + available]);
            if available < whisper::N_FRAMES {
                features.extend(std::iter::repeat_n(0.0, whisper::N_FRAMES - available));
            }
        }
        let mel = Tensor::from_vec(features, (1, n_mel, whisper::N_FRAMES), &self.device).map_err(
            |error| model_output_mismatch(format!("failed to build mel tensor: {error}")),
        )?;
        let audio_features =
            self.model.encoder.forward(&mel, true).map_err(|error| {
                model_output_mismatch(format!("Whisper encoder failed: {error}"))
            })?;
        let token_ids = self.decode_tokens(&audio_features)?;
        self.tokenizer
            .decode(&token_ids, true)
            .map(|text| text.trim().to_string())
            .map_err(|error| {
                model_output_mismatch(format!("failed to decode Whisper tokens: {error}"))
            })
            .map(|text| text.replace("  ", " "))
            .map(|text| text.trim().to_string())
    }

    fn decode_tokens(&mut self, audio_features: &Tensor) -> Result<Vec<u32>> {
        let mut tokens = self.initial_tokens()?;
        let eos = self.eos_token_id()?;
        let prompt_len = tokens.len();
        let max_length = self
            .generation
            .max_length
            .unwrap_or(self.model.config.max_target_positions)
            .min(self.model.config.max_target_positions);
        while tokens.len() < max_length {
            let token_tensor = Tensor::new(tokens.as_slice(), &self.device)
                .and_then(|tensor| tensor.unsqueeze(0))
                .map_err(|error| {
                    model_output_mismatch(format!("failed to build token tensor: {error}"))
                })?;
            let decoded = self
                .model
                .decoder
                .forward(&token_tensor, audio_features, true)
                .map_err(|error| {
                    model_output_mismatch(format!("Whisper decoder failed: {error}"))
                })?;
            let logits = self.model.decoder.final_linear(&decoded).map_err(|error| {
                model_output_mismatch(format!("Whisper logits projection failed: {error}"))
            })?;
            let seq_index = tokens.len() - 1;
            let next = logits
                .i((0, seq_index, ..))
                .and_then(|logits| logits.to_dtype(DType::F32))
                .and_then(|logits| logits.argmax(D::Minus1))
                .and_then(|token| token.to_scalar::<u32>())
                .map_err(|error| {
                    model_output_mismatch(format!("Whisper greedy decode failed: {error}"))
                })?;
            if next == eos {
                break;
            }
            tokens.push(next);
        }
        Ok(tokens.into_iter().skip(prompt_len).collect())
    }

    fn initial_tokens(&self) -> Result<Vec<u32>> {
        Self::initial_prompt_tokens(
            &self.generation,
            &self.tokenizer,
            self.setup.language.as_deref(),
        )
    }

    fn initial_prompt_tokens(
        generation: &GenerationConfig,
        tokenizer: &Tokenizer,
        language: Option<&str>,
    ) -> Result<Vec<u32>> {
        let decoder_start = Self::decoder_start_token_id(generation, tokenizer)?;
        let mut tokens = vec![decoder_start];
        if let Some(language) = language {
            let token = Self::language_token_id(generation, tokenizer, language).ok_or_else(|| {
                invalid_request(format!(
                    "Whisper generation config/tokenizer does not define language token `{language}`"
                ))
            })?;
            tokens.push(token);
        }
        let transcribe =
            Self::task_token_id(generation, tokenizer, "transcribe").ok_or_else(|| {
                invalid_request(
                    "Whisper generation config/tokenizer is missing transcribe task token",
                )
            })?;
        tokens.push(transcribe);
        let no_timestamps = token_id(tokenizer, whisper::NO_TIMESTAMPS_TOKEN)
            .ok_or_else(|| invalid_request("Whisper tokenizer is missing no-timestamps token"))?;
        tokens.push(no_timestamps);
        if let Some(forced) = &generation.forced_decoder_ids {
            for (position, token) in forced {
                let Some(token) = token else {
                    continue;
                };
                if *position < tokens.len() {
                    tokens[*position] = *token;
                } else {
                    while tokens.len() < *position {
                        tokens.push(decoder_start);
                    }
                    tokens.push(*token);
                }
            }
        }
        Ok(tokens)
    }

    fn decoder_start_token_id(generation: &GenerationConfig, tokenizer: &Tokenizer) -> Result<u32> {
        generation
            .decoder_start_token_id
            .or_else(|| token_id(tokenizer, whisper::SOT_TOKEN))
            .ok_or_else(|| {
                invalid_request("Whisper generation config is missing decoder_start_token_id")
            })
    }

    fn eos_token_id(&self) -> Result<u32> {
        Self::resolve_eos_token_id(&self.generation, &self.tokenizer)
    }

    fn resolve_eos_token_id(generation: &GenerationConfig, tokenizer: &Tokenizer) -> Result<u32> {
        generation
            .eos_token_id
            .or_else(|| token_id(tokenizer, whisper::EOT_TOKEN))
            .ok_or_else(|| invalid_request("Whisper generation config is missing eos_token_id"))
    }

    fn language_token_id(
        generation: &GenerationConfig,
        tokenizer: &Tokenizer,
        language: &str,
    ) -> Option<u32> {
        let normalized = language.trim().to_lowercase();
        let wrapped = format!("<|{normalized}|>");
        generation
            .lang_to_id
            .get(&wrapped)
            .or_else(|| generation.lang_to_id.get(&normalized))
            .copied()
            .or_else(|| token_id(tokenizer, &wrapped))
    }

    fn task_token_id(
        generation: &GenerationConfig,
        tokenizer: &Tokenizer,
        task: &str,
    ) -> Option<u32> {
        let wrapped = format!("<|{task}|>");
        generation
            .task_to_id
            .get(&wrapped)
            .or_else(|| generation.task_to_id.get(task))
            .copied()
            .or_else(|| token_id(tokenizer, &wrapped))
    }
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path, label: &str) -> Result<T> {
    let bytes = std::fs::read(path).map_err(|error| {
        setup_error(format!(
            "failed to read {label} `{}`: {error}",
            path.display()
        ))
    })?;
    serde_json::from_slice(&bytes).map_err(|error| {
        invalid_request(format!(
            "failed to parse {label} `{}`: {error}",
            path.display()
        ))
    })
}

fn candle_device(resolved: &ResolvedNativeDevice) -> Result<Device> {
    match resolved {
        ResolvedNativeDevice::Cpu => Ok(Device::Cpu),
        #[cfg(feature = "cuda")]
        ResolvedNativeDevice::Cuda(index) => Device::new_cuda(*index)
            .map_err(|error| setup_error(format!("failed to create CUDA device {index}: {error}"))),
    }
}

fn device_label(resolved: &ResolvedNativeDevice) -> String {
    match resolved {
        ResolvedNativeDevice::Cpu => "cpu".to_string(),
        #[cfg(feature = "cuda")]
        ResolvedNativeDevice::Cuda(index) => format!("cuda:{index}"),
    }
}

fn device_is_cuda(resolved: &ResolvedNativeDevice) -> bool {
    match resolved {
        ResolvedNativeDevice::Cpu => false,
        #[cfg(feature = "cuda")]
        ResolvedNativeDevice::Cuda(_) => true,
    }
}

#[derive(Debug, Clone)]
struct ChunkWindow {
    samples: Vec<f32>,
    local_start_seconds: f64,
    local_end_seconds: f64,
}

fn chunk_windows(
    samples: &[f32],
    sample_rate: u32,
    chunk: &SpeechActivitySegment,
) -> Result<Vec<ChunkWindow>> {
    let start = seconds_to_index(chunk.start_seconds, sample_rate, samples.len());
    let end = seconds_to_index(chunk.end_seconds, sample_rate, samples.len()).max(start + 1);
    let max_window = whisper::N_SAMPLES;
    let mut windows = Vec::new();
    let mut cursor = start;
    while cursor < end {
        let window_end = (cursor + max_window).min(end);
        windows.push(ChunkWindow {
            samples: samples[cursor..window_end].to_vec(),
            local_start_seconds: (cursor - start) as f64 / sample_rate as f64,
            local_end_seconds: (window_end - start) as f64 / sample_rate as f64,
        });
        cursor = window_end;
    }
    Ok(windows)
}

fn seconds_to_index(seconds: f64, sample_rate: u32, limit: usize) -> usize {
    (seconds * sample_rate as f64)
        .round()
        .clamp(0.0, limit as f64) as usize
}

fn token_id(tokenizer: &Tokenizer, token: &str) -> Option<u32> {
    tokenizer.token_to_id(token)
}

fn mel_filter_bank(n_mels: usize, n_fft: usize, sample_rate: usize) -> Vec<f32> {
    let n_freqs = n_fft / 2 + 1;
    let min_mel = hz_to_mel(0.0);
    let max_mel = hz_to_mel(sample_rate as f32 / 2.0);
    let mel_points = (0..n_mels + 2)
        .map(|index| min_mel + (max_mel - min_mel) * index as f32 / (n_mels + 1) as f32)
        .map(mel_to_hz)
        .collect::<Vec<_>>();
    let fft_freqs = (0..n_freqs)
        .map(|index| sample_rate as f32 * index as f32 / n_fft as f32)
        .collect::<Vec<_>>();
    let mut filters = vec![0.0; n_mels * n_freqs];
    for mel_index in 0..n_mels {
        let lower = mel_points[mel_index];
        let center = mel_points[mel_index + 1];
        let upper = mel_points[mel_index + 2];
        for (freq_index, freq) in fft_freqs.iter().enumerate() {
            let value = if *freq < lower || *freq > upper {
                0.0
            } else if *freq <= center {
                (*freq - lower) / (center - lower).max(f32::EPSILON)
            } else {
                (upper - *freq) / (upper - center).max(f32::EPSILON)
            };
            filters[mel_index * n_freqs + freq_index] = value.max(0.0);
        }
    }
    filters
}

fn hz_to_mel(hz: f32) -> f32 {
    2595.0 * (1.0 + hz / 700.0).log10()
}

fn mel_to_hz(mel: f32) -> f32 {
    700.0 * (10_f32.powf(mel / 2595.0) - 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokenizers::models::wordlevel::WordLevel;

    fn test_generation() -> GenerationConfig {
        GenerationConfig {
            decoder_start_token_id: Some(1),
            eos_token_id: Some(2),
            forced_decoder_ids: None,
            max_length: Some(8),
            lang_to_id: [("<|en|>".to_string(), 3), ("<|de|>".to_string(), 4)]
                .into_iter()
                .collect(),
            task_to_id: [("transcribe".to_string(), 5), ("translate".to_string(), 6)]
                .into_iter()
                .collect(),
        }
    }

    fn test_tokenizer() -> Tokenizer {
        let temp = tempfile::tempdir().unwrap();
        let vocab = temp.path().join("vocab.json");
        std::fs::write(
            &vocab,
            serde_json::json!({
                "<unk>": 0,
                whisper::SOT_TOKEN: 1,
                whisper::EOT_TOKEN: 2,
                "<|en|>": 3,
                "<|de|>": 4,
                "<|transcribe|>": 5,
                "<|translate|>": 6,
                whisper::NO_TIMESTAMPS_TOKEN: 7
            })
            .to_string(),
        )
        .unwrap();
        let model = WordLevel::from_file(vocab.to_str().unwrap(), "<unk>".to_string()).unwrap();
        Tokenizer::new(model)
    }

    #[test]
    fn initial_prompt_uses_requested_language_and_transcribe_task() {
        let tokens = CandleWhisperSession::initial_prompt_tokens(
            &test_generation(),
            &test_tokenizer(),
            Some("en"),
        )
        .unwrap();
        assert_eq!(tokens, vec![1, 3, 5, 7]);
        assert!(!tokens.contains(&6));
    }

    #[test]
    fn initial_prompt_uses_option_language_when_request_language_absent() {
        let setup = WhisperRunSetup {
            model_id: "openai/whisper-tiny".to_string(),
            language: Some("de".to_string()),
            bundle: WhisperBundlePaths {
                root: PathBuf::from("bundle"),
                config_json: PathBuf::from("config.json"),
                generation_config_json: PathBuf::from("generation_config.json"),
                tokenizer_json: PathBuf::from("tokenizer.json"),
                preprocessor_config_json: PathBuf::from("preprocessor_config.json"),
                model_safetensors: PathBuf::from("model.safetensors"),
            },
            resolved_device: ResolvedNativeDevice::Cpu,
        };
        let tokens = CandleWhisperSession::initial_prompt_tokens(
            &test_generation(),
            &test_tokenizer(),
            setup.language.as_deref(),
        )
        .unwrap();
        assert_eq!(tokens[1], 4);
    }

    #[test]
    fn request_language_wins_before_prompt_construction() {
        let request = AsrRequest {
            audio: crate::LoadedAudio {
                samples: vec![0.0; 16_000],
                sample_rate: 16_000,
                channels: 1,
                source: None,
            },
            chunks: vec![SpeechActivitySegment::new(0.0, 1.0, 0.5).unwrap()],
            language: Some("en".to_string()),
            model_id: "openai/whisper-tiny".to_string(),
        };
        let options = CandleWhisperOptions {
            language: Some("de".to_string()),
            model_bundle: Some(PathBuf::from("missing")),
            ..CandleWhisperOptions::default()
        };
        let language = request
            .language
            .clone()
            .or_else(|| options.language.clone())
            .unwrap();
        assert_eq!(language, "en");
    }

    #[test]
    fn nullable_forced_decoder_ids_are_skipped() {
        let mut generation = test_generation();
        generation.forced_decoder_ids = Some(vec![(1, None), (2, Some(5)), (3, Some(7))]);
        let tokens =
            CandleWhisperSession::initial_prompt_tokens(&generation, &test_tokenizer(), Some("en"))
                .unwrap();
        assert_eq!(tokens, vec![1, 3, 5, 7]);
    }

    #[test]
    fn unknown_explicit_language_returns_invalid_request() {
        let error = CandleWhisperSession::initial_prompt_tokens(
            &test_generation(),
            &test_tokenizer(),
            Some("zz"),
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("invalid_request"));
        assert!(error.contains("language token"));
    }

    #[test]
    fn missing_eos_returns_invalid_request() {
        let mut generation = test_generation();
        generation.eos_token_id = None;
        let tokenizer = test_tokenizer();
        assert!(CandleWhisperSession::resolve_eos_token_id(&generation, &tokenizer).is_ok());

        let temp = tempfile::tempdir().unwrap();
        let vocab = temp.path().join("vocab.json");
        std::fs::write(
            &vocab,
            serde_json::json!({
                "<unk>": 0,
                whisper::SOT_TOKEN: 1,
                "<|en|>": 3,
                "<|transcribe|>": 5,
                whisper::NO_TIMESTAMPS_TOKEN: 7
            })
            .to_string(),
        )
        .unwrap();
        let tokenizer = Tokenizer::new(
            WordLevel::from_file(vocab.to_str().unwrap(), "<unk>".to_string()).unwrap(),
        );
        let error = CandleWhisperSession::resolve_eos_token_id(&generation, &tokenizer)
            .unwrap_err()
            .to_string();
        assert!(error.contains("invalid_request"));
        assert!(error.contains("eos_token_id"));
    }

    #[test]
    fn whisper_bundle_resolution_accepts_direct_and_files_layouts() {
        for nested in [false, true] {
            let temp = tempfile::tempdir().unwrap();
            let root = temp.path();
            let file_root = if nested {
                std::fs::create_dir(root.join("files")).unwrap();
                root.join("files")
            } else {
                root.to_path_buf()
            };
            for file in [
                "config.json",
                "generation_config.json",
                "tokenizer.json",
                "preprocessor_config.json",
                "model.safetensors",
            ] {
                std::fs::write(file_root.join(file), "").unwrap();
            }
            let paths = resolve_whisper_bundle_paths(root).unwrap();
            assert!(paths.config_json.exists());
            assert!(paths.generation_config_json.exists());
            assert!(paths.tokenizer_json.exists());
            assert!(paths.preprocessor_config_json.exists());
            assert!(paths.model_safetensors.exists());
        }
    }

    #[cfg(feature = "model-bundles")]
    #[test]
    fn whisper_bundle_resolution_accepts_manifest_layout() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        std::fs::create_dir(root.join("snapshots")).unwrap();
        for file in [
            "config.json",
            "generation_config.json",
            "tokenizer.json",
            "preprocessor_config.json",
            "model.safetensors",
        ] {
            std::fs::write(root.join("snapshots").join(file), "").unwrap();
        }
        std::fs::write(
            root.join("manifest.json"),
            serde_json::json!({
                "schema_version": 1,
                "name": "whisper-test",
                "repo_id": "openai/whisper-tiny",
                "revision": "main",
                "task": "speech_recognition",
                "files": {
                    "config.json": {"remote_path": "config.json", "local_path": "snapshots/config.json", "size_bytes": 0},
                    "generation_config.json": {"remote_path": "generation_config.json", "local_path": "snapshots/generation_config.json", "size_bytes": 0},
                    "tokenizer.json": {"remote_path": "tokenizer.json", "local_path": "snapshots/tokenizer.json", "size_bytes": 0},
                    "preprocessor_config.json": {"remote_path": "preprocessor_config.json", "local_path": "snapshots/preprocessor_config.json", "size_bytes": 0},
                    "model.safetensors": {"remote_path": "model.safetensors", "local_path": "snapshots/model.safetensors", "size_bytes": 0}
                }
            })
            .to_string(),
        )
        .unwrap();
        let paths = resolve_whisper_bundle_paths(root).unwrap();
        assert_eq!(
            paths.model_safetensors,
            root.join("snapshots/model.safetensors")
        );
    }
}
