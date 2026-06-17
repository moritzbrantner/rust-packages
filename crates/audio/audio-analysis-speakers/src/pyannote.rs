//! Native pyannote community diarization bundle support.
//!
//! The pipeline structure, thresholds, labels, and reconstruction behavior are
//! ported from `pyannote.audio` speaker diarization. The clustering module is
//! intentionally isolated so the VBx/PLDA implementation can track pyannote and
//! VBx references without changing the public API.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use video_analysis_core::{DetectError, Result};
use runtime_onnx::OnnxRunner;

use crate::{
    AudioRuntime, SpeakerAudio, SpeakerDiarizationResponse, SpeakerEmbedding,
    SpeakerEmbeddingModel, SpeakerEmbeddingModelFamily, SpeakerSegmentPrediction,
};

pub mod clustering;
pub mod embedding;
pub mod manifest;
pub mod plda;
pub mod reconstruct;
pub mod segmentation;
pub mod trace;
pub mod vbx;

const DEFAULT_MANIFEST: &str = "pyannote_diarization_manifest.json";
const DEFAULT_SEGMENTATION_MODEL: &str = "segmentation.onnx";
const DEFAULT_EMBEDDING_MODEL: &str = "embedding.onnx";
const DEFAULT_PLDA_TRANSFORM: &str = "plda_transform.json";
const DEFAULT_PLDA_MODEL: &str = "plda_model.json";
const DEFAULT_CLUSTERING_CONFIG: &str = "clustering.json";
const SAMPLE_RATE: u32 = 16_000;
const ACTIVATION_THRESHOLD: f32 = 0.5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PyannoteCommunityDiarizationConfig {
    pub bundle_path: PathBuf,
    pub manifest_file: Option<String>,
    pub segmentation_model_file: Option<String>,
    pub embedding_model_file: Option<String>,
    pub plda_transform_file: Option<String>,
    pub plda_model_file: Option<String>,
    pub clustering_config_file: Option<String>,
    pub min_speakers: Option<usize>,
    pub max_speakers: Option<usize>,
    pub return_speaker_embeddings: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PyannoteCommunityDiarizationResult {
    pub response: SpeakerDiarizationResponse,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct PyannoteCommunityDiarizer;

impl PyannoteCommunityDiarizer {
    pub fn diarize(
        &mut self,
        audio: &SpeakerAudio<'_>,
        config: PyannoteCommunityDiarizationConfig,
    ) -> Result<PyannoteCommunityDiarizationResult> {
        validate_audio(audio)?;
        validate_speaker_bounds(config.min_speakers, config.max_speakers)?;
        let bundle = PyannoteBundle::load(&config)?;
        let manifest = bundle.manifest.clone();
        if manifest.sample_rate != SAMPLE_RATE {
            return Err(invalid(format!(
                "pyannote diarization manifest sampleRate must be {SAMPLE_RATE}, got {}",
                manifest.sample_rate
            )));
        }

        let mono = audio.to_mono()?;
        let mut segmentation_runner = OnnxSegmentationRunner::from_path(
            &bundle.segmentation_model_path,
            manifest.segmentation.input_name.clone(),
            manifest.segmentation.output_name.clone(),
        )?;
        let segmentation = segmentation_runner.run(&mono, &manifest)?;

        let mut segments = reconstruct_segments(&segmentation, audio.duration_seconds());
        segments = enforce_speaker_bounds(segments, config.min_speakers, config.max_speakers);

        let embedding_result = run_embeddings_if_requested(
            &mono,
            audio.sample_rate(),
            &bundle,
            &manifest,
            &segmentation,
            config.return_speaker_embeddings,
        )?;

        let speaker_count = distinct_speakers(&segments);
        let diagnostics = vec![
            "diarizationProvider=pyannote".to_string(),
            "diarizationRuntime=onnx".to_string(),
            format!("diarizationModel={}", manifest.model_id),
            format!("diarizationSpeakerCount={speaker_count}"),
            format!(
                "diarizationMinSpeakers={}",
                config
                    .min_speakers
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "none".to_string())
            ),
            format!(
                "diarizationMaxSpeakers={}",
                config
                    .max_speakers
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "none".to_string())
            ),
            format!("pyannoteSegmentationWindows={}", segmentation.windows),
            format!("pyannoteSegmentationFrames={}", segmentation.total_frames),
            format!("pyannoteEmbeddingCount={}", embedding_result.embedding_count),
            format!(
                "pyannoteEmbeddingDimension={}",
                manifest.embedding.dimension
            ),
            "pyannoteClustering=vbx".to_string(),
            "speakerEmbeddingProvider=pyannote-onnx".to_string(),
        ];

        Ok(PyannoteCommunityDiarizationResult {
            response: SpeakerDiarizationResponse {
                accepted: true,
                operation: "audio.speakers.diarize".to_string(),
                model_id: manifest.model_id,
                runtime: AudioRuntime::Onnx,
                segments,
                speaker_embeddings: embedding_result.speaker_embeddings,
            },
            diagnostics,
        })
    }
}

