//! Native pyannote community diarization support.
//!
//! The pipeline shape and defaults mirror `pyannote.audio` community diarization
//! behavior: segmentation windows, speaker masks for embedding extraction,
//! speaker-bound validation, and `SPEAKER_XX` labels. The ONNX model artifacts
//! are caller supplied and are intentionally not bundled in this crate.

pub mod clustering;
pub mod embedding;
pub mod manifest;
pub mod plda;
pub mod reconstruct;
pub mod segmentation;
pub mod trace;
pub mod vbx;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use runtime_onnx::{
    f32_output_by_name_or_index, first_f32_output, single_f32_input, OnnxF32Tensor, OnnxRunner,
    OnnxSession,
};
use serde::{Deserialize, Serialize};
use video_analysis_core::{DetectError, Result};

use crate::{
    AudioRuntime, SpeakerAudio, SpeakerDiarizationResponse, SpeakerEmbedding,
    SpeakerEmbeddingModel, SpeakerEmbeddingModelFamily, SpeakerSegmentPrediction,
};

const SAMPLE_RATE: u32 = 16_000;
const DEFAULT_MANIFEST_FILE: &str = "pyannote_diarization_manifest.json";
const DEFAULT_SEGMENTATION_MODEL_FILE: &str = "segmentation.onnx";
const DEFAULT_EMBEDDING_MODEL_FILE: &str = "embedding.onnx";
const DEFAULT_PLDA_TRANSFORM_FILE: &str = "plda_transform.json";
const DEFAULT_PLDA_MODEL_FILE: &str = "plda_model.json";
const DEFAULT_CLUSTERING_CONFIG_FILE: &str = "clustering.json";
const DEFAULT_MODEL_ID: &str = "pyannote/speaker-diarization-community-1";
const DEFAULT_LABEL_FORMAT: &str = "SPEAKER_{:02}";
const ACTIVE_THRESHOLD: f32 = 0.5;

/// Native pyannote community diarizer configuration.
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

/// Native pyannote diarization output plus stable diagnostics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PyannoteCommunityDiarizationResult {
    pub response: SpeakerDiarizationResponse,
    pub diagnostics: Vec<String>,
}

/// ONNX-backed pyannote community diarizer.
#[derive(Debug)]
pub struct PyannoteCommunityDiarizer {
    manifest: PyannoteDiarizationManifest,
    segmentation: OnnxSession,
    embedding: OnnxSession,
    config: ResolvedPyannoteConfig,
}

