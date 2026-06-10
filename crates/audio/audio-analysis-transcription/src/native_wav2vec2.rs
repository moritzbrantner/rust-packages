use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;
use video_analysis_core::Result;

use crate::ctc_alignment::{
    backtrack_ctc, build_ctc_trellis, tokens_to_segment_words, CtcVocabulary,
};
use crate::{
    invalid_request, model_output_mismatch, unsupported_runtime, AlignedWord, AlignmentRequest,
};

#[derive(Debug, Clone)]
pub(crate) struct Wav2Vec2BundlePaths {
    pub config_json: PathBuf,
    pub tokenizer_json: PathBuf,
    pub preprocessor_config_json: PathBuf,
    pub model_safetensors: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct Wav2Vec2ConfigSummary {
    pub model_type: Option<String>,
    pub architectures: Vec<String>,
    pub vocab_size: Option<usize>,
    pub word_delimiter_token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct Wav2Vec2Vocabulary {
    pub ctc: CtcVocabulary,
    pub word_delimiter_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawWav2Vec2Config {
    #[serde(default)]
    model_type: Option<String>,
    #[serde(default)]
    architectures: Vec<String>,
    #[serde(default)]
    vocab_size: Option<usize>,
    #[serde(default)]
    word_delimiter_token: Option<String>,
    #[serde(default)]
    hidden_size: Option<usize>,
    #[serde(default)]
    num_hidden_layers: Option<usize>,
    #[serde(default)]
    num_attention_heads: Option<usize>,
    #[serde(default)]
    intermediate_size: Option<usize>,
    #[serde(default)]
    hidden_act: Option<String>,
    #[serde(default)]
    layer_norm_eps: Option<f64>,
    #[serde(default)]
    feat_extract_norm: Option<String>,
    #[serde(default)]
    feat_extract_activation: Option<String>,
    #[serde(default)]
    conv_dim: Vec<usize>,
    #[serde(default)]
    conv_stride: Vec<usize>,
    #[serde(default)]
    conv_kernel: Vec<usize>,
    #[serde(default)]
    conv_bias: Option<bool>,
    #[serde(default)]
    num_conv_pos_embeddings: Option<usize>,
    #[serde(default)]
    num_conv_pos_embedding_groups: Option<usize>,
    #[serde(default)]
    do_stable_layer_norm: Option<bool>,
    #[serde(default)]
    final_dropout: Option<f64>,
    #[serde(default)]
    hidden_dropout: Option<f64>,
    #[serde(default)]
    activation_dropout: Option<f64>,
    #[serde(default)]
    attention_dropout: Option<f64>,
    #[serde(default)]
    layerdrop: Option<f64>,
    #[serde(default)]
    pad_token_id: Option<usize>,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub(crate) struct Wav2Vec2CtcConfig {
    pub model_type: Option<String>,
    pub architectures: Vec<String>,
    pub vocab_size: usize,
    pub word_delimiter_token: Option<String>,
    pub hidden_size: usize,
    pub num_hidden_layers: usize,
    pub num_attention_heads: usize,
    pub intermediate_size: usize,
    pub hidden_act: String,
    pub layer_norm_eps: f64,
    pub feat_extract_norm: Option<String>,
    pub feat_extract_activation: String,
    pub conv_dim: Vec<usize>,
    pub conv_stride: Vec<usize>,
    pub conv_kernel: Vec<usize>,
    pub conv_bias: bool,
    pub num_conv_pos_embeddings: usize,
    pub num_conv_pos_embedding_groups: usize,
    pub do_stable_layer_norm: bool,
    pub final_dropout: Option<f64>,
    pub hidden_dropout: Option<f64>,
    pub activation_dropout: Option<f64>,
    pub attention_dropout: Option<f64>,
    pub layerdrop: Option<f64>,
    pub pad_token_id: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct RawWav2Vec2PreprocessorConfig {
    #[serde(default)]
    sampling_rate: Option<u32>,
    #[serde(default)]
    do_normalize: Option<bool>,
    #[serde(default)]
    return_attention_mask: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct Wav2Vec2PreprocessorConfig {
    pub sampling_rate: Option<u32>,
    pub do_normalize: Option<bool>,
    pub return_attention_mask: Option<bool>,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub(crate) struct Wav2Vec2CtcEmission {
    pub segment_index: u64,
    pub emissions: Vec<Vec<f32>>,
    pub token_ids: Vec<usize>,
    pub blank_id: usize,
    pub transcript_words: Vec<String>,
    pub segment_start_seconds: f64,
    pub segment_end_seconds: f64,
    pub frame_seconds: f64,
}

#[allow(dead_code)]
pub(crate) fn emit_wav2vec2_ctc(
    bundle: &Path,
    request: &AlignmentRequest,
) -> Result<Vec<Vec<f32>>> {
    let emissions = emit_wav2vec2_ctc_segments(bundle, request)?;
    Ok(emissions
        .into_iter()
        .flat_map(|segment| segment.emissions)
        .collect())
}

pub(crate) fn align_wav2vec2_ctc(
    bundle: &Path,
    request: &AlignmentRequest,
) -> Result<Vec<AlignedWord>> {
    let emission_segments = emit_wav2vec2_ctc_segments(bundle, request)?;
    let mut aligned_words = Vec::new();
    for segment in emission_segments {
        let trellis = build_ctc_trellis(&segment.emissions, &segment.token_ids, segment.blank_id)?;
        let path = backtrack_ctc(
            &trellis,
            &segment.emissions,
            &segment.token_ids,
            segment.blank_id,
        )?;
        aligned_words.extend(tokens_to_segment_words(
            segment.segment_index,
            &path,
            &segment.transcript_words,
            segment.segment_start_seconds,
            segment.segment_end_seconds,
            segment.frame_seconds,
        )?);
    }
    Ok(aligned_words)
}

pub(crate) fn emit_wav2vec2_ctc_segments(
    bundle: &Path,
    request: &AlignmentRequest,
) -> Result<Vec<Wav2Vec2CtcEmission>> {
    let paths = resolve_wav2vec2_bundle_paths(bundle)?;
    let config = parse_wav2vec2_ctc_config(&paths.config_json)?;
    let preprocessor = parse_wav2vec2_preprocessor_config(&paths.preprocessor_config_json)?;
    let vocab = parse_ctc_vocabulary(&paths.tokenizer_json)?;
    if vocab.tokens.len() > config.vocab_size {
        return Err(model_output_mismatch(format!(
            "wav2vec2 tokenizer vocab has {} entries but config vocab_size is {}",
            vocab.tokens.len(),
            config.vocab_size
        )));
    }
    let model = crate::native_wav2vec2_model::Wav2Vec2ForCtc::load(
        &paths.model_safetensors,
        config,
        preprocessor,
    )?;
    let mut segments = Vec::new();
    let audio_duration = request.audio.duration_seconds();
    for segment in &request.transcript.segments {
        let segment_start = segment.start_seconds.unwrap_or(0.0);
        let segment_end = segment.end_seconds.unwrap_or(audio_duration);
        if !segment_start.is_finite()
            || !segment_end.is_finite()
            || segment_end <= segment_start
            || segment_end > audio_duration + 1e-6
        {
            return Err(invalid_request(
                "transcript segment timing is outside audio range",
            ));
        }
        let transcript_words = segment_words(segment);
        if transcript_words.is_empty() {
            continue;
        }
        let transcript_text = transcript_words.join(" ");
        let token_ids = normalized_text_to_token_ids(&transcript_text, &vocab)?;
        let samples = slice_segment_samples(
            &request.audio.samples,
            request.audio.sample_rate,
            request.audio.channels,
            segment_start,
            segment_end,
        )?;
        let emissions = model.emit_log_probs(&samples)?;
        let frame_seconds = (segment_end - segment_start) / emissions.len() as f64;
        if !frame_seconds.is_finite() || frame_seconds <= 0.0 {
            return Err(model_output_mismatch(
                "wav2vec2 CTC frame timing is invalid",
            ));
        }
        segments.push(Wav2Vec2CtcEmission {
            segment_index: segment.index,
            emissions,
            token_ids,
            blank_id: vocab.blank_id,
            transcript_words,
            segment_start_seconds: segment_start,
            segment_end_seconds: segment_end,
            frame_seconds,
        });
    }
    Ok(segments)
}

pub(crate) fn resolve_wav2vec2_bundle_paths(bundle: &Path) -> Result<Wav2Vec2BundlePaths> {
    Ok(Wav2Vec2BundlePaths {
        config_json: crate::native_bundles::resolve_required_bundle_file(bundle, "config.json")?,
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

#[allow(dead_code)]
pub(crate) fn parse_wav2vec2_config(path: &Path) -> Result<Wav2Vec2ConfigSummary> {
    let config = parse_raw_wav2vec2_config(path)?;
    validate_model_type_and_architecture(&config)?;
    Ok(Wav2Vec2ConfigSummary {
        model_type: config.model_type,
        architectures: config.architectures,
        vocab_size: config.vocab_size,
        word_delimiter_token: config.word_delimiter_token,
    })
}

pub(crate) fn parse_wav2vec2_ctc_config(path: &Path) -> Result<Wav2Vec2CtcConfig> {
    let raw = parse_raw_wav2vec2_config(path)?;
    validate_model_type_and_architecture(&raw)?;
    let vocab_size = raw
        .vocab_size
        .ok_or_else(|| invalid_request("wav2vec2 config missing vocab_size"))?;
    let hidden_size = raw
        .hidden_size
        .ok_or_else(|| invalid_request("wav2vec2 config missing hidden_size"))?;
    let num_attention_heads = raw
        .num_attention_heads
        .ok_or_else(|| invalid_request("wav2vec2 config missing num_attention_heads"))?;
    if num_attention_heads == 0 || hidden_size % num_attention_heads != 0 {
        return Err(invalid_request(
            "wav2vec2 hidden_size must be divisible by num_attention_heads",
        ));
    }
    if raw.conv_dim.is_empty()
        || raw.conv_dim.len() != raw.conv_stride.len()
        || raw.conv_dim.len() != raw.conv_kernel.len()
    {
        return Err(invalid_request(
            "wav2vec2 conv_dim, conv_stride, and conv_kernel must be non-empty and have matching lengths",
        ));
    }
    if raw
        .conv_dim
        .iter()
        .chain(raw.conv_stride.iter())
        .chain(raw.conv_kernel.iter())
        .any(|value| *value == 0)
    {
        return Err(invalid_request(
            "wav2vec2 convolution dimensions, strides, and kernels must be positive",
        ));
    }
    Ok(Wav2Vec2CtcConfig {
        model_type: raw.model_type,
        architectures: raw.architectures,
        vocab_size,
        word_delimiter_token: raw.word_delimiter_token,
        hidden_size,
        num_hidden_layers: raw.num_hidden_layers.unwrap_or(0),
        num_attention_heads,
        intermediate_size: raw.intermediate_size.unwrap_or(hidden_size * 4),
        hidden_act: raw.hidden_act.unwrap_or_else(|| "gelu".to_string()),
        layer_norm_eps: raw.layer_norm_eps.unwrap_or(1e-5),
        feat_extract_norm: raw.feat_extract_norm,
        feat_extract_activation: raw
            .feat_extract_activation
            .unwrap_or_else(|| "gelu".to_string()),
        conv_dim: raw.conv_dim,
        conv_stride: raw.conv_stride,
        conv_kernel: raw.conv_kernel,
        conv_bias: raw.conv_bias.unwrap_or(false),
        num_conv_pos_embeddings: raw.num_conv_pos_embeddings.unwrap_or(0),
        num_conv_pos_embedding_groups: raw.num_conv_pos_embedding_groups.unwrap_or(1),
        do_stable_layer_norm: raw.do_stable_layer_norm.unwrap_or(false),
        final_dropout: raw.final_dropout,
        hidden_dropout: raw.hidden_dropout,
        activation_dropout: raw.activation_dropout,
        attention_dropout: raw.attention_dropout,
        layerdrop: raw.layerdrop,
        pad_token_id: raw.pad_token_id,
    })
}

pub(crate) fn parse_wav2vec2_preprocessor_config(
    path: &Path,
) -> Result<Wav2Vec2PreprocessorConfig> {
    let bytes = std::fs::read(path).map_err(|error| {
        crate::setup_error(format!(
            "failed to read wav2vec2 preprocessor config `{}`: {error}",
            path.display()
        ))
    })?;
    let raw: RawWav2Vec2PreprocessorConfig = serde_json::from_slice(&bytes).map_err(|error| {
        invalid_request(format!(
            "failed to parse wav2vec2 preprocessor config `{}`: {error}",
            path.display()
        ))
    })?;
    if let Some(sampling_rate) = raw.sampling_rate {
        if sampling_rate != 16_000 {
            return Err(invalid_request(format!(
                "wav2vec2 preprocessor sampling_rate must be 16000, got {sampling_rate}"
            )));
        }
    }
    Ok(Wav2Vec2PreprocessorConfig {
        sampling_rate: raw.sampling_rate,
        do_normalize: raw.do_normalize,
        return_attention_mask: raw.return_attention_mask,
    })
}

fn parse_raw_wav2vec2_config(path: &Path) -> Result<RawWav2Vec2Config> {
    let bytes = std::fs::read(path).map_err(|error| {
        crate::setup_error(format!(
            "failed to read wav2vec2 config `{}`: {error}",
            path.display()
        ))
    })?;
    serde_json::from_slice(&bytes).map_err(|error| {
        invalid_request(format!(
            "failed to parse wav2vec2 config `{}`: {error}",
            path.display()
        ))
    })
}

fn validate_model_type_and_architecture(raw: &RawWav2Vec2Config) -> Result<()> {
    if let Some(model_type) = raw.model_type.as_deref() {
        if model_type != "wav2vec2" {
            return Err(unsupported_runtime(format!(
                "unsupported CTC alignment model type `{model_type}`; expected wav2vec2"
            )));
        }
    }
    if !raw.architectures.is_empty()
        && !raw
            .architectures
            .iter()
            .any(|architecture| architecture == "Wav2Vec2ForCTC")
    {
        return Err(unsupported_runtime(format!(
            "unsupported wav2vec2 architecture `{}`; expected Wav2Vec2ForCTC",
            raw.architectures.join(",")
        )));
    }
    Ok(())
}

pub(crate) fn parse_ctc_vocabulary(tokenizer_json: &Path) -> Result<CtcVocabulary> {
    let bytes = std::fs::read(tokenizer_json).map_err(|error| {
        crate::setup_error(format!(
            "failed to read wav2vec2 tokenizer `{}`: {error}",
            tokenizer_json.display()
        ))
    })?;
    let value: Value = serde_json::from_slice(&bytes).map_err(|error| {
        invalid_request(format!(
            "failed to parse wav2vec2 tokenizer `{}`: {error}",
            tokenizer_json.display()
        ))
    })?;
    let vocab = value
        .pointer("/model/vocab")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            unsupported_runtime(
                "unsupported wav2vec2 tokenizer layout; expected flat model.vocab mapping",
            )
        })?;
    let max_id = vocab
        .values()
        .filter_map(Value::as_u64)
        .max()
        .ok_or_else(|| unsupported_runtime("wav2vec2 tokenizer vocab is empty"))?
        as usize;
    let mut tokens = vec![None; max_id + 1];
    for (token, id) in vocab {
        let Some(id) = id.as_u64() else {
            return Err(unsupported_runtime(
                "unsupported wav2vec2 tokenizer layout; vocab ids must be integers",
            ));
        };
        let id = id as usize;
        if tokens[id].is_some() {
            return Err(unsupported_runtime(
                "unsupported wav2vec2 tokenizer layout; duplicate vocab id",
            ));
        }
        tokens[id] = Some(token.clone());
    }
    let tokens = tokens
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| {
            unsupported_runtime("unsupported wav2vec2 tokenizer layout; vocab ids must be dense")
        })?;
    let word_delimiter_token = value
        .pointer("/word_delimiter_token")
        .or_else(|| value.pointer("/model/word_delimiter_token"))
        .and_then(Value::as_str);
    let pad_token = value
        .pointer("/padding/pad_token")
        .or_else(|| value.pointer("/model/pad_token"))
        .and_then(Value::as_str);
    let blank_id = pad_token
        .and_then(|token| token_id(&tokens, token))
        .or_else(|| token_id(&tokens, "[PAD]"))
        .or_else(|| token_id(&tokens, "<pad>"))
        .or_else(|| {
            (word_delimiter_token == Some("|"))
                .then(|| token_id(&tokens, "|"))
                .flatten()
        })
        .ok_or_else(|| {
            unsupported_runtime(
                "unsupported wav2vec2 tokenizer layout; could not determine CTC blank token",
            )
        })?;
    Ok(CtcVocabulary { blank_id, tokens })
}

pub(crate) fn normalized_text_to_token_ids(
    text: &str,
    vocab: &CtcVocabulary,
) -> Result<Vec<usize>> {
    let delimiter = token_id(&vocab.tokens, "|").map(|_| "|");
    let uppercase_hits = vocab
        .tokens
        .iter()
        .filter(|token| token.chars().any(|ch| ch.is_ascii_uppercase()))
        .count();
    let lowercase_hits = vocab
        .tokens
        .iter()
        .filter(|token| token.chars().any(|ch| ch.is_ascii_lowercase()))
        .count();
    let use_uppercase = uppercase_hits >= lowercase_hits;
    let mut ids = Vec::new();
    for character in text.chars() {
        if character.is_whitespace() {
            if let Some(delimiter) = delimiter {
                if let Some(id) = token_id(&vocab.tokens, delimiter) {
                    ids.push(id);
                }
            }
            continue;
        }
        let token = if character.is_alphabetic() {
            if use_uppercase {
                character.to_uppercase().collect::<String>()
            } else {
                character.to_lowercase().collect::<String>()
            }
        } else {
            character.to_string()
        };
        if let Some(id) = token_id(&vocab.tokens, &token) {
            ids.push(id);
        } else if character.is_alphanumeric() {
            return Err(invalid_request(format!(
                "transcript character `{character}` is not representable by wav2vec2 tokenizer"
            )));
        }
    }
    if ids.is_empty() {
        return Err(invalid_request(
            "transcript text does not contain any wav2vec2 CTC tokens",
        ));
    }
    Ok(ids)
}

fn segment_words(segment: &text_transcripts::TranscriptSegmentContract) -> Vec<String> {
    if segment.words.is_empty() {
        segment
            .text
            .split_whitespace()
            .map(|word| word.trim().to_string())
            .filter(|word| !word.is_empty())
            .collect()
    } else {
        segment
            .words
            .iter()
            .map(|word| word.text.trim().to_string())
            .filter(|word| !word.is_empty())
            .collect()
    }
}

fn slice_segment_samples(
    samples: &[f32],
    sample_rate: u32,
    channels: u16,
    segment_start: f64,
    segment_end: f64,
) -> Result<Vec<f32>> {
    if sample_rate == 0 || channels == 0 {
        return Err(invalid_request(
            "audio sample rate and channels must be positive",
        ));
    }
    let channels = channels as usize;
    let frame_count = samples.len() / channels;
    let start_frame = (segment_start * sample_rate as f64)
        .round()
        .clamp(0.0, frame_count as f64) as usize;
    let end_frame = (segment_end * sample_rate as f64)
        .round()
        .clamp(start_frame as f64, frame_count as f64) as usize;
    if end_frame <= start_frame {
        return Err(invalid_request("alignment segment audio slice is empty"));
    }
    let mut mono = Vec::with_capacity(end_frame - start_frame);
    for frame in start_frame..end_frame {
        let offset = frame * channels;
        let value = if channels == 1 {
            samples[offset]
        } else {
            samples[offset..offset + channels]
                .iter()
                .copied()
                .sum::<f32>()
                / channels as f32
        };
        mono.push(value);
    }
    Ok(mono)
}

fn token_id(tokens: &[String], token: &str) -> Option<usize> {
    tokens.iter().position(|candidate| candidate == token)
}

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::{Device, Tensor};
    use std::collections::HashMap;
    use text_transcripts::{TranscriptSegmentContract, TranscriptionContract};

    fn write_valid_bundle(root: &Path, nested: bool) {
        let file_root = if nested {
            std::fs::create_dir(root.join("files")).unwrap();
            root.join("files")
        } else {
            root.to_path_buf()
        };
        std::fs::write(
            file_root.join("config.json"),
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
        std::fs::write(file_root.join("tokenizer.json"), minimal_tokenizer()).unwrap();
        std::fs::write(file_root.join("preprocessor_config.json"), "{}").unwrap();
        std::fs::write(file_root.join("model.safetensors"), "").unwrap();
    }

    fn minimal_tokenizer() -> String {
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
        .to_string()
    }

    fn write_tiny_bundle(root: &Path) {
        write_valid_bundle(root, false);
        std::fs::write(
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

    fn alignment_request(text: &str) -> AlignmentRequest {
        let mut segment = TranscriptSegmentContract::new(7, text);
        segment.start_seconds = Some(0.0);
        segment.end_seconds = Some(1.0);
        let transcript =
            TranscriptionContract::from_segments(None, Some("en".to_string()), vec![segment])
                .unwrap();
        AlignmentRequest {
            audio: crate::LoadedAudio {
                samples: vec![0.0; 16_000],
                sample_rate: 16_000,
                channels: 1,
                source: None,
            },
            transcript,
            language: Some("en".to_string()),
            model_id: "facebook/wav2vec2-base-960h".to_string(),
        }
    }

    #[test]
    fn bundle_resolution_accepts_direct_and_files_layouts() {
        for nested in [false, true] {
            let temp = tempfile::tempdir().unwrap();
            write_valid_bundle(temp.path(), nested);
            let paths = resolve_wav2vec2_bundle_paths(temp.path()).unwrap();
            assert!(paths.config_json.exists());
            assert!(paths.tokenizer_json.exists());
            assert!(paths.preprocessor_config_json.exists());
            assert!(paths.model_safetensors.exists());
        }
    }

    #[cfg(feature = "model-bundles")]
    #[test]
    fn bundle_resolution_accepts_manifest_layout() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        std::fs::create_dir(root.join("snapshots")).unwrap();
        write_valid_bundle(&root.join("snapshots"), false);
        std::fs::write(
            root.join("manifest.json"),
            serde_json::json!({
                "schema_version": 1,
                "name": "wav2vec2-test",
                "repo_id": "facebook/wav2vec2-base-960h",
                "revision": "main",
                "task": "speech_recognition",
                "files": {
                    "config.json": {"remote_path": "config.json", "local_path": "snapshots/config.json", "size_bytes": 0},
                    "tokenizer.json": {"remote_path": "tokenizer.json", "local_path": "snapshots/tokenizer.json", "size_bytes": 0},
                    "preprocessor_config.json": {"remote_path": "preprocessor_config.json", "local_path": "snapshots/preprocessor_config.json", "size_bytes": 0},
                    "model.safetensors": {"remote_path": "model.safetensors", "local_path": "snapshots/model.safetensors", "size_bytes": 0}
                }
            })
            .to_string(),
        )
        .unwrap();
        let paths = resolve_wav2vec2_bundle_paths(root).unwrap();
        assert_eq!(
            paths.model_safetensors,
            root.join("snapshots/model.safetensors")
        );
    }

    #[test]
    fn tokenizer_vocab_parser_accepts_minimal_wav2vec2_layout() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("tokenizer.json");
        std::fs::write(&path, minimal_tokenizer()).unwrap();
        let vocab = parse_ctc_vocabulary(&path).unwrap();
        assert_eq!(vocab.blank_id, 0);
        assert_eq!(vocab.tokens[5], "|");
    }

    #[test]
    fn text_normalization_maps_words_to_ctc_ids() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("tokenizer.json");
        std::fs::write(&path, minimal_tokenizer()).unwrap();
        let vocab = parse_ctc_vocabulary(&path).unwrap();
        let ids = normalized_text_to_token_ids("hello world!", &vocab).unwrap();
        assert_eq!(ids, vec![1, 2, 3, 3, 4, 5, 6, 4, 7, 3, 8]);
    }

    #[test]
    fn unsupported_tokenizer_layout_returns_unsupported_runtime() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("tokenizer.json");
        std::fs::write(
            &path,
            serde_json::json!({"model": {"type": "BPE"}}).to_string(),
        )
        .unwrap();
        let error = parse_ctc_vocabulary(&path).unwrap_err().to_string();
        assert!(error.contains("unsupported_runtime"));
        assert!(error.contains("flat model.vocab"));
    }

    #[test]
    fn wav2vec2_config_accepts_minimal_ctc_config() {
        let temp = tempfile::tempdir().unwrap();
        write_valid_bundle(temp.path(), false);
        let config = parse_wav2vec2_ctc_config(&temp.path().join("config.json")).unwrap();
        assert_eq!(config.vocab_size, 10);
        assert_eq!(config.hidden_size, 1);
        assert_eq!(config.conv_dim, vec![1]);
    }

    #[test]
    fn wav2vec2_config_rejects_bad_conv_shapes() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(
            temp.path().join("config.json"),
            serde_json::json!({
                "model_type": "wav2vec2",
                "architectures": ["Wav2Vec2ForCTC"],
                "vocab_size": 10,
                "hidden_size": 4,
                "num_attention_heads": 2,
                "conv_dim": [4, 4],
                "conv_stride": [2],
                "conv_kernel": [3, 3]
            })
            .to_string(),
        )
        .unwrap();
        let error = parse_wav2vec2_ctc_config(&temp.path().join("config.json"))
            .unwrap_err()
            .to_string();
        assert!(error.contains("invalid_request"));
        assert!(error.contains("conv_dim"));
    }

    #[test]
    fn wav2vec2_preprocessor_accepts_16khz() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("preprocessor_config.json");
        std::fs::write(
            &path,
            serde_json::json!({"sampling_rate": 16000, "do_normalize": true}).to_string(),
        )
        .unwrap();
        let config = parse_wav2vec2_preprocessor_config(&path).unwrap();
        assert_eq!(config.sampling_rate, Some(16_000));
        assert_eq!(config.do_normalize, Some(true));
    }

    #[test]
    fn wav2vec2_preprocessor_rejects_non_16khz() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("preprocessor_config.json");
        std::fs::write(
            &path,
            serde_json::json!({"sampling_rate": 8000}).to_string(),
        )
        .unwrap();
        let error = parse_wav2vec2_preprocessor_config(&path)
            .unwrap_err()
            .to_string();
        assert!(error.contains("invalid_request"));
        assert!(error.contains("16000"));
    }

    #[test]
    fn unsupported_wav2vec2_layout_reports_missing_key() {
        let temp = tempfile::tempdir().unwrap();
        write_valid_bundle(temp.path(), false);
        let error = emit_wav2vec2_ctc(temp.path(), &alignment_request("hello"))
            .unwrap_err()
            .to_string();
        assert!(error.contains("unsupported_runtime"));
        assert!(error.contains("safetensors"));
    }

    #[test]
    fn tiny_wav2vec2_model_emits_finite_log_probs() {
        let temp = tempfile::tempdir().unwrap();
        write_tiny_bundle(temp.path());
        let emissions = emit_wav2vec2_ctc(temp.path(), &alignment_request("hello")).unwrap();
        assert!(!emissions.is_empty());
        assert!(emissions
            .iter()
            .all(|frame| frame.len() == 10 && frame.iter().all(|score| score.is_finite())));
    }

    #[test]
    fn alignment_with_tiny_wav2vec2_bundle_returns_words() {
        let temp = tempfile::tempdir().unwrap();
        write_tiny_bundle(temp.path());
        let words = align_wav2vec2_ctc(temp.path(), &alignment_request("hello")).unwrap();
        assert_eq!(words.len(), 1);
        assert_eq!(words[0].segment_index, 7);
        assert_eq!(words[0].text, "hello");
        assert!(words[0].start_seconds >= 0.0);
        assert!(words[0].end_seconds <= 1.0);
        assert!(words[0].end_seconds >= words[0].start_seconds);
        assert!(words[0].confidence.is_some());
    }
}
