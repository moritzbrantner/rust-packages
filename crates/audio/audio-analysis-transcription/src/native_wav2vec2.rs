use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;
use video_analysis_core::Result;

use crate::ctc_alignment::CtcVocabulary;
use crate::{invalid_request, unsupported_runtime, AlignmentRequest};

const WAV2VEC2_UNSUPPORTED_MESSAGE: &str =
    "wav2vec2 Candle emission execution is not available because candle-transformers 0.10.2 does not expose a wav2vec2 model implementation";

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
}

pub(crate) fn emit_wav2vec2_ctc(
    bundle: &Path,
    request: &AlignmentRequest,
) -> Result<Vec<Vec<f32>>> {
    let paths = resolve_wav2vec2_bundle_paths(bundle)?;
    let _config = parse_wav2vec2_config(&paths.config_json)?;
    let vocab = parse_ctc_vocabulary(&paths.tokenizer_json)?;
    let transcript_text = request.transcript.text.clone().unwrap_or_else(|| {
        request
            .transcript
            .segments
            .iter()
            .map(|segment| segment.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    });
    let _token_ids = normalized_text_to_token_ids(&transcript_text, &vocab)?;
    let _ = paths.preprocessor_config_json;
    let _ = paths.model_safetensors;
    Err(unsupported_runtime(WAV2VEC2_UNSUPPORTED_MESSAGE))
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

pub(crate) fn parse_wav2vec2_config(path: &Path) -> Result<Wav2Vec2ConfigSummary> {
    let bytes = std::fs::read(path).map_err(|error| {
        crate::setup_error(format!(
            "failed to read wav2vec2 config `{}`: {error}",
            path.display()
        ))
    })?;
    let raw: RawWav2Vec2Config = serde_json::from_slice(&bytes).map_err(|error| {
        invalid_request(format!(
            "failed to parse wav2vec2 config `{}`: {error}",
            path.display()
        ))
    })?;
    if let Some(model_type) = raw.model_type.as_deref() {
        if model_type != "wav2vec2" {
            return Err(unsupported_runtime(format!(
                "unsupported CTC alignment model type `{model_type}`; expected wav2vec2"
            )));
        }
    }
    Ok(Wav2Vec2ConfigSummary {
        model_type: raw.model_type,
        architectures: raw.architectures,
        vocab_size: raw.vocab_size,
        word_delimiter_token: raw.word_delimiter_token,
    })
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

fn token_id(tokens: &[String], token: &str) -> Option<usize> {
    tokens.iter().position(|candidate| candidate == token)
}

#[cfg(test)]
mod tests {
    use super::*;
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
                "word_delimiter_token": "|"
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
    fn emit_validates_then_reports_typed_runtime_gap() {
        let temp = tempfile::tempdir().unwrap();
        write_valid_bundle(temp.path(), false);
        let mut segment = TranscriptSegmentContract::new(0, "hello world");
        segment.start_seconds = Some(0.0);
        segment.end_seconds = Some(1.0);
        let transcript =
            TranscriptionContract::from_segments(None, Some("en".to_string()), vec![segment])
                .unwrap();
        let request = AlignmentRequest {
            audio: crate::LoadedAudio {
                samples: vec![0.0; 16_000],
                sample_rate: 16_000,
                channels: 1,
                source: None,
            },
            transcript,
            language: Some("en".to_string()),
            model_id: "facebook/wav2vec2-base-960h".to_string(),
        };
        let error = emit_wav2vec2_ctc(temp.path(), &request)
            .unwrap_err()
            .to_string();
        assert!(error.contains("unsupported_runtime"));
        assert!(error.contains("candle-transformers 0.10.2"));
    }
}