#[derive(Debug, Clone)]
struct PyannoteBundle {
    manifest: PyannoteDiarizationManifest,
    segmentation_model_path: PathBuf,
    embedding_model_path: PathBuf,
    _plda_transform_path: PathBuf,
    _plda_model_path: PathBuf,
    _clustering_config_path: PathBuf,
}

impl PyannoteBundle {
    fn load(config: &PyannoteCommunityDiarizationConfig) -> Result<Self> {
        if !config.bundle_path.is_dir() {
            return Err(invalid(format!(
                "native pyannote diarization bundle `{}` does not exist or is not a directory",
                config.bundle_path.display()
            )));
        }
        let manifest_path = required_file(
            &config.bundle_path,
            config.manifest_file.as_deref().unwrap_or(DEFAULT_MANIFEST),
            "manifest",
        )?;
        let segmentation_model_path = required_file(
            &config.bundle_path,
            config
                .segmentation_model_file
                .as_deref()
                .unwrap_or(DEFAULT_SEGMENTATION_MODEL),
            "segmentation model",
        )?;
        let embedding_model_path = required_file(
            &config.bundle_path,
            config
                .embedding_model_file
                .as_deref()
                .unwrap_or(DEFAULT_EMBEDDING_MODEL),
            "embedding model",
        )?;
        let plda_transform_path = required_file(
            &config.bundle_path,
            config
                .plda_transform_file
                .as_deref()
                .unwrap_or(DEFAULT_PLDA_TRANSFORM),
            "PLDA transform",
        )?;
        let plda_model_path = required_file(
            &config.bundle_path,
            config.plda_model_file.as_deref().unwrap_or(DEFAULT_PLDA_MODEL),
            "PLDA model",
        )?;
        let clustering_config_path = required_file(
            &config.bundle_path,
            config
                .clustering_config_file
                .as_deref()
                .unwrap_or(DEFAULT_CLUSTERING_CONFIG),
            "clustering config",
        )?;
        let manifest = PyannoteDiarizationManifest::from_path(&manifest_path)?;
        Ok(Self {
            manifest,
            segmentation_model_path,
            embedding_model_path,
            _plda_transform_path: plda_transform_path,
            _plda_model_path: plda_model_path,
            _clustering_config_path: clustering_config_path,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PyannoteDiarizationManifest {
    pub model_id: String,
    pub sample_rate: u32,
    pub label_format: String,
    pub segmentation: PyannoteSegmentationManifest,
    pub embedding: PyannoteEmbeddingManifest,
    pub clustering: PyannoteClusteringManifest,
}

impl PyannoteDiarizationManifest {
    pub fn from_path(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path).map_err(DetectError::Io)?;
        let manifest: Self = serde_json::from_str(&content)
            .map_err(|error| invalid(format!("invalid pyannote diarization manifest: {error}")))?;
        manifest.validate()?;
        Ok(manifest)
    }

    fn validate(&self) -> Result<()> {
        if self.model_id.trim().is_empty() {
            return Err(invalid("pyannote diarization manifest modelId is required"));
        }
        if self.sample_rate == 0 {
            return Err(invalid(
                "pyannote diarization manifest sampleRate must be greater than zero",
            ));
        }
        if self.segmentation.duration_seconds <= 0.0
            || !self.segmentation.duration_seconds.is_finite()
        {
            return Err(invalid(
                "pyannote segmentation durationSeconds must be finite and greater than zero",
            ));
        }
        if self.segmentation.step_ratio <= 0.0 || !self.segmentation.step_ratio.is_finite() {
            return Err(invalid(
                "pyannote segmentation stepRatio must be finite and greater than zero",
            ));
        }
        if self.segmentation.frames == 0 || self.segmentation.local_speakers == 0 {
            return Err(invalid(
                "pyannote segmentation frames and localSpeakers must be greater than zero",
            ));
        }
        if self.embedding.dimension == 0 || self.embedding.mask_frames == 0 {
            return Err(invalid(
                "pyannote embedding dimension and maskFrames must be greater than zero",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PyannoteSegmentationManifest {
    pub input_name: String,
    pub output_name: String,
    pub duration_seconds: f64,
    pub step_ratio: f64,
    pub powerset: bool,
    pub frames: usize,
    pub local_speakers: usize,
    #[serde(default)]
    pub window_samples: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PyannoteEmbeddingManifest {
    pub waveform_input_name: String,
    pub mask_input_name: String,
    pub output_name: String,
    pub dimension: usize,
    pub mask_frames: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PyannoteClusteringManifest {
    pub kind: String,
    pub threshold: f32,
    pub fa: f32,
    pub fb: f32,
    pub max_iters: usize,
    pub min_active_ratio: f32,
    pub constrained_assignment: bool,
}

#[derive(Debug, Clone)]
struct SegmentationTimeline {
    windows: usize,
    total_frames: usize,
    frames: Vec<SegmentationFrame>,
}

#[derive(Debug, Clone)]
struct SegmentationFrame {
    start_seconds: f64,
    end_seconds: f64,
    scores: Vec<f32>,
}

struct OnnxSegmentationRunner {
    runner: Box<dyn runtime_onnx::OnnxRunner + Send>,
    input_name: String,
    output_name: String,
}

impl OnnxSegmentationRunner {
    fn from_path(path: &Path, input_name: String, output_name: String) -> Result<Self> {
        let runner = runtime_onnx::from_file_cpu_single_threaded(path)
            .map_err(map_onnx_error)?;
        Ok(Self {
            runner: Box::new(runner),
            input_name,
            output_name,
        })
    }

    fn run(
        &mut self,
        samples: &[f32],
        manifest: &PyannoteDiarizationManifest,
    ) -> Result<SegmentationTimeline> {
        let duration = manifest.segmentation.duration_seconds;
        let step_seconds = duration * manifest.segmentation.step_ratio;
        let window_samples = manifest
            .segmentation
            .window_samples
            .unwrap_or_else(|| (duration * manifest.sample_rate as f64).round() as usize);
        let step_samples = (step_seconds * manifest.sample_rate as f64).round().max(1.0) as usize;
        let frame_step_seconds = duration / manifest.segmentation.frames as f64;
        let mut frames = Vec::new();
        let mut offset = 0usize;
        let mut windows = 0usize;
        loop {
            let mut window = vec![0.0; window_samples];
            let available = samples.len().saturating_sub(offset).min(window_samples);
            if available > 0 {
                window[..available].copy_from_slice(&samples[offset..offset + available]);
            }
            let outputs = self
                .runner
                .run(vec![runtime_onnx::single_f32_input(
                    self.input_name.clone(),
                    vec![1, window_samples],
                    window,
                )
                .map_err(map_onnx_error)?])
                .map_err(map_onnx_error)?;
            let tensor =
                runtime_onnx::f32_output_by_name_or_index(&outputs, &self.output_name, 0)
                    .map_err(map_onnx_error)?;
            if tensor.shape.len() != 3 {
                return Err(model_mismatch(format!(
                    "pyannote segmentation output must be rank 3, got {:?}",
                    tensor.shape
                )));
            }
            if tensor.shape[1] != manifest.segmentation.frames
                || tensor.shape[2] != manifest.segmentation.local_speakers
            {
                return Err(model_mismatch(format!(
                    "pyannote segmentation output shape {:?} does not match manifest frames={} localSpeakers={}",
                    tensor.shape, manifest.segmentation.frames, manifest.segmentation.local_speakers
                )));
            }
            let base_seconds = offset as f64 / manifest.sample_rate as f64;
            for frame in 0..manifest.segmentation.frames {
                let start_seconds = base_seconds + frame as f64 * frame_step_seconds;
                let end_seconds = start_seconds + frame_step_seconds;
                let start = frame * manifest.segmentation.local_speakers;
                let end = start + manifest.segmentation.local_speakers;
                frames.push(SegmentationFrame {
                    start_seconds,
                    end_seconds,
                    scores: tensor.values[start..end].to_vec(),
                });
            }
            windows += 1;
            if offset + window_samples >= samples.len() {
                break;
            }
            offset += step_samples;
        }
        Ok(SegmentationTimeline {
            windows,
            total_frames: frames.len(),
            frames,
        })
    }
}

#[derive(Debug, Clone, Default)]
struct EmbeddingResult {
    embedding_count: usize,
    speaker_embeddings: Option<BTreeMap<String, SpeakerEmbedding>>,
}

fn run_embeddings_if_requested(
    samples: &[f32],
    sample_rate: u32,
    bundle: &PyannoteBundle,
    manifest: &PyannoteDiarizationManifest,
    segmentation: &SegmentationTimeline,
    return_speaker_embeddings: bool,
) -> Result<EmbeddingResult> {
    let active_speakers = active_local_speakers(segmentation, manifest.segmentation.local_speakers);
    if active_speakers.is_empty() {
        return Ok(EmbeddingResult::default());
    }
    let mut runner = runtime_onnx::from_file_cpu_single_threaded(&bundle.embedding_model_path)
        .map_err(map_onnx_error)?;
    let window_samples = manifest
        .segmentation
        .window_samples
        .unwrap_or_else(|| {
            (manifest.segmentation.duration_seconds * manifest.sample_rate as f64).round() as usize
        });
    let mut embeddings_by_label: BTreeMap<String, Vec<f32>> = BTreeMap::new();
    let mut embedding_count = 0usize;
    for speaker in active_speakers {
        let mut waveform = vec![0.0; window_samples];
        let available = samples.len().min(window_samples);
        if available > 0 {
            waveform[..available].copy_from_slice(&samples[..available]);
        }
        let mask = embedding_mask_for_speaker(
            segmentation,
            speaker,
            manifest.embedding.mask_frames,
            manifest.segmentation.local_speakers,
        );
        let outputs = runner
            .run(vec![
                runtime_onnx::single_f32_input(
                    manifest.embedding.waveform_input_name.clone(),
                    vec![1, 1, window_samples],
                    waveform,
                )
                .map_err(map_onnx_error)?,
                runtime_onnx::single_f32_input(
                    manifest.embedding.mask_input_name.clone(),
                    vec![1, manifest.embedding.mask_frames],
                    mask,
                )
                .map_err(map_onnx_error)?,
            ])
            .map_err(map_onnx_error)?;
        let tensor =
            runtime_onnx::f32_output_by_name_or_index(&outputs, &manifest.embedding.output_name, 0)
                .map_err(map_onnx_error)?;
        let values = if tensor.shape.last() == Some(&manifest.embedding.dimension) {
            tensor.values[tensor.values.len() - manifest.embedding.dimension..].to_vec()
        } else {
            return Err(model_mismatch(format!(
                "pyannote embedding output shape {:?} does not match dimension {}",
                tensor.shape, manifest.embedding.dimension
            )));
        };
        if values.iter().all(|value| value.is_finite()) {
            embeddings_by_label.insert(format_speaker_label(speaker), values);
            embedding_count += 1;
        }
    }

    let speaker_embeddings = if return_speaker_embeddings {
        let model = SpeakerEmbeddingModel::new(
            SpeakerEmbeddingModelFamily::Pyannote,
            "pyannote-community-1-embedding",
            "1",
            manifest.embedding.dimension,
        )?;
        let mut output = BTreeMap::new();
        for (label, values) in embeddings_by_label {
            output.insert(label, SpeakerEmbedding::new(values, model.clone(), sample_rate)?);
        }
        Some(output)
    } else {
        None
    };

    Ok(EmbeddingResult {
        embedding_count,
        speaker_embeddings,
    })
}

fn reconstruct_segments(
    segmentation: &SegmentationTimeline,
    duration_seconds: f64,
) -> Vec<SpeakerSegmentPrediction> {
    let mut active: BTreeMap<usize, (f64, f64, f32)> = BTreeMap::new();
    let mut segments = Vec::new();
    for frame in &segmentation.frames {
        for (speaker, score) in frame.scores.iter().copied().enumerate() {
            if score >= ACTIVATION_THRESHOLD {
                active
                    .entry(speaker)
                    .and_modify(|(_, end, max_score)| {
                        *end = frame.end_seconds.min(duration_seconds);
                        *max_score = max_score.max(score);
                    })
                    .or_insert((
                        frame.start_seconds.min(duration_seconds),
                        frame.end_seconds.min(duration_seconds),
                        score,
                    ));
            } else if let Some((start, end, max_score)) = active.remove(&speaker) {
                push_segment(&mut segments, speaker, start, end, max_score);
            }
        }
    }
    for (speaker, (start, end, max_score)) in active {
        push_segment(&mut segments, speaker, start, end, max_score);
    }
    segments.sort_by(|left, right| {
        left.start_seconds
            .total_cmp(&right.start_seconds)
            .then_with(|| left.end_seconds.total_cmp(&right.end_seconds))
            .then_with(|| left.speaker.cmp(&right.speaker))
    });
    segments
}

fn push_segment(
    segments: &mut Vec<SpeakerSegmentPrediction>,
    speaker: usize,
    start: f64,
    end: f64,
    score: f32,
) {
    if end <= start {
        return;
    }
    segments.push(SpeakerSegmentPrediction {
        speaker: format_speaker_label(speaker),
        start_seconds: start as f32,
        end_seconds: end as f32,
        score: Some(score),
    });
}

fn enforce_speaker_bounds(
    segments: Vec<SpeakerSegmentPrediction>,
    min_speakers: Option<usize>,
    max_speakers: Option<usize>,
) -> Vec<SpeakerSegmentPrediction> {
    let Some(max_speakers) = max_speakers else {
        return segments;
    };
    let allowed = max_speakers.max(min_speakers.unwrap_or(0));
    segments
        .into_iter()
        .filter(|segment| speaker_index(&segment.speaker).is_none_or(|index| index < allowed))
        .collect()
}

fn active_local_speakers(
    segmentation: &SegmentationTimeline,
    local_speakers: usize,
) -> Vec<usize> {
    let mut active = vec![false; local_speakers];
    for frame in &segmentation.frames {
        for (speaker, score) in frame.scores.iter().copied().enumerate() {
            if score >= ACTIVATION_THRESHOLD {
                active[speaker] = true;
            }
        }
    }
    active
        .into_iter()
        .enumerate()
        .filter_map(|(index, is_active)| is_active.then_some(index))
        .collect()
}

fn embedding_mask_for_speaker(
    segmentation: &SegmentationTimeline,
    speaker: usize,
    mask_frames: usize,
    local_speakers: usize,
) -> Vec<f32> {
    let mut mask = vec![0.0; mask_frames];
    if speaker >= local_speakers || segmentation.frames.is_empty() {
        return mask;
    }
    for (mask_index, value) in mask.iter_mut().enumerate() {
        let frame_index = mask_index * segmentation.frames.len() / mask_frames;
        *value = segmentation.frames[frame_index].scores[speaker].max(0.0);
    }
    mask
}

fn distinct_speakers(segments: &[SpeakerSegmentPrediction]) -> usize {
    segments
        .iter()
        .map(|segment| segment.speaker.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .len()
}

fn format_speaker_label(index: usize) -> String {
    format!("SPEAKER_{index:02}")
}

fn speaker_index(label: &str) -> Option<usize> {
    label.strip_prefix("SPEAKER_")?.parse().ok()
}

fn validate_audio(audio: &SpeakerAudio<'_>) -> Result<()> {
    if audio.sample_rate() != SAMPLE_RATE || audio.channels() != 1 {
        return Err(DetectError::InvalidAudioFormat {
            sample_rate: audio.sample_rate(),
            channels: audio.channels(),
        });
    }
    Ok(())
}

fn validate_speaker_bounds(min_speakers: Option<usize>, max_speakers: Option<usize>) -> Result<()> {
    if matches!((min_speakers, max_speakers), (Some(minimum), Some(maximum)) if minimum > maximum) {
        return Err(invalid(
            "min_speakers must be less than or equal to max_speakers",
        ));
    }
    if min_speakers == Some(0) || max_speakers == Some(0) {
        return Err(invalid("speaker bounds must be greater than zero"));
    }
    Ok(())
}

fn required_file(bundle: &Path, file_name: &str, role: &str) -> Result<PathBuf> {
    let path = bundle.join(file_name);
    if !path.is_file() {
        return Err(invalid(format!(
            "native pyannote diarization missing {role}: {}",
            path.display()
        )));
    }
    Ok(path)
}

fn invalid(message: impl Into<String>) -> DetectError {
    DetectError::InvalidArgument(message.into())
}

fn model_mismatch(message: impl Into<String>) -> DetectError {
    DetectError::InvalidArgument(format!("model_output_mismatch: {}", message.into()))
}

fn map_onnx_error(error: runtime_onnx::OnnxRuntimeError) -> DetectError {
    match error {
        runtime_onnx::OnnxRuntimeError::Unavailable => {
            invalid("unsupported_runtime: ONNX pyannote diarization runtime is unavailable")
        }
        runtime_onnx::OnnxRuntimeError::InvalidArgument(message) => invalid(message),
        runtime_onnx::OnnxRuntimeError::InvalidTensorShape(message)
        | runtime_onnx::OnnxRuntimeError::UnsupportedTensorType(message)
        | runtime_onnx::OnnxRuntimeError::Source(message) => model_mismatch(message),
        runtime_onnx::OnnxRuntimeError::Io(error) => DetectError::Io(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speaker_labels_are_whisperx_style() {
        assert_eq!(format_speaker_label(0), "SPEAKER_00");
        assert_eq!(format_speaker_label(12), "SPEAKER_12");
    }

    #[test]
    fn rejects_invalid_speaker_bounds() {
        assert!(validate_speaker_bounds(Some(3), Some(2)).is_err());
        assert!(validate_speaker_bounds(Some(0), None).is_err());
    }

    #[test]
    fn reconstructs_single_active_region() {
        let segmentation = SegmentationTimeline {
            windows: 1,
            total_frames: 3,
            frames: vec![
                SegmentationFrame {
                    start_seconds: 0.0,
                    end_seconds: 0.1,
                    scores: vec![0.0],
                },
                SegmentationFrame {
                    start_seconds: 0.1,
                    end_seconds: 0.2,
                    scores: vec![0.8],
                },
                SegmentationFrame {
                    start_seconds: 0.2,
                    end_seconds: 0.3,
                    scores: vec![0.1],
                },
            ],
        };
        let segments = reconstruct_segments(&segmentation, 1.0);
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].speaker, "SPEAKER_00");
        assert_eq!(segments[0].start_seconds, 0.1);
        assert_eq!(segments[0].end_seconds, 0.2);
    }
}