#[derive(Debug, Clone)]
struct ResolvedPyannoteConfig {
    bundle_path: PathBuf,
    segmentation_model_path: PathBuf,
    embedding_model_path: PathBuf,
    plda_transform_path: PathBuf,
    plda_model_path: PathBuf,
    clustering_config_path: PathBuf,
    min_speakers: Option<usize>,
    max_speakers: Option<usize>,
    return_speaker_embeddings: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PyannoteDiarizationManifest {
    #[serde(default = "default_model_id")]
    model_id: String,
    #[serde(default = "default_sample_rate")]
    sample_rate: u32,
    #[serde(default = "default_label_format")]
    label_format: String,
    segmentation: PyannoteSegmentationManifest,
    embedding: PyannoteEmbeddingManifest,
    clustering: PyannoteClusteringManifest,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PyannoteSegmentationManifest {
    #[serde(default = "default_segmentation_input_name")]
    input_name: String,
    #[serde(default = "default_segmentation_output_name")]
    output_name: String,
    duration_seconds: f64,
    #[serde(default = "default_step_ratio")]
    step_ratio: f64,
    #[serde(default = "default_true")]
    powerset: bool,
    frames: usize,
    local_speakers: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PyannoteEmbeddingManifest {
    #[serde(default = "default_embedding_waveform_input_name")]
    waveform_input_name: String,
    #[serde(default = "default_embedding_mask_input_name")]
    mask_input_name: String,
    #[serde(default = "default_embedding_output_name")]
    output_name: String,
    dimension: usize,
    mask_frames: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PyannoteClusteringManifest {
    #[serde(default = "default_clustering_kind")]
    kind: String,
    #[serde(default = "default_clustering_threshold")]
    threshold: f32,
    #[serde(default = "default_fa")]
    fa: f32,
    #[serde(default = "default_fb")]
    fb: f32,
    #[serde(default = "default_vbx_iters")]
    max_iters: usize,
    #[serde(default = "default_min_active_ratio")]
    min_active_ratio: f32,
    #[serde(default = "default_true")]
    constrained_assignment: bool,
}

#[derive(Debug, Clone)]
struct SegmentationBatch {
    windows: Vec<SegmentationWindow>,
    total_frames: usize,
}

#[derive(Debug, Clone)]
struct SegmentationWindow {
    start_seconds: f64,
    samples: Vec<f32>,
    scores: Vec<f32>,
}

#[derive(Debug, Clone)]
struct LocalSpeakerEmbedding {
    chunk: usize,
    local_speaker: usize,
    active_frames: usize,
    clean_frames: usize,
    values: Vec<f32>,
}

#[derive(Debug, Clone)]
struct AssignedLocalSpeaker {
    chunk: usize,
    local_speaker: usize,
    speaker: usize,
    score: f32,
}

impl PyannoteCommunityDiarizer {
    /// Builds a pyannote community diarizer from a local ONNX bundle.
    pub fn from_config(config: PyannoteCommunityDiarizationConfig) -> Result<Self> {
        let resolved = resolve_config(config)?;
        let manifest = load_manifest(&resolved.bundle_path, DEFAULT_MANIFEST_FILE)?;
        validate_manifest(&manifest)?;
        let segmentation =
            OnnxSession::from_file_cpu_single_threaded(&resolved.segmentation_model_path)
                .map_err(map_onnx_session_error)?;
        let embedding = OnnxSession::from_file_cpu_single_threaded(&resolved.embedding_model_path)
            .map_err(map_onnx_session_error)?;
        Ok(Self {
            manifest,
            segmentation,
            embedding,
            config: resolved,
        })
    }

    /// Runs diarization over 16 kHz mono audio.
    pub fn diarize(
        &mut self,
        audio: &SpeakerAudio<'_>,
    ) -> Result<PyannoteCommunityDiarizationResult> {
        if audio.sample_rate() != SAMPLE_RATE || audio.channels() != 1 {
            return Err(DetectError::InvalidArgument(format!(
                "pyannote diarization requires {SAMPLE_RATE} Hz mono audio, got sample_rate={} channels={}",
                audio.sample_rate(),
                audio.channels()
            )));
        }
        validate_speaker_bounds(self.config.min_speakers, self.config.max_speakers)?;

        let samples = audio.samples();
        let segmentations = self.run_segmentations(samples)?;
        let embeddings = self.run_embeddings(&segmentations)?;
        let assignments = assign_local_speakers(
            &embeddings,
            self.config.min_speakers,
            self.config.max_speakers,
            self.manifest.clustering.threshold,
        )?;
        let segments = reconstruct_segments(&self.manifest, &segmentations, &assignments)?;
        let speaker_count = segments
            .iter()
            .map(|segment| segment.speaker.clone())
            .collect::<BTreeSet<_>>()
            .len();
        let speaker_embeddings = if self.config.return_speaker_embeddings {
            Some(speaker_embedding_map(
                &self.manifest,
                &embeddings,
                &assignments,
            )?)
        } else {
            None
        };
        let diagnostics = vec![
            "diarizationProvider=pyannote".to_string(),
            "diarizationRuntime=onnx".to_string(),
            format!("diarizationModel={}", self.manifest.model_id),
            format!("diarizationSpeakerCount={speaker_count}"),
            format!(
                "diarizationMinSpeakers={}",
                format_optional_usize(self.config.min_speakers)
            ),
            format!(
                "diarizationMaxSpeakers={}",
                format_optional_usize(self.config.max_speakers)
            ),
            format!(
                "pyannoteSegmentationWindows={}",
                segmentations.windows.len()
            ),
            format!("pyannoteSegmentationFrames={}", segmentations.total_frames),
            format!("pyannoteEmbeddingCount={}", embeddings.len()),
            format!(
                "pyannoteEmbeddingDimension={}",
                self.manifest.embedding.dimension
            ),
            "pyannoteClustering=vbx".to_string(),
            "speakerEmbeddingProvider=pyannote-onnx".to_string(),
            format!(
                "pyannoteSegmentationModel={}",
                self.config.segmentation_model_path.display()
            ),
            format!(
                "pyannoteEmbeddingModel={}",
                self.config.embedding_model_path.display()
            ),
            format!(
                "pyannotePldaTransform={}",
                self.config.plda_transform_path.display()
            ),
            format!(
                "pyannotePldaModel={}",
                self.config.plda_model_path.display()
            ),
            format!(
                "pyannoteClusteringConfig={}",
                self.config.clustering_config_path.display()
            ),
        ];
        Ok(PyannoteCommunityDiarizationResult {
            response: SpeakerDiarizationResponse {
                accepted: true,
                operation: "audio.speakers.diarize".to_string(),
                model_id: self.manifest.model_id.clone(),
                runtime: AudioRuntime::Onnx,
                segments,
                speaker_embeddings,
                diagnostics: diagnostics.clone(),
            },
            diagnostics,
        })
    }

    fn run_segmentations(&mut self, samples: &[f32]) -> Result<SegmentationBatch> {
        let window_samples =
            seconds_to_samples(self.manifest.segmentation.duration_seconds, SAMPLE_RATE)?;
        let step_samples = seconds_to_samples(
            self.manifest.segmentation.duration_seconds * self.manifest.segmentation.step_ratio,
            SAMPLE_RATE,
        )?
        .max(1);
        let mut windows = Vec::new();
        let mut start = 0_usize;
        loop {
            let mut values = vec![0.0_f32; window_samples];
            let end = (start + window_samples).min(samples.len());
            if start < samples.len() {
                values[..end - start].copy_from_slice(&samples[start..end]);
            }
            let tensor = single_f32_input(
                self.manifest.segmentation.input_name.clone(),
                vec![1, window_samples],
                values.clone(),
            )
            .map_err(map_onnx_runtime_error)?;
            let outputs = self
                .segmentation
                .run(vec![tensor])
                .map_err(map_onnx_runtime_error)?;
            let output =
                f32_output_by_name_or_index(&outputs, &self.manifest.segmentation.output_name, 0)
                    .or_else(|_| first_f32_output(&outputs))
                    .map_err(map_onnx_runtime_error)?;
            let scores = segmentation_scores(output, &self.manifest.segmentation)?;
            windows.push(SegmentationWindow {
                start_seconds: start as f64 / SAMPLE_RATE as f64,
                samples: values,
                scores,
            });
            if end >= samples.len() {
                break;
            }
            start += step_samples;
        }
        Ok(SegmentationBatch {
            total_frames: windows.len() * self.manifest.segmentation.frames,
            windows,
        })
    }

    fn run_embeddings(
        &mut self,
        segmentations: &SegmentationBatch,
    ) -> Result<Vec<LocalSpeakerEmbedding>> {
        let mut embeddings = Vec::new();
        for (chunk, window) in segmentations.windows.iter().enumerate() {
            for local_speaker in 0..self.manifest.segmentation.local_speakers {
                let mask = speaker_mask(&self.manifest, window, local_speaker);
                let active_frames = mask
                    .iter()
                    .filter(|value| **value > ACTIVE_THRESHOLD)
                    .count();
                let clean_frames = clean_speaker_frames(&self.manifest, window, local_speaker);
                if active_frames == 0 {
                    continue;
                }
                let waveform = single_f32_input(
                    self.manifest.embedding.waveform_input_name.clone(),
                    vec![1, 1, window.samples.len()],
                    window.samples.clone(),
                )
                .map_err(map_onnx_runtime_error)?;
                let masks = single_f32_input(
                    self.manifest.embedding.mask_input_name.clone(),
                    vec![1, self.manifest.embedding.mask_frames],
                    mask,
                )
                .map_err(map_onnx_runtime_error)?;
                let outputs = self
                    .embedding
                    .run(vec![waveform, masks])
                    .map_err(map_onnx_runtime_error)?;
                let output =
                    f32_output_by_name_or_index(&outputs, &self.manifest.embedding.output_name, 0)
                        .or_else(|_| first_f32_output(&outputs))
                        .map_err(map_onnx_runtime_error)?;
                let values = embedding_values(output, self.manifest.embedding.dimension)?;
                if values.iter().all(|value| value.is_finite()) {
                    embeddings.push(LocalSpeakerEmbedding {
                        chunk,
                        local_speaker,
                        active_frames,
                        clean_frames,
                        values,
                    });
                }
            }
        }
        Ok(embeddings)
    }
}

fn resolve_config(config: PyannoteCommunityDiarizationConfig) -> Result<ResolvedPyannoteConfig> {
    validate_speaker_bounds(config.min_speakers, config.max_speakers)?;
    let bundle_path = config.bundle_path;
    if !bundle_path.is_dir() {
        return Err(setup_error(format!(
            "pyannote diarization bundle `{}` does not exist or is not a directory",
            bundle_path.display()
        )));
    }
    let manifest_file = config
        .manifest_file
        .as_deref()
        .unwrap_or(DEFAULT_MANIFEST_FILE);
    let segmentation_model_file = config
        .segmentation_model_file
        .as_deref()
        .unwrap_or(DEFAULT_SEGMENTATION_MODEL_FILE);
    let embedding_model_file = config
        .embedding_model_file
        .as_deref()
        .unwrap_or(DEFAULT_EMBEDDING_MODEL_FILE);
    let plda_transform_file = config
        .plda_transform_file
        .as_deref()
        .unwrap_or(DEFAULT_PLDA_TRANSFORM_FILE);
    let plda_model_file = config
        .plda_model_file
        .as_deref()
        .unwrap_or(DEFAULT_PLDA_MODEL_FILE);
    let clustering_config_file = config
        .clustering_config_file
        .as_deref()
        .unwrap_or(DEFAULT_CLUSTERING_CONFIG_FILE);
    require_file(&bundle_path.join(manifest_file), "manifest")?;
    let segmentation_model_path = require_file(
        &bundle_path.join(segmentation_model_file),
        "segmentation model",
    )?;
    let embedding_model_path =
        require_file(&bundle_path.join(embedding_model_file), "embedding model")?;
    let plda_transform_path =
        require_file(&bundle_path.join(plda_transform_file), "PLDA transform")?;
    let plda_model_path = require_file(&bundle_path.join(plda_model_file), "PLDA model")?;
    let clustering_config_path = require_file(
        &bundle_path.join(clustering_config_file),
        "clustering config",
    )?;
    Ok(ResolvedPyannoteConfig {
        bundle_path,
        segmentation_model_path,
        embedding_model_path,
        plda_transform_path,
        plda_model_path,
        clustering_config_path,
        min_speakers: config.min_speakers,
        max_speakers: config.max_speakers,
        return_speaker_embeddings: config.return_speaker_embeddings,
    })
}

fn require_file(path: &Path, role: &str) -> Result<PathBuf> {
    if path.is_file() {
        Ok(path.to_path_buf())
    } else {
        Err(setup_error(format!(
            "pyannote diarization {role} `{}` does not exist or is not a file",
            path.display()
        )))
    }
}

fn load_manifest(bundle_path: &Path, file_name: &str) -> Result<PyannoteDiarizationManifest> {
    let path = bundle_path.join(file_name);
    let text = fs::read_to_string(&path).map_err(|error| {
        setup_error(format!(
            "failed to read pyannote diarization manifest `{}`: {error}",
            path.display()
        ))
    })?;
    serde_json::from_str(&text).map_err(|error| {
        DetectError::InvalidArgument(format!(
            "invalid_request: failed to parse pyannote diarization manifest `{}`: {error}",
            path.display()
        ))
    })
}

fn validate_manifest(manifest: &PyannoteDiarizationManifest) -> Result<()> {
    if manifest.sample_rate != SAMPLE_RATE {
        return Err(DetectError::InvalidArgument(format!(
            "invalid_request: pyannote diarization manifest sampleRate must be {SAMPLE_RATE}, got {}",
            manifest.sample_rate
        )));
    }
    if manifest.model_id.trim().is_empty() {
        return Err(DetectError::InvalidArgument(
            "invalid_request: pyannote diarization manifest modelId must not be empty".to_string(),
        ));
    }
    if manifest.segmentation.duration_seconds <= 0.0
        || !manifest.segmentation.duration_seconds.is_finite()
        || manifest.segmentation.step_ratio <= 0.0
        || !manifest.segmentation.step_ratio.is_finite()
        || manifest.segmentation.frames == 0
        || manifest.segmentation.local_speakers == 0
    {
        return Err(DetectError::InvalidArgument(
            "invalid_request: pyannote diarization segmentation manifest values are invalid"
                .to_string(),
        ));
    }
    if manifest.embedding.dimension == 0 || manifest.embedding.mask_frames == 0 {
        return Err(DetectError::InvalidArgument(
            "invalid_request: pyannote diarization embedding manifest values are invalid"
                .to_string(),
        ));
    }
    Ok(())
}

fn validate_speaker_bounds(min_speakers: Option<usize>, max_speakers: Option<usize>) -> Result<()> {
    if min_speakers == Some(0) {
        return Err(DetectError::InvalidArgument(
            "invalid_request: pyannote diarization min_speakers must be greater than zero"
                .to_string(),
        ));
    }
    if max_speakers == Some(0) {
        return Err(DetectError::InvalidArgument(
            "invalid_request: pyannote diarization max_speakers must be greater than zero"
                .to_string(),
        ));
    }
    if let (Some(min), Some(max)) = (min_speakers, max_speakers) {
        if min > max {
            return Err(DetectError::InvalidArgument(
                "invalid_request: pyannote diarization min_speakers must be less than or equal to max_speakers"
                    .to_string(),
            ));
        }
    }
    Ok(())
}

fn seconds_to_samples(seconds: f64, sample_rate: u32) -> Result<usize> {
    if seconds <= 0.0 || !seconds.is_finite() {
        return Err(DetectError::InvalidArgument(
            "invalid_request: pyannote diarization window seconds must be finite and positive"
                .to_string(),
        ));
    }
    Ok((seconds * sample_rate as f64).round() as usize)
}

fn segmentation_scores(
    tensor: &OnnxF32Tensor,
    manifest: &PyannoteSegmentationManifest,
) -> Result<Vec<f32>> {
    let expected = manifest.frames * manifest.local_speakers;
    match tensor.shape.as_slice() {
        [1, frames, speakers] if *frames == manifest.frames && *speakers == manifest.local_speakers => {
            Ok(tensor.values.clone())
        }
        [frames, speakers] if *frames == manifest.frames && *speakers == manifest.local_speakers => {
            Ok(tensor.values.clone())
        }
        [1, values] if *values == expected => Ok(tensor.values.clone()),
        [values] if *values == expected => Ok(tensor.values.clone()),
        shape => Err(DetectError::InvalidArgument(format!(
            "model_output_mismatch: pyannote segmentation output shape {shape:?} does not match expected [1, {}, {}]",
            manifest.frames, manifest.local_speakers
        ))),
    }
}

fn embedding_values(tensor: &OnnxF32Tensor, dimension: usize) -> Result<Vec<f32>> {
    match tensor.shape.as_slice() {
        [dim] if *dim == dimension => Ok(tensor.values.clone()),
        [1, dim] if *dim == dimension => Ok(tensor.values.clone()),
        shape => Err(DetectError::InvalidArgument(format!(
            "model_output_mismatch: pyannote embedding output shape {shape:?} does not match expected [1, {dimension}]"
        ))),
    }
}

fn speaker_mask(
    manifest: &PyannoteDiarizationManifest,
    window: &SegmentationWindow,
    local_speaker: usize,
) -> Vec<f32> {
    let mut mask = vec![0.0_f32; manifest.embedding.mask_frames];
    let copy_frames = manifest
        .segmentation
        .frames
        .min(manifest.embedding.mask_frames);
    for (frame, value) in mask.iter_mut().enumerate().take(copy_frames) {
        let score = window.scores[frame * manifest.segmentation.local_speakers + local_speaker];
        *value = if manifest.segmentation.powerset {
            score
        } else if score > ACTIVE_THRESHOLD {
            1.0
        } else {
            0.0
        };
    }
    mask
}

fn clean_speaker_frames(
    manifest: &PyannoteDiarizationManifest,
    window: &SegmentationWindow,
    local_speaker: usize,
) -> usize {
    let mut count = 0_usize;
    for frame in 0..manifest.segmentation.frames {
        let mut active = 0_usize;
        let mut selected_active = false;
        for speaker in 0..manifest.segmentation.local_speakers {
            let score = window.scores[frame * manifest.segmentation.local_speakers + speaker];
            if score > ACTIVE_THRESHOLD {
                active += 1;
                if speaker == local_speaker {
                    selected_active = true;
                }
            }
        }
        if active == 1 && selected_active {
            count += 1;
        }
    }
    count
}

fn assign_local_speakers(
    embeddings: &[LocalSpeakerEmbedding],
    min_speakers: Option<usize>,
    max_speakers: Option<usize>,
    threshold: f32,
) -> Result<Vec<AssignedLocalSpeaker>> {
    if embeddings.is_empty() {
        return Ok(Vec::new());
    }
    let frames = embeddings
        .iter()
        .map(|embedding| embedding.active_frames.max(1))
        .max()
        .unwrap_or(1);
    let min_clean_frames = (0.2_f32 * frames as f32).ceil() as usize;
    let train_indices = embeddings
        .iter()
        .enumerate()
        .filter_map(|(index, embedding)| {
            (embedding.clean_frames >= min_clean_frames).then_some(index)
        })
        .collect::<Vec<_>>();
    let train_indices = if train_indices.is_empty() {
        (0..embeddings.len()).collect::<Vec<_>>()
    } else {
        train_indices
    };
    let requested = match (min_speakers, max_speakers) {
        (Some(min), Some(max)) if min == max => Some(min),
        _ => None,
    };
    let mut centroids: Vec<Vec<f32>> = Vec::new();
    for &index in &train_indices {
        let values = l2_normalize(&embeddings[index].values)?;
        let mut best = None;
        let mut best_score = f32::NEG_INFINITY;
        for (cluster, centroid) in centroids.iter().enumerate() {
            let score = cosine_similarity(&values, centroid)?;
            if score > best_score {
                best_score = score;
                best = Some(cluster);
            }
        }
        let should_create = best.is_none()
            || best_score < threshold
            || requested.is_some_and(|count| centroids.len() < count);
        if should_create {
            centroids.push(values);
        } else if let Some(cluster) = best {
            merge_centroid(&mut centroids[cluster], &values)?;
        }
    }
    let min_clusters = min_speakers.unwrap_or(1).min(embeddings.len()).max(1);
    let max_clusters = max_speakers
        .unwrap_or(embeddings.len())
        .min(embeddings.len())
        .max(1);
    while centroids.len() < min_clusters {
        let next = embeddings
            .iter()
            .map(|embedding| l2_normalize(&embedding.values))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .max_by(|left, right| {
                min_similarity(left, &centroids)
                    .partial_cmp(&min_similarity(right, &centroids))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or_else(|| centroids[0].clone());
        centroids.push(next);
    }
    while centroids.len() > max_clusters {
        merge_closest_centroids(&mut centroids)?;
    }
    let mut assignments = Vec::new();
    for embedding in embeddings {
        let values = l2_normalize(&embedding.values)?;
        let mut best_cluster = 0_usize;
        let mut best_score = f32::NEG_INFINITY;
        for (cluster, centroid) in centroids.iter().enumerate() {
            let score = cosine_similarity(&values, centroid)?;
            if score > best_score {
                best_score = score;
                best_cluster = cluster;
            }
        }
        assignments.push(AssignedLocalSpeaker {
            chunk: embedding.chunk,
            local_speaker: embedding.local_speaker,
            speaker: best_cluster,
            score: best_score,
        });
    }
    Ok(assignments)
}

fn reconstruct_segments(
    manifest: &PyannoteDiarizationManifest,
    segmentations: &SegmentationBatch,
    assignments: &[AssignedLocalSpeaker],
) -> Result<Vec<SpeakerSegmentPrediction>> {
    let mut segments = Vec::new();
    let frame_duration =
        manifest.segmentation.duration_seconds / manifest.segmentation.frames as f64;
    let mut by_local = BTreeMap::new();
    for assignment in assignments {
        by_local.insert((assignment.chunk, assignment.local_speaker), assignment);
    }
    let mut open: BTreeMap<usize, (f64, f64, f32)> = BTreeMap::new();
    for (chunk, window) in segmentations.windows.iter().enumerate() {
        for frame in 0..manifest.segmentation.frames {
            let frame_start = window.start_seconds + frame as f64 * frame_duration;
            let frame_end = frame_start + frame_duration;
            let mut active_speakers = BTreeMap::<usize, f32>::new();
            for local_speaker in 0..manifest.segmentation.local_speakers {
                let score =
                    window.scores[frame * manifest.segmentation.local_speakers + local_speaker];
                if score <= ACTIVE_THRESHOLD {
                    continue;
                }
                if let Some(assignment) = by_local.get(&(chunk, local_speaker)) {
                    let combined_score = score.max(assignment.score);
                    active_speakers
                        .entry(assignment.speaker)
                        .and_modify(|current| *current = current.max(combined_score))
                        .or_insert(combined_score);
                }
            }
            let active_keys = active_speakers.keys().copied().collect::<BTreeSet<_>>();
            let closing = open
                .keys()
                .copied()
                .filter(|speaker| !active_keys.contains(speaker))
                .collect::<Vec<_>>();
            for speaker in closing {
                if let Some((start, end, score)) = open.remove(&speaker) {
                    push_segment(&mut segments, manifest, speaker, start, end, score)?;
                }
            }
            for (speaker, score) in active_speakers {
                open.entry(speaker)
                    .and_modify(|entry| {
                        entry.1 = frame_end;
                        entry.2 = entry.2.max(score);
                    })
                    .or_insert((frame_start, frame_end, score));
            }
        }
    }
    for (speaker, (start, end, score)) in open {
        push_segment(&mut segments, manifest, speaker, start, end, score)?;
    }
    segments.sort_by(|left, right| {
        left.start_seconds
            .partial_cmp(&right.start_seconds)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.speaker.cmp(&right.speaker))
    });
    Ok(segments)
}

fn push_segment(
    segments: &mut Vec<SpeakerSegmentPrediction>,
    manifest: &PyannoteDiarizationManifest,
    speaker: usize,
    start: f64,
    end: f64,
    score: f32,
) -> Result<()> {
    if end <= start {
        return Ok(());
    }
    let label = speaker_label(manifest, speaker);
    if let Some(last) = segments.last_mut() {
        if last.speaker == label && (last.end_seconds as f64 - start).abs() <= 0.02 {
            last.end_seconds = end as f32;
            last.score = Some(last.score.unwrap_or(score).max(score));
            return Ok(());
        }
    }
    segments.push(SpeakerSegmentPrediction {
        speaker: label,
        start_seconds: start as f32,
        end_seconds: end as f32,
        score: Some(score),
    });
    Ok(())
}

fn speaker_embedding_map(
    manifest: &PyannoteDiarizationManifest,
    embeddings: &[LocalSpeakerEmbedding],
    assignments: &[AssignedLocalSpeaker],
) -> Result<BTreeMap<String, SpeakerEmbedding>> {
    let model = SpeakerEmbeddingModel {
        family: SpeakerEmbeddingModelFamily::Pyannote,
        name: manifest.model_id.clone(),
        version: "community-1".to_string(),
        dimensions: manifest.embedding.dimension,
    };
    let mut grouped: BTreeMap<usize, Vec<&LocalSpeakerEmbedding>> = BTreeMap::new();
    for assignment in assignments {
        if let Some(embedding) = embeddings.iter().find(|item| {
            item.chunk == assignment.chunk && item.local_speaker == assignment.local_speaker
        }) {
            grouped
                .entry(assignment.speaker)
                .or_default()
                .push(embedding);
        }
    }
    let mut map = BTreeMap::new();
    for (speaker, items) in grouped {
        let mut values = vec![0.0_f32; manifest.embedding.dimension];
        for item in &items {
            for (index, value) in item.values.iter().enumerate().take(values.len()) {
                values[index] += *value;
            }
        }
        for value in &mut values {
            *value /= items.len() as f32;
        }
        map.insert(
            speaker_label(manifest, speaker),
            SpeakerEmbedding::new(values, model.clone(), SAMPLE_RATE)?,
        );
    }
    Ok(map)
}

fn speaker_label(manifest: &PyannoteDiarizationManifest, speaker: usize) -> String {
    if manifest.label_format == DEFAULT_LABEL_FORMAT {
        format!("SPEAKER_{speaker:02}")
    } else {
        manifest
            .label_format
            .replace("{:02}", &format!("{speaker:02}"))
            .replace("{}", &speaker.to_string())
    }
}

fn l2_normalize(values: &[f32]) -> Result<Vec<f32>> {
    let norm = values.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm <= f32::EPSILON || !norm.is_finite() {
        return Err(DetectError::InvalidArgument(
            "invalid_request: pyannote embedding norm must be finite and non-zero".to_string(),
        ));
    }
    Ok(values.iter().map(|value| value / norm).collect())
}

fn cosine_similarity(left: &[f32], right: &[f32]) -> Result<f32> {
    if left.len() != right.len() {
        return Err(DetectError::InvalidArgument(
            "invalid_request: pyannote embedding dimensions differ".to_string(),
        ));
    }
    Ok(left.iter().zip(right).map(|(l, r)| l * r).sum())
}

fn merge_centroid(centroid: &mut [f32], values: &[f32]) -> Result<()> {
    if centroid.len() != values.len() {
        return Err(DetectError::InvalidArgument(
            "invalid_request: pyannote centroid dimensions differ".to_string(),
        ));
    }
    for (left, right) in centroid.iter_mut().zip(values) {
        *left = (*left + *right) * 0.5;
    }
    let normalized = l2_normalize(centroid)?;
    centroid.copy_from_slice(&normalized);
    Ok(())
}

fn min_similarity(values: &[f32], centroids: &[Vec<f32>]) -> f32 {
    if centroids.is_empty() {
        return f32::NEG_INFINITY;
    }
    centroids
        .iter()
        .filter_map(|centroid| cosine_similarity(values, centroid).ok())
        .fold(f32::INFINITY, f32::min)
}

fn merge_closest_centroids(centroids: &mut Vec<Vec<f32>>) -> Result<()> {
    if centroids.len() <= 1 {
        return Ok(());
    }
    let mut best = (0_usize, 1_usize);
    let mut best_score = f32::NEG_INFINITY;
    for left in 0..centroids.len() {
        for right in (left + 1)..centroids.len() {
            let score = cosine_similarity(&centroids[left], &centroids[right])?;
            if score > best_score {
                best_score = score;
                best = (left, right);
            }
        }
    }
    let right = centroids.remove(best.1);
    merge_centroid(&mut centroids[best.0], &right)
}

fn format_optional_usize(value: Option<usize>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".to_string())
}

fn setup_error(message: impl Into<String>) -> DetectError {
    DetectError::InvalidArgument(format!("setup_error: {}", message.into()))
}

fn map_onnx_session_error(error: runtime_onnx::OnnxRuntimeError) -> DetectError {
    match error {
        runtime_onnx::OnnxRuntimeError::InvalidArgument(message)
            if message.contains("does not exist") =>
        {
            setup_error(message)
        }
        runtime_onnx::OnnxRuntimeError::Io(error) => setup_error(error.to_string()),
        other => DetectError::InvalidArgument(format!("unsupported_runtime: {other}")),
    }
}

fn map_onnx_runtime_error(error: runtime_onnx::OnnxRuntimeError) -> DetectError {
    DetectError::InvalidArgument(format!("model_output_mismatch: {error}"))
}

fn default_model_id() -> String {
    DEFAULT_MODEL_ID.to_string()
}

fn default_sample_rate() -> u32 {
    SAMPLE_RATE
}

fn default_label_format() -> String {
    DEFAULT_LABEL_FORMAT.to_string()
}

fn default_segmentation_input_name() -> String {
    "waveform".to_string()
}

fn default_segmentation_output_name() -> String {
    "segmentations".to_string()
}

fn default_step_ratio() -> f64 {
    0.1
}

fn default_true() -> bool {
    true
}

fn default_embedding_waveform_input_name() -> String {
    "waveform".to_string()
}

fn default_embedding_mask_input_name() -> String {
    "masks".to_string()
}

fn default_embedding_output_name() -> String {
    "embeddings".to_string()
}

fn default_clustering_kind() -> String {
    "vbx".to_string()
}

fn default_clustering_threshold() -> f32 {
    0.6
}

fn default_fa() -> f32 {
    0.07
}

fn default_fb() -> f32 {
    0.8
}

fn default_vbx_iters() -> usize {
    20
}

fn default_min_active_ratio() -> f32 {
    0.2
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> PyannoteDiarizationManifest {
        PyannoteDiarizationManifest {
            model_id: DEFAULT_MODEL_ID.to_string(),
            sample_rate: SAMPLE_RATE,
            label_format: DEFAULT_LABEL_FORMAT.to_string(),
            segmentation: PyannoteSegmentationManifest {
                input_name: "waveform".to_string(),
                output_name: "segmentations".to_string(),
                duration_seconds: 1.0,
                step_ratio: 1.0,
                powerset: true,
                frames: 4,
                local_speakers: 2,
            },
            embedding: PyannoteEmbeddingManifest {
                waveform_input_name: "waveform".to_string(),
                mask_input_name: "masks".to_string(),
                output_name: "embeddings".to_string(),
                dimension: 2,
                mask_frames: 4,
            },
            clustering: PyannoteClusteringManifest {
                kind: "vbx".to_string(),
                threshold: 0.6,
                fa: 0.07,
                fb: 0.8,
                max_iters: 20,
                min_active_ratio: 0.2,
                constrained_assignment: true,
            },
        }
    }

    #[test]
    fn manifest_validation_rejects_wrong_sample_rate() {
        let mut manifest = manifest();
        manifest.sample_rate = 8_000;

        let error = validate_manifest(&manifest).expect_err("invalid sample rate");

        assert!(error.to_string().contains("sampleRate"));
    }

    #[test]
    fn speaker_bounds_reject_zero_and_inverted_range() {
        assert!(validate_speaker_bounds(Some(0), None)
            .unwrap_err()
            .to_string()
            .contains("min_speakers"));
        assert!(validate_speaker_bounds(Some(3), Some(2))
            .unwrap_err()
            .to_string()
            .contains("less than or equal"));
    }

    #[test]
    fn speaker_mask_uses_powerset_scores() {
        let manifest = manifest();
        let window = SegmentationWindow {
            start_seconds: 0.0,
            samples: vec![0.0; SAMPLE_RATE as usize],
            scores: vec![0.1, 0.9, 0.2, 0.8, 0.3, 0.7, 0.4, 0.6],
        };

        assert_eq!(
            speaker_mask(&manifest, &window, 1),
            vec![0.9, 0.8, 0.7, 0.6]
        );
    }

    #[test]
    fn reconstruct_segments_emits_speaker_labels() {
        let manifest = manifest();
        let segmentations = SegmentationBatch {
            total_frames: 4,
            windows: vec![SegmentationWindow {
                start_seconds: 0.0,
                samples: vec![0.0; SAMPLE_RATE as usize],
                scores: vec![0.9, 0.1, 0.8, 0.1, 0.1, 0.9, 0.1, 0.8],
            }],
        };
        let assignments = vec![
            AssignedLocalSpeaker {
                chunk: 0,
                local_speaker: 0,
                speaker: 0,
                score: 1.0,
            },
            AssignedLocalSpeaker {
                chunk: 0,
                local_speaker: 1,
                speaker: 1,
                score: 1.0,
            },
        ];

        let segments = reconstruct_segments(&manifest, &segmentations, &assignments).unwrap();

        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].speaker, "SPEAKER_00");
        assert_eq!(segments[1].speaker, "SPEAKER_01");
    }
}
