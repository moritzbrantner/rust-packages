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

const WHISPER_TIMESTAMP_SECONDS_PER_TOKEN: f64 = 0.02;
const WHISPER_TIMESTAMP_TOKEN_COUNT: u32 =
    (whisper::CHUNK_LENGTH as f64 / WHISPER_TIMESTAMP_SECONDS_PER_TOKEN) as u32 + 1;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WhisperDecodeMode {
    WithoutTimestamps,
    TimestampTokens,
}

#[derive(Debug, Clone, PartialEq)]
struct WhisperTimestampSpec {
    begin_token_id: u32,
    end_token_id: u32,
    seconds_per_token: f64,
}

#[derive(Debug, Clone, PartialEq)]
struct WhisperDecodedWindow {
    text: String,
    segments: Vec<WhisperDecodedSegment>,
}

#[derive(Debug, Clone, PartialEq)]
struct WhisperDecodedSegment {
    text: String,
    start_seconds: f64,
    end_seconds: f64,
    token_ids: Vec<u32>,
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
        let mut used_timestamp_tokens = false;
        for chunk in &request.chunks {
            for window in chunk_windows(&request.audio.samples, request.audio.sample_rate, chunk)? {
                let decoded =
                    self.decode_window(&window.samples, WhisperDecodeMode::WithoutTimestamps)?;
                if decoded.segments.is_empty() {
                    if decoded.text.trim().is_empty() {
                        continue;
                    }
                    segments.push(window_fallback_segment(
                        next_index,
                        decoded.text,
                        window.local_start_seconds,
                        window.local_end_seconds,
                        self.setup.language.clone(),
                    ));
                    next_index += 1;
                } else {
                    used_timestamp_tokens = true;
                    segments.extend(decoded_window_to_contract_segments(
                        decoded,
                        &mut next_index,
                        window.local_start_seconds,
                        window.local_end_seconds,
                        self.setup.language.clone(),
                    ));
                }
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
            if used_timestamp_tokens {
                "timing=whisperTimestampTokens".to_string()
            } else {
                "timing=chunk/window".to_string()
            },
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

    fn decode_window(
        &mut self,
        samples: &[f32],
        mode: WhisperDecodeMode,
    ) -> Result<WhisperDecodedWindow> {
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
        let token_ids = self.decode_tokens(&audio_features, mode)?;
        match mode {
            WhisperDecodeMode::WithoutTimestamps => Ok(WhisperDecodedWindow {
                text: decode_text_tokens(&self.tokenizer, &token_ids)?,
                segments: Vec::new(),
            }),
            WhisperDecodeMode::TimestampTokens => {
                decode_timestamp_window(&self.tokenizer, &token_ids)?
                    .map(Ok)
                    .unwrap_or_else(|| {
                        Ok(WhisperDecodedWindow {
                            text: decode_text_tokens(&self.tokenizer, &token_ids)?,
                            segments: Vec::new(),
                        })
                    })
            }
        }
    }

    fn decode_tokens(
        &mut self,
        audio_features: &Tensor,
        mode: WhisperDecodeMode,
    ) -> Result<Vec<u32>> {
        let mut tokens = self.initial_tokens(mode)?;
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

    fn initial_tokens(&self, mode: WhisperDecodeMode) -> Result<Vec<u32>> {
        Self::initial_prompt_tokens_for_mode(
            &self.generation,
            &self.tokenizer,
            self.setup.language.as_deref(),
            mode,
        )
    }

    #[cfg(test)]
    fn initial_prompt_tokens(
        generation: &GenerationConfig,
        tokenizer: &Tokenizer,
        language: Option<&str>,
    ) -> Result<Vec<u32>> {
        Self::initial_prompt_tokens_for_mode(
            generation,
            tokenizer,
            language,
            WhisperDecodeMode::WithoutTimestamps,
        )
    }

    fn initial_prompt_tokens_for_mode(
        generation: &GenerationConfig,
        tokenizer: &Tokenizer,
        language: Option<&str>,
        mode: WhisperDecodeMode,
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
        let no_timestamps = token_id(tokenizer, whisper::NO_TIMESTAMPS_TOKEN);
        if mode == WhisperDecodeMode::WithoutTimestamps {
            tokens.push(no_timestamps.ok_or_else(|| {
                invalid_request("Whisper tokenizer is missing no-timestamps token")
            })?);
        }
        if let Some(forced) = &generation.forced_decoder_ids {
            for (position, token) in forced {
                let Some(token) = token else {
                    continue;
                };
                if mode == WhisperDecodeMode::TimestampTokens
                    && no_timestamps.is_some_and(|no_timestamps| no_timestamps == *token)
                {
                    continue;
                }
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

fn decode_timestamp_window(
    tokenizer: &Tokenizer,
    token_ids: &[u32],
) -> Result<Option<WhisperDecodedWindow>> {
    let spec = whisper_timestamp_spec(tokenizer)?;
    decode_timestamp_window_with_spec(tokenizer, token_ids, &spec)
}

fn decode_timestamp_window_with_spec(
    tokenizer: &Tokenizer,
    token_ids: &[u32],
    spec: &WhisperTimestampSpec,
) -> Result<Option<WhisperDecodedWindow>> {
    let mut segments = Vec::new();
    let mut pending_text_tokens = Vec::new();
    let mut segment_start = None;
    let mut previous_timestamp = None;
    let mut saw_timestamp = false;

    for token_id in token_ids {
        if let Some(seconds) = timestamp_seconds(*token_id, spec) {
            saw_timestamp = true;
            if let Some(previous) = previous_timestamp {
                if seconds < previous {
                    return Err(model_output_mismatch(format!(
                        "Whisper timestamp tokens are not monotonic: {seconds:.2} after {previous:.2}"
                    )));
                }
            }
            if !pending_text_tokens.is_empty() {
                let start_seconds = segment_start.unwrap_or(seconds);
                if seconds < start_seconds {
                    return Err(model_output_mismatch(format!(
                        "Whisper timestamp segment ends before it starts: {seconds:.2} < {start_seconds:.2}"
                    )));
                }
                let text = decode_text_tokens(tokenizer, &pending_text_tokens)?;
                if !text.is_empty() {
                    segments.push(WhisperDecodedSegment {
                        text,
                        start_seconds,
                        end_seconds: seconds,
                        token_ids: std::mem::take(&mut pending_text_tokens),
                    });
                } else {
                    pending_text_tokens.clear();
                }
            }
            segment_start = Some(seconds);
            previous_timestamp = Some(seconds);
        } else {
            pending_text_tokens.push(*token_id);
        }
    }

    if !pending_text_tokens.is_empty() {
        return Ok(None);
    }
    if !saw_timestamp {
        return Ok(None);
    }

    let text = segments
        .iter()
        .map(|segment| segment.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    Ok(Some(WhisperDecodedWindow { text, segments }))
}

fn whisper_timestamp_spec(tokenizer: &Tokenizer) -> Result<WhisperTimestampSpec> {
    let begin_token_id = token_id(tokenizer, "<|0.00|>").ok_or_else(|| {
        invalid_request("Whisper tokenizer is missing timestamp token `<|0.00|>`")
    })?;
    let end_token_id = token_id(tokenizer, "<|30.00|>")
        .map(|token| token + 1)
        .unwrap_or(begin_token_id + WHISPER_TIMESTAMP_TOKEN_COUNT);
    if end_token_id <= begin_token_id {
        return Err(invalid_request(
            "Whisper timestamp token range is empty or malformed",
        ));
    }
    Ok(WhisperTimestampSpec {
        begin_token_id,
        end_token_id,
        seconds_per_token: WHISPER_TIMESTAMP_SECONDS_PER_TOKEN,
    })
}

fn timestamp_seconds(token_id: u32, spec: &WhisperTimestampSpec) -> Option<f64> {
    (spec.begin_token_id..spec.end_token_id)
        .contains(&token_id)
        .then(|| (token_id - spec.begin_token_id) as f64 * spec.seconds_per_token)
}

fn decode_text_tokens(tokenizer: &Tokenizer, token_ids: &[u32]) -> Result<String> {
    tokenizer
        .decode(token_ids, true)
        .map(clean_decoded_text)
        .map_err(|error| model_output_mismatch(format!("failed to decode Whisper tokens: {error}")))
}

fn clean_decoded_text(text: String) -> String {
    text.replace("  ", " ").trim().to_string()
}

fn window_fallback_segment(
    index: u64,
    text: String,
    start_seconds: f64,
    end_seconds: f64,
    language: Option<String>,
) -> TranscriptSegmentContract {
    let mut segment = TranscriptSegmentContract::new(index, text);
    segment.start_seconds = Some(start_seconds);
    segment.end_seconds = Some(end_seconds);
    segment.language = language;
    segment
        .attributes
        .insert("provider".to_string(), "candle-whisper".to_string());
    segment
        .attributes
        .insert("timing".to_string(), "chunkLocal".to_string());
    segment
}

fn decoded_window_to_contract_segments(
    decoded: WhisperDecodedWindow,
    next_index: &mut u64,
    window_start_seconds: f64,
    window_end_seconds: f64,
    language: Option<String>,
) -> Vec<TranscriptSegmentContract> {
    decoded
        .segments
        .into_iter()
        .filter_map(|decoded_segment| {
            let text = decoded_segment.text.trim().to_string();
            if text.is_empty() {
                return None;
            }
            let mut segment = TranscriptSegmentContract::new(*next_index, text);
            *next_index += 1;
            segment.start_seconds = Some(
                (window_start_seconds + decoded_segment.start_seconds)
                    .clamp(window_start_seconds, window_end_seconds),
            );
            segment.end_seconds = Some(
                (window_start_seconds + decoded_segment.end_seconds)
                    .clamp(window_start_seconds, window_end_seconds),
            );
            segment.language = language.clone();
            segment
                .attributes
                .insert("provider".to_string(), "candle-whisper".to_string());
            segment
                .attributes
                .insert("timing".to_string(), "whisperTimestampTokens".to_string());
            Some(segment)
        })
        .collect()
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

    fn timestamp_test_tokenizer() -> Tokenizer {
        let temp = tempfile::tempdir().unwrap();
        let vocab = temp.path().join("vocab.json");
        std::fs::write(
            &vocab,
            serde_json::json!({
                "<unk>": 0,
                "hello": 10,
                "world": 11,
                "again": 12,
                "<|0.00|>": 100,
                "<|1.00|>": 150,
                "<|2.00|>": 200,
                "<|3.00|>": 250,
                "<|30.00|>": 1600
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
    fn timestamp_prompt_omits_no_timestamps_token() {
        let mut generation = test_generation();
        generation.forced_decoder_ids = Some(vec![(3, Some(7))]);
        let tokens = CandleWhisperSession::initial_prompt_tokens_for_mode(
            &generation,
            &test_tokenizer(),
            Some("en"),
            WhisperDecodeMode::TimestampTokens,
        )
        .unwrap();
        assert_eq!(tokens, vec![1, 3, 5]);
        assert!(!tokens.contains(&7));
    }

    #[test]
    fn timestamp_token_detection_uses_whisper_zero_token() {
        let spec = whisper_timestamp_spec(&timestamp_test_tokenizer()).unwrap();
        assert_eq!(spec.begin_token_id, 100);
        assert_eq!(spec.end_token_id, 1601);
        assert_eq!(timestamp_seconds(150, &spec), Some(1.0));
        assert_eq!(timestamp_seconds(99, &spec), None);
    }

    #[test]
    fn missing_timestamp_metadata_returns_invalid_request() {
        let error = whisper_timestamp_spec(&test_tokenizer())
            .unwrap_err()
            .to_string();
        assert!(error.contains("invalid_request"));
        assert!(error.contains("<|0.00|>"));
    }

    #[test]
    fn timestamp_decode_reads_one_bounded_segment() {
        let tokenizer = timestamp_test_tokenizer();
        let decoded = decode_timestamp_window(&tokenizer, &[100, 10, 11, 150])
            .unwrap()
            .unwrap();
        assert_eq!(decoded.text, "hello world");
        assert_eq!(decoded.segments.len(), 1);
        assert_eq!(decoded.segments[0].text, "hello world");
        assert_eq!(decoded.segments[0].start_seconds, 0.0);
        assert_eq!(decoded.segments[0].end_seconds, 1.0);
        assert_eq!(decoded.segments[0].token_ids, vec![10, 11]);
    }

    #[test]
    fn timestamp_decode_reads_multiple_bounded_segments() {
        let tokenizer = timestamp_test_tokenizer();
        let decoded = decode_timestamp_window(&tokenizer, &[100, 10, 150, 150, 11, 200])
            .unwrap()
            .unwrap();
        assert_eq!(decoded.text, "hello world");
        assert_eq!(decoded.segments.len(), 2);
        assert_eq!(decoded.segments[0].text, "hello");
        assert_eq!(decoded.segments[0].start_seconds, 0.0);
        assert_eq!(decoded.segments[0].end_seconds, 1.0);
        assert_eq!(decoded.segments[1].text, "world");
        assert_eq!(decoded.segments[1].start_seconds, 1.0);
        assert_eq!(decoded.segments[1].end_seconds, 2.0);
    }

    #[test]
    fn timestamp_decode_missing_end_timestamp_falls_back() {
        let tokenizer = timestamp_test_tokenizer();
        let decoded = decode_timestamp_window(&tokenizer, &[100, 10, 11]).unwrap();
        assert!(decoded.is_none());
    }

    #[test]
    fn timestamp_decode_rejects_non_monotonic_timestamps() {
        let tokenizer = timestamp_test_tokenizer();
        let error = decode_timestamp_window(&tokenizer, &[150, 10, 100])
            .unwrap_err()
            .to_string();
        assert!(error.contains("model_output_mismatch"));
        assert!(error.contains("not monotonic"));
    }

    #[test]
    fn timestamp_decode_uses_text_between_timestamp_pairs() {
        let tokenizer = timestamp_test_tokenizer();
        let decoded = decode_timestamp_window(&tokenizer, &[100, 150, 10, 200, 250])
            .unwrap()
            .unwrap();
        assert_eq!(decoded.segments.len(), 1);
        assert_eq!(decoded.segments[0].text, "hello");
        assert_eq!(decoded.segments[0].start_seconds, 1.0);
        assert_eq!(decoded.segments[0].end_seconds, 2.0);
    }

    #[test]
    fn timestamp_decoded_segments_map_to_transcript_contracts() {
        let decoded = WhisperDecodedWindow {
            text: "hello".to_string(),
            segments: vec![WhisperDecodedSegment {
                text: "hello".to_string(),
                start_seconds: 0.5,
                end_seconds: 1.25,
                token_ids: vec![10],
            }],
        };
        let mut next_index = 7;
        let segments = decoded_window_to_contract_segments(
            decoded,
            &mut next_index,
            10.0,
            12.0,
            Some("en".to_string()),
        );
        assert_eq!(next_index, 8);
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].index, 7);
        assert_eq!(segments[0].text, "hello");
        assert_eq!(segments[0].start_seconds, Some(10.5));
        assert_eq!(segments[0].end_seconds, Some(11.25));
        assert_eq!(segments[0].language.as_deref(), Some("en"));
        assert_eq!(
            segments[0].attributes.get("timing").map(String::as_str),
            Some("whisperTimestampTokens")
        );
        TranscriptionContract::from_segments(None, Some("en".to_string()), segments).unwrap();
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
