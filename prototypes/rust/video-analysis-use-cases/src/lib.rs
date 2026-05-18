#![doc = include_str!("../README.md")]

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;

use audio_analysis_processing::AudioEnergyAnalyzer;
use audio_analysis_separation::{DemucsModel, HtdemucsOptions, HtdemucsSeparator, Stem};
use serde::{Deserialize, Serialize};
use text_analysis_features::TranscriptHeuristicAnalyzer;
use text_analysis_transcription::{
    segment_to_owned_text_segment, Transcriber, TranscriptSegment, WhisperCliTranscriber,
    WhisperCppProgressEvent, WhisperCppTranscriber,
};
/// Re-exports the text analysis transcription whisper cpp config API.
pub use text_analysis_transcription::{WhisperCppConfig, WhisperCppModel};
use video_analysis_core::{
    AnalysisEvent, BoundingBox, DetectError, Observation, ObservationKind, RealtimeVideoPipeline,
    Result, SampledVideoAnalyzer,
};
use video_analysis_data::{
    BucketAggregator, BucketConfig, DataBucket, DataRecord, DataStreamKind, StreamSummary,
};
use video_analysis_detectors::ContentDetector;
use video_analysis_ffmpeg::{
    extract_wav, FfmpegAudioSource, FfmpegAudioSourceOptions, FfmpegVideoSource,
};
use video_analysis_ingest::{AudioFrameSource, VideoFrameSource};
use video_analysis_models::{
    DownloadedModel, ExternalCommandModel, HuggingFaceModelSpec, ModelTask, ModelTextAnalyzer,
    ModelVideoAnalyzer,
};

/// Constant for youtube video use case.
pub const YOUTUBE_VIDEO_USE_CASE: &str = "youtube-video";
/// Constant for video red cars use case.
pub const VIDEO_RED_CARS_USE_CASE: &str = "video-red-cars";
/// Constant for audio voice analysis use case.
pub const AUDIO_VOICE_ANALYSIS_USE_CASE: &str = "audio-voice-analysis";
/// Constant for image person edit use case.
pub const IMAGE_PERSON_EDIT_USE_CASE: &str = "image-person-edit";

/// Public module for audio voice analysis.
pub mod audio_voice_analysis;
/// Public module for image person edit.
pub mod image_person_edit;
mod song_impl;
mod subtitle_impl;
/// Public module for video red cars.
pub mod video_red_cars;
mod workflow_support;

/// Public module for workflow catalog.
pub mod workflow_catalog;

/// Public module for youtube.
pub mod youtube {
    /// Re-exports the super API.
    pub use super::{
        discover_youtube_collection_manifest, run_youtube_collection_workflow,
        run_youtube_collection_workflow_with_progress, run_youtube_video,
        run_youtube_video_workflow, run_youtube_video_workflow_with_progress,
        write_youtube_video_report, AlreadyDownloadedPolicy, AppliedCollectionFilters, AssetReport,
        AudioReport, AudioSeparationConfig, AudioSeparationReport, AudioStemReport,
        CapabilityReport, CollectionAssetReport, CollectionItemReport, CollectionItemStatus,
        CollectionSummary, DataBucketReport, EventReport, ExternalCommandConfig,
        FailedProbeBehavior, LocalFileFilterSpec, ManifestFilterSpec, MetadataDepth,
        ModelCommandConfig, ObservationReport, RegionReport, SceneReport, SourceReport, SourceSpec,
        StreamBucketReport, TextReport, TranscriptSegmentReport, TranscriptionConfig,
        TranscriptionEngine, TranscriptionReport, VideoReport, WhisperCppConfig, WhisperCppModel,
        YoutubeCollectionManifest, YoutubeCollectionManifestItem, YoutubeCollectionManifestRequest,
        YoutubeCollectionReport, YoutubeCollectionRunRequest, YoutubeCollectionSource,
        YoutubeCollectionSourceReport, YoutubeRunOptions, YoutubeRunRequest, YoutubeVideoReport,
        YoutubeVideoRequest, YOUTUBE_VIDEO_USE_CASE,
    };
}

/// Public module for song.
pub mod song {
    /// Re-exports the song impl API.
    pub use crate::song_impl::{
        import_song_catalog_items, run_song_analysis_workflow, MusicAnalysisConfig,
        MusicAnalysisReport, MusicFeatureReport, MusicSegmentReport, SongAnalysisReport,
        SongAnalysisRunRequest, SongAssetReport, SongCatalogConfig, SongCatalogImportItem,
        SongCatalogImportRequest, SongCatalogImportResponse, SongCatalogImportResult,
        SongCatalogMetadata, SongFingerprintQueryReport, SongIdentificationConfig,
        SongIdentificationReport, SongMatchCandidateReport, SongMatchedSegmentReport,
        SongSourceReport,
    };
}

/// Public module for subtitle.
pub mod subtitle {
    /// Re-exports the subtitle impl API.
    pub use crate::subtitle_impl::{
        run_subtitle_generation_workflow, SubtitleAssetReport, SubtitleGenerationReport,
        SubtitleGenerationRunRequest, SubtitleReport,
    };
}

#[derive(Debug, Clone)]
/// Data type for youtube video request.
pub struct YoutubeVideoRequest {
    /// URL associated with this value.
    pub url: Option<String>,
    /// The input value.
    pub input: Option<PathBuf>,
    /// The work dir value.
    pub work_dir: PathBuf,
    /// The output value.
    pub output: Option<PathBuf>,
    /// The scene threshold value.
    pub scene_threshold: f32,
    /// The min scene len value.
    pub min_scene_len: u64,
    /// The max frames value.
    pub max_frames: Option<u64>,
    /// The visual sample every value.
    pub visual_sample_every: u64,
    /// The skip transcription value.
    pub skip_transcription: bool,
    /// The transcriber engine value.
    pub transcriber_engine: TranscriptionEngine,
    /// The transcriber command value.
    pub transcriber_command: Option<PathBuf>,
    /// The transcriber args value.
    pub transcriber_args: Vec<String>,
    /// The whisper cpp value.
    pub whisper_cpp: WhisperCppConfig,
    /// The audio separation value.
    pub audio_separation: AudioSeparationConfig,
    /// The object command value.
    pub object_command: Option<PathBuf>,
    /// The object args value.
    pub object_args: Vec<String>,
    /// The ocr command value.
    pub ocr_command: Option<PathBuf>,
    /// The ocr args value.
    pub ocr_args: Vec<String>,
    /// The text command value.
    pub text_command: Option<PathBuf>,
    /// The text args value.
    pub text_args: Vec<String>,
}

impl Default for YoutubeVideoRequest {
    fn default() -> Self {
        Self {
            url: None,
            input: None,
            work_dir: PathBuf::from("use-case-output/youtube-video"),
            output: None,
            scene_threshold: 27.0,
            min_scene_len: 15,
            max_frames: None,
            visual_sample_every: 30,
            skip_transcription: false,
            transcriber_engine: TranscriptionEngine::default(),
            transcriber_command: None,
            transcriber_args: Vec::new(),
            whisper_cpp: WhisperCppConfig::default(),
            audio_separation: AudioSeparationConfig::default(),
            object_command: None,
            object_args: Vec::new(),
            ocr_command: None,
            ocr_args: Vec::new(),
            text_command: None,
            text_args: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
/// Variants describing source spec.
pub enum SourceSpec {
    /// The youtube URL variant.
    YoutubeUrl {
        /// URL associated with this variant.
        url: String,
    },
    /// The local file variant.
    LocalFile {
        /// Filesystem path for this variant.
        path: PathBuf,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
/// Data type for external command config.
pub struct ExternalCommandConfig {
    /// The command value.
    pub command: PathBuf,
    #[serde(default)]
    /// The args value.
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
/// Variants describing transcription engine.
pub enum TranscriptionEngine {
    #[default]
    /// The whisper cpp variant.
    WhisperCpp,
    /// The whisper variant.
    Whisper,
    #[serde(alias = "fast_whisper")]
    /// The faster whisper variant.
    FasterWhisper,
    /// The whisper x variant.
    WhisperX,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
/// Data type for transcription config.
pub struct TranscriptionConfig {
    /// The enabled value.
    pub enabled: bool,
    #[serde(default)]
    /// The engine value.
    pub engine: TranscriptionEngine,
    /// The command value.
    pub command: Option<ExternalCommandConfig>,
    #[serde(default)]
    /// The whisper cpp value.
    pub whisper_cpp: WhisperCppConfig,
}

impl Default for TranscriptionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            engine: TranscriptionEngine::default(),
            command: None,
            whisper_cpp: WhisperCppConfig::default(),
        }
    }
}

impl TranscriptionConfig {
    /// Returns a normalized copy of this value.
    pub fn normalized(mut self) -> Self {
        if self.engine == TranscriptionEngine::Whisper && self.command.is_none() {
            self.engine = TranscriptionEngine::WhisperCpp;
        }
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
/// Data type for audio separation config.
pub struct AudioSeparationConfig {
    /// The enabled value.
    pub enabled: bool,
    /// The command value.
    pub command: Option<ExternalCommandConfig>,
    /// The model value.
    pub model: Option<String>,
    /// The two stems value.
    pub two_stems: Option<String>,
    /// The device value.
    pub device: Option<String>,
    /// The output dir value.
    pub output_dir: Option<PathBuf>,
}

impl AudioSeparationConfig {
    /// Validates this value.
    pub fn validate(&self) -> Result<()> {
        if let Some(model) = &self.model {
            DemucsModel::from_str(model)?;
        }
        if let Some(stem) = &self.two_stems {
            Stem::from_str(stem)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
/// Data type for model command config.
pub struct ModelCommandConfig {
    /// The object value.
    pub object: Option<ExternalCommandConfig>,
    /// The ocr value.
    pub ocr: Option<ExternalCommandConfig>,
    /// Text content for this value.
    pub text: Option<ExternalCommandConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for youtube run request.
pub struct YoutubeRunRequest {
    /// The source value.
    pub source: SourceSpec,
    /// The work dir value.
    pub work_dir: Option<PathBuf>,
    #[serde(default = "default_scene_threshold")]
    /// The scene threshold value.
    pub scene_threshold: f32,
    #[serde(default = "default_min_scene_len")]
    /// The min scene len value.
    pub min_scene_len: u64,
    /// The max frames value.
    pub max_frames: Option<u64>,
    #[serde(default = "default_visual_sample_every")]
    /// The visual sample every value.
    pub visual_sample_every: u64,
    #[serde(default)]
    /// The transcription value.
    pub transcription: TranscriptionConfig,
    #[serde(default)]
    /// The audio separation value.
    pub audio_separation: AudioSeparationConfig,
    #[serde(default)]
    /// The models value.
    pub models: ModelCommandConfig,
}

impl YoutubeRunRequest {
    /// Validates this value.
    pub fn validate(&self) -> Result<()> {
        validate_analysis_options(
            self.scene_threshold,
            self.min_scene_len,
            self.visual_sample_every,
        )?;
        self.audio_separation.validate()?;
        match &self.source {
            SourceSpec::YoutubeUrl { url } => validate_youtube_url(url),
            SourceSpec::LocalFile { path } => validate_local_file(path),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
/// Variants describing metadata depth.
pub enum MetadataDepth {
    /// The fast variant.
    Fast,
    #[default]
    /// The enriched when needed variant.
    EnrichedWhenNeeded,
    /// The full variant.
    Full,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
/// Variants describing already downloaded policy.
pub enum AlreadyDownloadedPolicy {
    /// The include variant.
    Include,
    /// The skip variant.
    Skip,
    #[default]
    /// The reuse variant.
    Reuse,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
/// Data type for manifest filter spec.
pub struct ManifestFilterSpec {
    /// The title contains value.
    pub title_contains: Option<String>,
    #[serde(default)]
    /// The title excludes value.
    pub title_excludes: Vec<String>,
    /// The playlist index min value.
    pub playlist_index_min: Option<u64>,
    /// The playlist index max value.
    pub playlist_index_max: Option<u64>,
    /// The upload date from value.
    pub upload_date_from: Option<String>,
    /// The upload date to value.
    pub upload_date_to: Option<String>,
    /// The duration seconds min value.
    pub duration_seconds_min: Option<f64>,
    /// The duration seconds max value.
    pub duration_seconds_max: Option<f64>,
    #[serde(default)]
    /// The already downloaded value.
    pub already_downloaded: AlreadyDownloadedPolicy,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
/// Variants describing failed probe behavior.
pub enum FailedProbeBehavior {
    #[default]
    /// The failed variant.
    Failed,
    /// The include variant.
    Include,
    /// The filtered variant.
    Filtered,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
/// Data type for local file filter spec.
pub struct LocalFileFilterSpec {
    /// The duration seconds min value.
    pub duration_seconds_min: Option<f64>,
    /// The duration seconds max value.
    pub duration_seconds_max: Option<f64>,
    /// The file size bytes min value.
    pub file_size_bytes_min: Option<u64>,
    /// The file size bytes max value.
    pub file_size_bytes_max: Option<u64>,
    #[serde(default)]
    /// The extension allowlist value.
    pub extension_allowlist: Vec<String>,
    #[serde(default)]
    /// The failed probe behavior value.
    pub failed_probe_behavior: FailedProbeBehavior,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for youtube run options.
pub struct YoutubeRunOptions {
    #[serde(default = "default_scene_threshold")]
    /// The scene threshold value.
    pub scene_threshold: f32,
    #[serde(default = "default_min_scene_len")]
    /// The min scene len value.
    pub min_scene_len: u64,
    /// The max frames value.
    pub max_frames: Option<u64>,
    #[serde(default = "default_visual_sample_every")]
    /// The visual sample every value.
    pub visual_sample_every: u64,
    #[serde(default)]
    /// The transcription value.
    pub transcription: TranscriptionConfig,
    #[serde(default)]
    /// The audio separation value.
    pub audio_separation: AudioSeparationConfig,
    #[serde(default)]
    /// The models value.
    pub models: ModelCommandConfig,
}

impl Default for YoutubeRunOptions {
    fn default() -> Self {
        Self {
            scene_threshold: default_scene_threshold(),
            min_scene_len: default_min_scene_len(),
            max_frames: None,
            visual_sample_every: default_visual_sample_every(),
            transcription: TranscriptionConfig::default(),
            audio_separation: AudioSeparationConfig::default(),
            models: ModelCommandConfig::default(),
        }
    }
}

impl YoutubeRunOptions {
    /// Validates this value.
    pub fn validate(&self) -> Result<()> {
        validate_analysis_options(
            self.scene_threshold,
            self.min_scene_len,
            self.visual_sample_every,
        )?;
        self.audio_separation.validate()
    }

    fn to_request(&self, source: SourceSpec, work_dir: PathBuf) -> YoutubeRunRequest {
        YoutubeRunRequest {
            source,
            work_dir: Some(work_dir),
            scene_threshold: self.scene_threshold,
            min_scene_len: self.min_scene_len,
            max_frames: self.max_frames,
            visual_sample_every: self.visual_sample_every,
            transcription: self.transcription.clone(),
            audio_separation: self.audio_separation.clone(),
            models: self.models.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
/// Variants describing youtube collection source.
pub enum YoutubeCollectionSource {
    /// The playlist URL variant.
    PlaylistUrl {
        /// URL associated with this variant.
        url: String,
    },
    /// The channel URL variant.
    ChannelUrl {
        /// URL associated with this variant.
        url: String,
    },
    /// The youtube URL variant.
    YoutubeUrl {
        /// URL associated with this variant.
        url: String,
    },
}

impl YoutubeCollectionSource {
    /// Returns URL.
    pub fn url(&self) -> &str {
        match self {
            Self::PlaylistUrl { url } | Self::ChannelUrl { url } | Self::YoutubeUrl { url } => url,
        }
    }

    /// Returns kind name.
    pub fn kind_name(&self) -> &'static str {
        match self {
            Self::PlaylistUrl { .. } => "playlist_url",
            Self::ChannelUrl { .. } => "channel_url",
            Self::YoutubeUrl { .. } => "youtube_url",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for youtube collection manifest request.
pub struct YoutubeCollectionManifestRequest {
    /// The source value.
    pub source: YoutubeCollectionSource,
    #[serde(default)]
    /// The metadata filter value.
    pub metadata_filter: ManifestFilterSpec,
    #[serde(default)]
    /// The metadata depth value.
    pub metadata_depth: MetadataDepth,
    /// The max items value.
    pub max_items: Option<u64>,
}

impl YoutubeCollectionManifestRequest {
    /// Validates this value.
    pub fn validate(&self) -> Result<()> {
        validate_youtube_url(self.source.url())?;
        validate_manifest_filter(&self.metadata_filter)?;
        if self.max_items == Some(0) {
            return Err(DetectError::InvalidArgument(
                "max_items must be positive when provided".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for youtube collection run request.
pub struct YoutubeCollectionRunRequest {
    /// The source value.
    pub source: Option<YoutubeCollectionSource>,
    /// The manifest value.
    pub manifest: Option<YoutubeCollectionManifest>,
    #[serde(default)]
    /// The metadata filter value.
    pub metadata_filter: ManifestFilterSpec,
    #[serde(default)]
    /// The local file filter value.
    pub local_file_filter: LocalFileFilterSpec,
    #[serde(default)]
    /// The analysis value.
    pub analysis: YoutubeRunOptions,
    #[serde(default)]
    /// The delete filtered downloads value.
    pub delete_filtered_downloads: bool,
    /// The work dir value.
    pub work_dir: Option<PathBuf>,
    /// The max items value.
    pub max_items: Option<u64>,
}

impl YoutubeCollectionRunRequest {
    /// Validates this value.
    pub fn validate(&self) -> Result<()> {
        if self.source.is_none() && self.manifest.is_none() {
            return Err(DetectError::InvalidArgument(
                "source or manifest is required".to_string(),
            ));
        }
        if let Some(source) = &self.source {
            validate_youtube_url(source.url())?;
        }
        validate_manifest_filter(&self.metadata_filter)?;
        validate_local_file_filter(&self.local_file_filter)?;
        self.analysis.validate()?;
        if self.max_items == Some(0) {
            return Err(DetectError::InvalidArgument(
                "max_items must be positive when provided".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for youtube collection manifest.
pub struct YoutubeCollectionManifest {
    /// The collection identifier value.
    pub collection_id: String,
    /// The source value.
    pub source: YoutubeCollectionSource,
    /// The title value.
    pub title: Option<String>,
    /// The items value.
    pub items: Vec<YoutubeCollectionManifestItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for youtube collection manifest item.
pub struct YoutubeCollectionManifestItem {
    /// The item identifier value.
    pub item_id: String,
    /// The youtube identifier value.
    pub youtube_id: Option<String>,
    /// The playlist index value.
    pub playlist_index: Option<u64>,
    /// The title value.
    pub title: Option<String>,
    /// The source URL value.
    pub source_url: String,
    /// Duration in seconds.
    pub duration_seconds: Option<f64>,
    /// The upload date value.
    pub upload_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for youtube video report.
pub struct YoutubeVideoReport {
    /// The use case value.
    pub use_case: String,
    /// The source value.
    pub source: SourceReport,
    /// The assets value.
    pub assets: AssetReport,
    /// The capabilities value.
    pub capabilities: CapabilityReport,
    /// The video value.
    pub video: VideoReport,
    /// The transcription value.
    pub transcription: TranscriptionReport,
    /// The audio value.
    pub audio: AudioReport,
    /// Text content for this value.
    pub text: TextReport,
    /// The data buckets value.
    pub data_buckets: Vec<DataBucketReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for source report.
pub struct SourceReport {
    /// URL associated with this value.
    pub url: Option<String>,
    /// The local video value.
    pub local_video: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for asset report.
pub struct AssetReport {
    /// The work dir value.
    pub work_dir: String,
    /// The report path value.
    pub report_path: String,
    /// The audio wav value.
    pub audio_wav: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for capability report.
pub struct CapabilityReport {
    /// The completed value.
    pub completed: Vec<String>,
    /// The skipped value.
    pub skipped: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for video report.
pub struct VideoReport {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// The frame rate value.
    pub frame_rate: String,
    /// Duration in seconds.
    pub duration_seconds: Option<f64>,
    /// The frames processed value.
    pub frames_processed: u64,
    /// The scenes value.
    pub scenes: Vec<SceneReport>,
    /// The observations value.
    pub observations: Vec<ObservationReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for scene report.
pub struct SceneReport {
    /// The index value.
    pub index: u64,
    /// The start frame value.
    pub start_frame: u64,
    /// The end frame value.
    pub end_frame: u64,
    /// The start seconds value.
    pub start_seconds: f64,
    /// The end seconds value.
    pub end_seconds: f64,
    /// The observations value.
    pub observations: Vec<ObservationReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for observation report.
pub struct ObservationReport {
    /// The timestamp seconds value.
    pub timestamp_seconds: Option<f64>,
    /// The frame index value.
    pub frame_index: Option<u64>,
    /// The scene index value.
    pub scene_index: Option<u64>,
    /// The analyzer value.
    pub analyzer: String,
    /// The kind value.
    pub kind: String,
    /// Label assigned to this value.
    pub label: Option<String>,
    /// Text content for this value.
    pub text: Option<String>,
    /// Score assigned to this value.
    pub score: Option<f32>,
    /// The region value.
    pub region: Option<RegionReport>,
    /// The track identifier value.
    pub track_id: Option<String>,
    /// The attributes value.
    pub attributes: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for region report.
pub struct RegionReport {
    /// The x value.
    pub x: u32,
    /// The y value.
    pub y: u32,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for transcription report.
pub struct TranscriptionReport {
    /// The status value.
    pub status: String,
    /// Text content for this value.
    pub text: Option<String>,
    /// The segments value.
    pub segments: Vec<TranscriptSegmentReport>,
    /// The message value.
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for transcript segment report.
pub struct TranscriptSegmentReport {
    /// The index value.
    pub index: u64,
    /// The start seconds value.
    pub start_seconds: Option<f64>,
    /// The end seconds value.
    pub end_seconds: Option<f64>,
    /// Text content for this value.
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for audio report.
pub struct AudioReport {
    /// The status value.
    pub status: String,
    /// The frames processed value.
    pub frames_processed: u64,
    /// The events value.
    pub events: Vec<EventReport>,
    /// The separation value.
    pub separation: Option<AudioSeparationReport>,
    /// The message value.
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for audio separation report.
pub struct AudioSeparationReport {
    /// The status value.
    pub status: String,
    /// The model value.
    pub model: String,
    /// The output dir value.
    pub output_dir: String,
    /// The stems value.
    pub stems: Vec<AudioStemReport>,
    /// The message value.
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for audio stem report.
pub struct AudioStemReport {
    /// The stem value.
    pub stem: String,
    /// Filesystem path for this value.
    pub path: String,
    /// The bytes value.
    pub bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for text report.
pub struct TextReport {
    /// The status value.
    pub status: String,
    /// The segments processed value.
    pub segments_processed: u64,
    /// The events value.
    pub events: Vec<EventReport>,
    /// The message value.
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for event report.
pub struct EventReport {
    /// The timestamp seconds value.
    pub timestamp_seconds: Option<f64>,
    /// The analyzer value.
    pub analyzer: String,
    /// Label assigned to this value.
    pub label: String,
    /// Score assigned to this value.
    pub score: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for data bucket report.
pub struct DataBucketReport {
    /// The bucket index value.
    pub bucket_index: u64,
    /// The records value.
    pub records: u64,
    /// The estimated bytes value.
    pub estimated_bytes: u64,
    /// The streams value.
    pub streams: Vec<StreamBucketReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for stream bucket report.
pub struct StreamBucketReport {
    /// The stream identifier value.
    pub stream_id: String,
    /// The records value.
    pub records: u64,
    /// The estimated bytes value.
    pub estimated_bytes: u64,
    /// The payload counts value.
    pub payload_counts: BTreeMap<String, u64>,
    /// The video frames value.
    pub video_frames: u64,
    /// The audio frames value.
    pub audio_frames: u64,
    /// The text segments value.
    pub text_segments: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for youtube collection report.
pub struct YoutubeCollectionReport {
    #[serde(alias = "use_case")]
    /// The workflow value.
    pub workflow: String,
    /// The collection value.
    pub collection: CollectionSummary,
    /// The source value.
    pub source: YoutubeCollectionSourceReport,
    /// The filters value.
    pub filters: AppliedCollectionFilters,
    /// The items value.
    pub items: Vec<CollectionItemReport>,
    /// The assets value.
    pub assets: CollectionAssetReport,
    /// The capabilities value.
    pub capabilities: CapabilityReport,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for collection summary.
pub struct CollectionSummary {
    /// The collection identifier value.
    pub collection_id: String,
    /// The title value.
    pub title: Option<String>,
    /// The source URL value.
    pub source_url: String,
    /// The discovered items value.
    pub discovered_items: u64,
    /// The selected items value.
    pub selected_items: u64,
    /// The analyzed items value.
    pub analyzed_items: u64,
    /// The failed items value.
    pub failed_items: u64,
    /// The filtered items value.
    pub filtered_items: u64,
    /// The skipped items value.
    pub skipped_items: u64,
    /// The downloaded items value.
    pub downloaded_items: u64,
    /// The total duration seconds value.
    pub total_duration_seconds: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for youtube collection source report.
pub struct YoutubeCollectionSourceReport {
    /// The kind value.
    pub kind: String,
    /// URL associated with this value.
    pub url: String,
    /// The title value.
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for applied collection filters.
pub struct AppliedCollectionFilters {
    /// Metadata associated with this value.
    pub metadata: Vec<String>,
    /// The local file value.
    pub local_file: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for collection asset report.
pub struct CollectionAssetReport {
    /// The work dir value.
    pub work_dir: String,
    /// The report path value.
    pub report_path: String,
    /// The manifest path value.
    pub manifest_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for collection item report.
pub struct CollectionItemReport {
    /// The item identifier value.
    pub item_id: String,
    /// The youtube identifier value.
    pub youtube_id: Option<String>,
    /// The playlist index value.
    pub playlist_index: Option<u64>,
    /// The title value.
    pub title: Option<String>,
    /// The source URL value.
    pub source_url: String,
    /// The local video value.
    pub local_video: Option<String>,
    /// The status value.
    pub status: CollectionItemStatus,
    /// The error value.
    pub error: Option<String>,
    /// The report value.
    pub report: Option<YoutubeVideoReport>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
/// Variants describing collection item status.
pub enum CollectionItemStatus {
    /// The discovered variant.
    Discovered,
    /// The metadata filtered variant.
    MetadataFiltered,
    /// The downloaded variant.
    Downloaded,
    /// The file filtered variant.
    FileFiltered,
    /// The analyzed variant.
    Analyzed,
    /// The failed variant.
    Failed,
    /// The skipped existing variant.
    SkippedExisting,
}

/// Runs youtube video.
pub fn run_youtube_video(args: YoutubeVideoRequest) -> Result<YoutubeVideoReport> {
    run_youtube_video_with_progress(args, &mut |_| {})
}

/// Runs youtube video with progress.
pub fn run_youtube_video_with_progress(
    args: YoutubeVideoRequest,
    progress: &mut dyn FnMut(WhisperCppProgressEvent),
) -> Result<YoutubeVideoReport> {
    fs::create_dir_all(&args.work_dir)?;
    let report_path = args
        .output
        .clone()
        .unwrap_or_else(|| args.work_dir.join("analysis.json"));

    let mut completed = Vec::new();
    let mut skipped = Vec::new();

    let video_path = match &args.input {
        Some(path) => path.clone(),
        None => {
            let url = args
                .url
                .as_deref()
                .ok_or_else(|| DetectError::InvalidArgument("missing --url".to_string()))?;
            let path = download_youtube_video(url, &args.work_dir)?;
            completed.push("youtube_download".to_string());
            path
        }
    };

    let mut data_buckets = Vec::new();
    let mut aggregator = BucketAggregator::new(BucketConfig::record_count(1_000)?)?;
    let (video_report, observations) =
        analyze_video(&args, &video_path, &mut aggregator, &mut data_buckets)?;
    completed.push("scene_detection".to_string());
    if args.object_command.is_some() {
        completed.push("object_person_detection".to_string());
    } else {
        skipped.push("object/person detection: pass --object-command".to_string());
    }
    if args.ocr_command.is_some() {
        completed.push("ocr".to_string());
    } else {
        skipped.push("on-screen text OCR: pass --ocr-command".to_string());
    }

    let (transcription, mut audio_wav) = transcribe_video(&args, &video_path, progress);
    match transcription.status.as_str() {
        "completed" => completed.push("transcription".to_string()),
        _ => skipped.push(format!(
            "transcription: {}",
            transcription.message.as_deref().unwrap_or("not available")
        )),
    }

    let separation_report = if args.audio_separation.enabled {
        let audio_path = match &audio_wav {
            Some(path) => path.clone(),
            None => {
                let extracted = extract_audio_wav(&video_path, &args.work_dir)?;
                audio_wav = Some(extracted.clone());
                extracted
            }
        };
        match run_audio_separation(&args.audio_separation, &audio_path, &args.work_dir) {
            Ok(report) => {
                if report.status == "completed" {
                    completed.push("audio_separation".to_string());
                } else {
                    skipped.push(format!(
                        "audio separation: {}",
                        report.message.as_deref().unwrap_or("not available")
                    ));
                }
                Some(report)
            }
            Err(err) => {
                skipped.push(format!("audio separation: {err}"));
                Some(AudioSeparationReport {
                    status: "skipped".to_string(),
                    model: args
                        .audio_separation
                        .model
                        .clone()
                        .unwrap_or_else(|| DemucsModel::default().to_string()),
                    output_dir: display_path(
                        &args
                            .audio_separation
                            .output_dir
                            .clone()
                            .unwrap_or_else(|| args.work_dir.join("separated")),
                    ),
                    stems: Vec::new(),
                    message: Some(err.to_string()),
                })
            }
        }
    } else {
        None
    };

    let audio_report = match analyze_audio(&video_path, &mut aggregator, &mut data_buckets) {
        Ok(report) => {
            completed.push("audio_analysis".to_string());
            report
        }
        Err(err) => AudioReport {
            status: "skipped".to_string(),
            frames_processed: 0,
            events: Vec::new(),
            separation: separation_report.clone(),
            message: Some(err.to_string()),
        },
    };
    let mut audio_report = audio_report;
    audio_report.separation = separation_report;
    if audio_report.status != "completed" {
        skipped.push(format!(
            "audio analysis: {}",
            audio_report.message.as_deref().unwrap_or("not available")
        ));
    }

    let text_report =
        match analyze_transcript_text(&args, &transcription, &mut aggregator, &mut data_buckets) {
            Ok(report) => {
                if report.status == "completed" {
                    completed.push("transcript_text_analysis".to_string());
                }
                report
            }
            Err(err) => TextReport {
                status: "skipped".to_string(),
                segments_processed: 0,
                events: Vec::new(),
                message: Some(err.to_string()),
            },
        };
    if text_report.status != "completed" {
        skipped.push(format!(
            "transcript text analysis: {}",
            text_report.message.as_deref().unwrap_or("not available")
        ));
    }

    if let Some(bucket) = aggregator.finish() {
        data_buckets.push(bucket_report(&bucket));
    }

    let report = YoutubeVideoReport {
        use_case: YOUTUBE_VIDEO_USE_CASE.to_string(),
        source: SourceReport {
            url: args.url.clone(),
            local_video: display_path(&video_path),
        },
        assets: AssetReport {
            work_dir: display_path(&args.work_dir),
            report_path: display_path(&report_path),
            audio_wav: audio_wav.as_ref().map(|path| display_path(path)),
        },
        capabilities: CapabilityReport { completed, skipped },
        video: video_report_with_observations(video_report, observations),
        transcription,
        audio: audio_report,
        text: text_report,
        data_buckets,
    };

    Ok(report)
}

/// Runs youtube video workflow.
pub fn run_youtube_video_workflow(
    request: YoutubeRunRequest,
    work_dir: PathBuf,
    report_path: PathBuf,
) -> Result<YoutubeVideoReport> {
    run_youtube_video_workflow_with_progress(request, work_dir, report_path, &mut |_| {})
}

/// Runs youtube video workflow with progress.
pub fn run_youtube_video_workflow_with_progress(
    request: YoutubeRunRequest,
    work_dir: PathBuf,
    report_path: PathBuf,
    progress: &mut dyn FnMut(WhisperCppProgressEvent),
) -> Result<YoutubeVideoReport> {
    let request = YoutubeRunRequest {
        transcription: request.transcription.normalized(),
        ..request
    };
    request.validate()?;
    let mut args = YoutubeVideoRequest {
        work_dir: work_dir.clone(),
        output: Some(report_path.clone()),
        scene_threshold: request.scene_threshold,
        min_scene_len: request.min_scene_len,
        max_frames: request.max_frames,
        visual_sample_every: request.visual_sample_every,
        skip_transcription: !request.transcription.enabled,
        transcriber_engine: request.transcription.engine,
        whisper_cpp: request.transcription.whisper_cpp.clone(),
        audio_separation: request.audio_separation.clone(),
        ..YoutubeVideoRequest::default()
    };
    match request.source {
        SourceSpec::YoutubeUrl { url } => args.url = Some(url),
        SourceSpec::LocalFile { path } => args.input = Some(path),
    }
    if let Some(config) = request.transcription.command {
        args.transcriber_command = Some(config.command);
        args.transcriber_args = config.args;
    }
    if let Some(config) = request.models.object {
        args.object_command = Some(config.command);
        args.object_args = config.args;
    }
    if let Some(config) = request.models.ocr {
        args.ocr_command = Some(config.command);
        args.ocr_args = config.args;
    }
    if let Some(config) = request.models.text {
        args.text_command = Some(config.command);
        args.text_args = config.args;
    }

    let report = run_youtube_video_with_progress(args, progress)?;
    write_youtube_video_report(&report_path, &report)?;
    Ok(report)
}

/// Writes youtube video report.
pub fn write_youtube_video_report(path: &Path, report: &YoutubeVideoReport) -> Result<()> {
    write_json_report(path, report)
}

/// Returns discover youtube collection manifest.
pub fn discover_youtube_collection_manifest(
    request: YoutubeCollectionManifestRequest,
) -> Result<YoutubeCollectionManifest> {
    request.validate()?;
    let mut manifest = discover_collection_manifest(&request.source, request.max_items)?;
    let needs_enrichment = request.metadata_depth == MetadataDepth::Full
        || (request.metadata_depth == MetadataDepth::EnrichedWhenNeeded
            && manifest_filter_needs_enrichment(&request.metadata_filter));
    if needs_enrichment {
        enrich_manifest_items(&mut manifest)?;
    }
    manifest
        .items
        .retain(|item| metadata_filter_matches(item, &request.metadata_filter));
    Ok(manifest)
}

/// Runs youtube collection workflow.
pub fn run_youtube_collection_workflow(
    request: YoutubeCollectionRunRequest,
    work_dir: PathBuf,
    report_path: PathBuf,
) -> Result<YoutubeCollectionReport> {
    run_youtube_collection_workflow_with_progress(request, work_dir, report_path, &mut |_| {})
}

/// Runs youtube collection workflow with progress.
pub fn run_youtube_collection_workflow_with_progress(
    request: YoutubeCollectionRunRequest,
    work_dir: PathBuf,
    report_path: PathBuf,
    progress: &mut dyn FnMut(WhisperCppProgressEvent),
) -> Result<YoutubeCollectionReport> {
    let request = YoutubeCollectionRunRequest {
        analysis: YoutubeRunOptions {
            transcription: request.analysis.transcription.clone().normalized(),
            ..request.analysis.clone()
        },
        ..request
    };
    request.validate()?;
    fs::create_dir_all(&work_dir)?;

    let mut manifest = match request.manifest.clone() {
        Some(manifest) => manifest,
        None => {
            let source = request.source.clone().ok_or_else(|| {
                DetectError::InvalidArgument("source or manifest is required".to_string())
            })?;
            discover_collection_manifest(&source, request.max_items)?
        }
    };
    if manifest_filter_needs_enrichment(&request.metadata_filter) {
        enrich_manifest_items(&mut manifest)?;
    }

    let manifest_path = work_dir.join("manifest.json");
    write_json_report(&manifest_path, &manifest)?;

    let mut completed = vec!["collection_discovery".to_string()];
    let mut skipped = Vec::new();
    let mut item_reports = Vec::new();
    let mut selected_items = 0_u64;
    let mut downloaded_items = 0_u64;

    for item in &manifest.items {
        if !metadata_filter_matches(item, &request.metadata_filter) {
            item_reports.push(collection_item_report(
                item,
                None,
                CollectionItemStatus::MetadataFiltered,
                Some("filtered by metadata".to_string()),
                None,
            ));
            continue;
        }

        let item_dir = work_dir.join("items").join(&item.item_id);
        let media_dir = item_dir.join("media");
        let existing = find_existing_item_video(&media_dir);
        if existing.is_some()
            && request.metadata_filter.already_downloaded == AlreadyDownloadedPolicy::Skip
        {
            item_reports.push(collection_item_report(
                item,
                existing.as_ref(),
                CollectionItemStatus::SkippedExisting,
                Some("already downloaded".to_string()),
                None,
            ));
            continue;
        }

        selected_items += 1;
        let mut local_video = existing;
        if local_video.is_none() {
            match download_youtube_video_to_dir(&item.source_url, &media_dir, &item.item_id) {
                Ok(path) => {
                    downloaded_items += 1;
                    local_video = Some(path);
                }
                Err(error) => {
                    item_reports.push(collection_item_report(
                        item,
                        None,
                        CollectionItemStatus::Failed,
                        Some(error.to_string()),
                        None,
                    ));
                    continue;
                }
            }
        }

        let Some(local_video_path) = local_video.clone() else {
            item_reports.push(collection_item_report(
                item,
                None,
                CollectionItemStatus::Failed,
                Some("download did not produce a local video".to_string()),
                None,
            ));
            continue;
        };

        match local_file_filter_matches(&local_video_path, &request.local_file_filter) {
            Ok(true) => {}
            Ok(false) => {
                if request.delete_filtered_downloads {
                    let _ = fs::remove_file(&local_video_path);
                }
                item_reports.push(collection_item_report(
                    item,
                    Some(&local_video_path),
                    CollectionItemStatus::FileFiltered,
                    Some("filtered by local file metadata".to_string()),
                    None,
                ));
                continue;
            }
            Err(error) => match request.local_file_filter.failed_probe_behavior {
                FailedProbeBehavior::Include => {}
                FailedProbeBehavior::Filtered => {
                    item_reports.push(collection_item_report(
                        item,
                        Some(&local_video_path),
                        CollectionItemStatus::FileFiltered,
                        Some(error.to_string()),
                        None,
                    ));
                    continue;
                }
                FailedProbeBehavior::Failed => {
                    item_reports.push(collection_item_report(
                        item,
                        Some(&local_video_path),
                        CollectionItemStatus::Failed,
                        Some(error.to_string()),
                        None,
                    ));
                    continue;
                }
            },
        }

        let analysis_work_dir = item_dir.join("analysis-work");
        let item_report_path = item_dir.join("analysis.json");
        let item_request = request.analysis.to_request(
            SourceSpec::LocalFile {
                path: local_video_path.clone(),
            },
            analysis_work_dir.clone(),
        );
        match run_youtube_video_workflow_with_progress(
            item_request,
            analysis_work_dir,
            item_report_path,
            progress,
        ) {
            Ok(report) => item_reports.push(collection_item_report(
                item,
                Some(&local_video_path),
                CollectionItemStatus::Analyzed,
                None,
                Some(report),
            )),
            Err(error) => item_reports.push(collection_item_report(
                item,
                Some(&local_video_path),
                CollectionItemStatus::Failed,
                Some(error.to_string()),
                None,
            )),
        }
    }

    let analyzed_items = item_reports
        .iter()
        .filter(|item| item.status == CollectionItemStatus::Analyzed)
        .count() as u64;
    let failed_items = item_reports
        .iter()
        .filter(|item| item.status == CollectionItemStatus::Failed)
        .count() as u64;
    let filtered_items = item_reports
        .iter()
        .filter(|item| {
            matches!(
                item.status,
                CollectionItemStatus::MetadataFiltered | CollectionItemStatus::FileFiltered
            )
        })
        .count() as u64;
    let skipped_items = item_reports
        .iter()
        .filter(|item| item.status == CollectionItemStatus::SkippedExisting)
        .count() as u64;

    if analyzed_items > 0 {
        completed.push("collection_analysis".to_string());
    }
    if failed_items > 0 {
        skipped.push(format!("{failed_items} collection items failed"));
    }
    if filtered_items > 0 {
        skipped.push(format!("{filtered_items} collection items filtered"));
    }
    if skipped_items > 0 {
        skipped.push(format!("{skipped_items} existing collection items skipped"));
    }

    let total_duration_seconds = item_reports
        .iter()
        .filter_map(|item| item.report.as_ref()?.video.duration_seconds)
        .reduce(|left, right| left + right);

    let report = YoutubeCollectionReport {
        workflow: "youtube_collection_analysis".to_string(),
        collection: CollectionSummary {
            collection_id: manifest.collection_id.clone(),
            title: manifest.title.clone(),
            source_url: manifest.source.url().to_string(),
            discovered_items: manifest.items.len() as u64,
            selected_items,
            analyzed_items,
            failed_items,
            filtered_items,
            skipped_items,
            downloaded_items,
            total_duration_seconds,
        },
        source: YoutubeCollectionSourceReport {
            kind: manifest.source.kind_name().to_string(),
            url: manifest.source.url().to_string(),
            title: manifest.title.clone(),
        },
        filters: AppliedCollectionFilters {
            metadata: describe_manifest_filter(&request.metadata_filter),
            local_file: describe_local_file_filter(&request.local_file_filter),
        },
        items: item_reports,
        assets: CollectionAssetReport {
            work_dir: display_path(&work_dir),
            report_path: display_path(&report_path),
            manifest_path: display_path(&manifest_path),
        },
        capabilities: CapabilityReport { completed, skipped },
    };

    write_json_report(&report_path, &report)?;

    if analyzed_items == 0 && failed_items > 0 {
        return Err(DetectError::Source(
            "collection analysis failed for every selected item".to_string(),
        ));
    }

    Ok(report)
}

fn download_youtube_video(url: &str, work_dir: &Path) -> Result<PathBuf> {
    let command = PathBuf::from("yt-dlp");
    if resolve_command(&command).is_none() {
        return Err(DetectError::Source(
            "yt-dlp is required for --url downloads".to_string(),
        ));
    }

    let output_template = work_dir.join("youtube-video.%(ext)s");
    let output = Command::new("yt-dlp")
        .arg("--no-playlist")
        .arg("--merge-output-format")
        .arg("mp4")
        .arg("--print")
        .arg("after_move:filepath")
        .arg("-o")
        .arg(&output_template)
        .arg(url)
        .output()?;

    if !output.status.success() {
        return Err(DetectError::Source(format!(
            "yt-dlp failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }

    for line in String::from_utf8_lossy(&output.stdout).lines().rev() {
        let path = PathBuf::from(line.trim());
        if path.exists() {
            return Ok(path);
        }
    }

    find_downloaded_video(work_dir).ok_or_else(|| {
        DetectError::Source("yt-dlp completed but no downloaded video was found".to_string())
    })
}

fn find_downloaded_video(work_dir: &Path) -> Option<PathBuf> {
    let mut candidates = fs::read_dir(work_dir)
        .ok()?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.file_stem()
                .and_then(|value| value.to_str())
                .map(|stem| stem == "youtube-video")
                .unwrap_or(false)
        })
        .filter(|path| {
            matches!(
                path.extension().and_then(|value| value.to_str()),
                Some("mp4" | "mkv" | "webm" | "mov")
            )
        })
        .collect::<Vec<_>>();
    candidates.sort();
    candidates.pop()
}

fn validate_youtube_url(url: &str) -> Result<()> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err(DetectError::InvalidArgument(
            "YouTube URL is required".to_string(),
        ));
    }
    let Some(rest) = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
    else {
        return Err(DetectError::InvalidArgument(
            "YouTube URL must use http or https".to_string(),
        ));
    };
    let host = rest
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default()
        .split('@')
        .next_back()
        .unwrap_or_default()
        .split(':')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let allowed = host == "youtu.be"
        || host == "youtube.com"
        || host == "www.youtube.com"
        || host == "m.youtube.com"
        || host.ends_with(".youtube.com");
    if !allowed {
        return Err(DetectError::InvalidArgument(format!(
            "unsupported YouTube URL host: {host}"
        )));
    }
    Ok(())
}

fn validate_local_file(path: &Path) -> Result<()> {
    if path.as_os_str().is_empty() {
        return Err(DetectError::InvalidArgument(
            "local file path is required".to_string(),
        ));
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
struct YtDlpCollectionJson {
    id: Option<String>,
    title: Option<String>,
    webpage_url: Option<String>,
    entries: Option<Vec<Option<YtDlpEntryJson>>>,
}

#[derive(Debug, Deserialize)]
struct YtDlpEntryJson {
    id: Option<String>,
    title: Option<String>,
    url: Option<String>,
    webpage_url: Option<String>,
    original_url: Option<String>,
    playlist_index: Option<u64>,
    duration: Option<f64>,
    upload_date: Option<String>,
}

fn discover_collection_manifest(
    source: &YoutubeCollectionSource,
    max_items: Option<u64>,
) -> Result<YoutubeCollectionManifest> {
    let command = PathBuf::from("yt-dlp");
    if resolve_command(&command).is_none() {
        return Err(DetectError::Source(
            "yt-dlp is required for YouTube collection discovery".to_string(),
        ));
    }

    let output = Command::new("yt-dlp")
        .arg("--flat-playlist")
        .arg("-J")
        .arg(source.url())
        .output()?;
    if !output.status.success() {
        return Err(DetectError::Source(format!(
            "yt-dlp collection discovery failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }

    let parsed: YtDlpCollectionJson = serde_json::from_slice(&output.stdout)
        .map_err(|err| DetectError::Source(err.to_string()))?;
    collection_manifest_from_yt_dlp(source.clone(), parsed, max_items)
}

fn collection_manifest_from_yt_dlp(
    source: YoutubeCollectionSource,
    parsed: YtDlpCollectionJson,
    max_items: Option<u64>,
) -> Result<YoutubeCollectionManifest> {
    let collection_id = sanitize_item_id(
        parsed
            .id
            .as_deref()
            .or(parsed.webpage_url.as_deref())
            .unwrap_or_else(|| source.url()),
    );
    let mut seen = BTreeMap::<String, u64>::new();
    let limit = max_items.unwrap_or(u64::MAX) as usize;
    let items = parsed
        .entries
        .unwrap_or_default()
        .into_iter()
        .flatten()
        .take(limit)
        .enumerate()
        .map(|(index, entry)| {
            let mut item_id = entry
                .id
                .as_deref()
                .map(sanitize_item_id)
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| format!("item-{}", index + 1));
            let count = seen.entry(item_id.clone()).or_insert(0);
            if *count > 0 {
                item_id = format!("{item_id}-{}", *count + 1);
            }
            *count += 1;
            YoutubeCollectionManifestItem {
                item_id,
                youtube_id: entry.id.clone(),
                playlist_index: entry.playlist_index.or(Some((index + 1) as u64)),
                title: entry.title.clone(),
                source_url: source_url_from_entry(&entry),
                duration_seconds: entry.duration,
                upload_date: entry.upload_date.clone(),
            }
        })
        .collect();

    Ok(YoutubeCollectionManifest {
        collection_id,
        source,
        title: parsed.title,
        items,
    })
}

fn enrich_manifest_items(manifest: &mut YoutubeCollectionManifest) -> Result<()> {
    let command = PathBuf::from("yt-dlp");
    if resolve_command(&command).is_none() {
        return Err(DetectError::Source(
            "yt-dlp is required for YouTube collection metadata enrichment".to_string(),
        ));
    }

    for item in &mut manifest.items {
        if item.duration_seconds.is_some() && item.upload_date.is_some() && item.title.is_some() {
            continue;
        }
        let output = Command::new("yt-dlp")
            .arg("--skip-download")
            .arg("--no-playlist")
            .arg("-J")
            .arg(&item.source_url)
            .output()?;
        if !output.status.success() {
            continue;
        }
        let Ok(entry) = serde_json::from_slice::<YtDlpEntryJson>(&output.stdout) else {
            continue;
        };
        if item.youtube_id.is_none() {
            item.youtube_id = entry.id;
        }
        if item.title.is_none() {
            item.title = entry.title;
        }
        if item.duration_seconds.is_none() {
            item.duration_seconds = entry.duration;
        }
        if item.upload_date.is_none() {
            item.upload_date = entry.upload_date;
        }
    }
    Ok(())
}

fn source_url_from_entry(entry: &YtDlpEntryJson) -> String {
    for value in [&entry.webpage_url, &entry.original_url, &entry.url]
        .into_iter()
        .flatten()
    {
        if value.starts_with("http://") || value.starts_with("https://") {
            return value.clone();
        }
    }
    if let Some(id) = entry.id.as_deref().or(entry.url.as_deref()) {
        return format!("https://www.youtube.com/watch?v={id}");
    }
    "https://www.youtube.com/".to_string()
}

fn download_youtube_video_to_dir(url: &str, media_dir: &Path, item_id: &str) -> Result<PathBuf> {
    let command = PathBuf::from("yt-dlp");
    if resolve_command(&command).is_none() {
        return Err(DetectError::Source(
            "yt-dlp is required for YouTube downloads".to_string(),
        ));
    }
    fs::create_dir_all(media_dir)?;
    let output_template = media_dir.join(format!("{}-%(id)s.%(ext)s", sanitize_item_id(item_id)));
    let output = Command::new("yt-dlp")
        .arg("--no-playlist")
        .arg("--merge-output-format")
        .arg("mp4")
        .arg("--print")
        .arg("after_move:filepath")
        .arg("-o")
        .arg(&output_template)
        .arg(url)
        .output()?;

    if !output.status.success() {
        return Err(DetectError::Source(format!(
            "yt-dlp failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }

    for line in String::from_utf8_lossy(&output.stdout).lines().rev() {
        let path = PathBuf::from(line.trim());
        if path.exists() {
            return Ok(path);
        }
    }

    find_existing_item_video(media_dir).ok_or_else(|| {
        DetectError::Source("yt-dlp completed but no downloaded video was found".to_string())
    })
}

fn find_existing_item_video(media_dir: &Path) -> Option<PathBuf> {
    let mut candidates = fs::read_dir(media_dir)
        .ok()?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| is_video_file(path))
        .collect::<Vec<_>>();
    candidates.sort();
    candidates.pop()
}

fn is_video_file(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|value| value.to_str())
            .map(|value| value.to_ascii_lowercase()),
        Some(value) if matches!(value.as_str(), "mp4" | "mkv" | "webm" | "mov")
    )
}

fn metadata_filter_matches(
    item: &YoutubeCollectionManifestItem,
    filter: &ManifestFilterSpec,
) -> bool {
    if let Some(needle) = filter.title_contains.as_deref().map(normalize_text_filter) {
        let title = item
            .title
            .as_deref()
            .map(normalize_text_filter)
            .unwrap_or_default();
        if !title.contains(&needle) {
            return false;
        }
    }
    let title = item
        .title
        .as_deref()
        .map(normalize_text_filter)
        .unwrap_or_default();
    for excluded in &filter.title_excludes {
        if !excluded.trim().is_empty() && title.contains(&normalize_text_filter(excluded)) {
            return false;
        }
    }
    if let Some(min) = filter.playlist_index_min {
        if item.playlist_index.unwrap_or(0) < min {
            return false;
        }
    }
    if let Some(max) = filter.playlist_index_max {
        if item.playlist_index.unwrap_or(u64::MAX) > max {
            return false;
        }
    }
    if let Some(min) = filter.duration_seconds_min {
        if item
            .duration_seconds
            .map(|duration| duration < min)
            .unwrap_or(true)
        {
            return false;
        }
    }
    if let Some(max) = filter.duration_seconds_max {
        if item
            .duration_seconds
            .map(|duration| duration > max)
            .unwrap_or(true)
        {
            return false;
        }
    }
    if let Some(from) = normalize_upload_date_filter(filter.upload_date_from.as_deref()) {
        if item
            .upload_date
            .as_deref()
            .and_then(|value| normalize_upload_date_filter(Some(value)))
            .map(|value| value < from)
            .unwrap_or(true)
        {
            return false;
        }
    }
    if let Some(to) = normalize_upload_date_filter(filter.upload_date_to.as_deref()) {
        if item
            .upload_date
            .as_deref()
            .and_then(|value| normalize_upload_date_filter(Some(value)))
            .map(|value| value > to)
            .unwrap_or(true)
        {
            return false;
        }
    }
    true
}

fn local_file_filter_matches(path: &Path, filter: &LocalFileFilterSpec) -> Result<bool> {
    let metadata = fs::metadata(path)?;
    if let Some(min) = filter.file_size_bytes_min {
        if metadata.len() < min {
            return Ok(false);
        }
    }
    if let Some(max) = filter.file_size_bytes_max {
        if metadata.len() > max {
            return Ok(false);
        }
    }
    if !filter.extension_allowlist.is_empty() {
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| value.trim_start_matches('.').to_ascii_lowercase())
            .unwrap_or_default();
        let allowed = filter.extension_allowlist.iter().any(|allowed| {
            allowed
                .trim()
                .trim_start_matches('.')
                .eq_ignore_ascii_case(&extension)
        });
        if !allowed {
            return Ok(false);
        }
    }

    if filter.duration_seconds_min.is_some() || filter.duration_seconds_max.is_some() {
        let duration = probe_local_video_duration(path)?.ok_or_else(|| {
            DetectError::Source(format!(
                "could not read local video duration: {}",
                display_path(path)
            ))
        })?;
        if let Some(min) = filter.duration_seconds_min {
            if duration < min {
                return Ok(false);
            }
        }
        if let Some(max) = filter.duration_seconds_max {
            if duration > max {
                return Ok(false);
            }
        }
    }

    Ok(true)
}

fn probe_local_video_duration(path: &Path) -> Result<Option<f64>> {
    let source = FfmpegVideoSource::open(path)?;
    Ok(source.metadata().duration_seconds)
}

fn collection_item_report(
    item: &YoutubeCollectionManifestItem,
    local_video: Option<&PathBuf>,
    status: CollectionItemStatus,
    error: Option<String>,
    report: Option<YoutubeVideoReport>,
) -> CollectionItemReport {
    CollectionItemReport {
        item_id: item.item_id.clone(),
        youtube_id: item.youtube_id.clone(),
        playlist_index: item.playlist_index,
        title: item.title.clone(),
        source_url: item.source_url.clone(),
        local_video: local_video.map(|path| display_path(path)),
        status,
        error,
        report,
    }
}

fn validate_manifest_filter(filter: &ManifestFilterSpec) -> Result<()> {
    validate_optional_nonnegative_f64(filter.duration_seconds_min, "duration_seconds_min")?;
    validate_optional_nonnegative_f64(filter.duration_seconds_max, "duration_seconds_max")?;
    if let (Some(min), Some(max)) = (filter.duration_seconds_min, filter.duration_seconds_max) {
        if min > max {
            return Err(DetectError::InvalidArgument(
                "duration_seconds_min must be less than or equal to duration_seconds_max"
                    .to_string(),
            ));
        }
    }
    if let (Some(min), Some(max)) = (filter.playlist_index_min, filter.playlist_index_max) {
        if min > max {
            return Err(DetectError::InvalidArgument(
                "playlist_index_min must be less than or equal to playlist_index_max".to_string(),
            ));
        }
    }
    Ok(())
}

fn validate_local_file_filter(filter: &LocalFileFilterSpec) -> Result<()> {
    validate_optional_nonnegative_f64(filter.duration_seconds_min, "duration_seconds_min")?;
    validate_optional_nonnegative_f64(filter.duration_seconds_max, "duration_seconds_max")?;
    if let (Some(min), Some(max)) = (filter.duration_seconds_min, filter.duration_seconds_max) {
        if min > max {
            return Err(DetectError::InvalidArgument(
                "local duration_seconds_min must be less than or equal to local duration_seconds_max"
                    .to_string(),
            ));
        }
    }
    if let (Some(min), Some(max)) = (filter.file_size_bytes_min, filter.file_size_bytes_max) {
        if min > max {
            return Err(DetectError::InvalidArgument(
                "file_size_bytes_min must be less than or equal to file_size_bytes_max".to_string(),
            ));
        }
    }
    Ok(())
}

fn validate_optional_nonnegative_f64(value: Option<f64>, name: &str) -> Result<()> {
    if let Some(value) = value {
        if !value.is_finite() || value < 0.0 {
            return Err(DetectError::InvalidArgument(format!(
                "{name} must be finite and non-negative"
            )));
        }
    }
    Ok(())
}

fn validate_analysis_options(
    scene_threshold: f32,
    min_scene_len: u64,
    visual_sample_every: u64,
) -> Result<()> {
    if !scene_threshold.is_finite() || scene_threshold <= 0.0 {
        return Err(DetectError::InvalidArgument(
            "scene_threshold must be finite and positive".to_string(),
        ));
    }
    if min_scene_len == 0 {
        return Err(DetectError::InvalidArgument(
            "min_scene_len must be positive".to_string(),
        ));
    }
    if visual_sample_every == 0 {
        return Err(DetectError::InvalidArgument(
            "visual_sample_every must be positive".to_string(),
        ));
    }
    Ok(())
}

fn manifest_filter_needs_enrichment(filter: &ManifestFilterSpec) -> bool {
    filter.duration_seconds_min.is_some()
        || filter.duration_seconds_max.is_some()
        || filter.upload_date_from.is_some()
        || filter.upload_date_to.is_some()
}

fn describe_manifest_filter(filter: &ManifestFilterSpec) -> Vec<String> {
    let mut descriptions = Vec::new();
    if let Some(value) = filter
        .title_contains
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        descriptions.push(format!("title contains '{value}'"));
    }
    for value in &filter.title_excludes {
        if !value.trim().is_empty() {
            descriptions.push(format!("title excludes '{value}'"));
        }
    }
    if let Some(value) = filter.playlist_index_min {
        descriptions.push(format!("playlist index >= {value}"));
    }
    if let Some(value) = filter.playlist_index_max {
        descriptions.push(format!("playlist index <= {value}"));
    }
    if let Some(value) = filter.upload_date_from.as_deref() {
        descriptions.push(format!("upload date >= {value}"));
    }
    if let Some(value) = filter.upload_date_to.as_deref() {
        descriptions.push(format!("upload date <= {value}"));
    }
    if let Some(value) = filter.duration_seconds_min {
        descriptions.push(format!("duration >= {value}s"));
    }
    if let Some(value) = filter.duration_seconds_max {
        descriptions.push(format!("duration <= {value}s"));
    }
    descriptions.push(format!(
        "already downloaded: {:?}",
        filter.already_downloaded
    ));
    descriptions
}

fn describe_local_file_filter(filter: &LocalFileFilterSpec) -> Vec<String> {
    let mut descriptions = Vec::new();
    if let Some(value) = filter.duration_seconds_min {
        descriptions.push(format!("local duration >= {value}s"));
    }
    if let Some(value) = filter.duration_seconds_max {
        descriptions.push(format!("local duration <= {value}s"));
    }
    if let Some(value) = filter.file_size_bytes_min {
        descriptions.push(format!("file size >= {value} bytes"));
    }
    if let Some(value) = filter.file_size_bytes_max {
        descriptions.push(format!("file size <= {value} bytes"));
    }
    if !filter.extension_allowlist.is_empty() {
        descriptions.push(format!(
            "extensions: {}",
            filter.extension_allowlist.join(", ")
        ));
    }
    descriptions.push(format!("failed probe: {:?}", filter.failed_probe_behavior));
    descriptions
}

fn normalize_text_filter(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn normalize_upload_date_filter(value: Option<&str>) -> Option<String> {
    let normalized = value?
        .chars()
        .filter(|ch| ch.is_ascii_digit())
        .collect::<String>();
    if normalized.len() == 8 {
        Some(normalized)
    } else {
        None
    }
}

fn sanitize_item_id(value: &str) -> String {
    let mut sanitized = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    while sanitized.contains("--") {
        sanitized = sanitized.replace("--", "-");
    }
    if sanitized.is_empty() {
        "collection".to_string()
    } else {
        sanitized
    }
}

fn analyze_video(
    args: &YoutubeVideoRequest,
    video_path: &Path,
    aggregator: &mut BucketAggregator,
    data_buckets: &mut Vec<DataBucketReport>,
) -> Result<(VideoReport, Vec<ObservationReport>)> {
    let mut source = FfmpegVideoSource::open(video_path)?;
    let metadata = source.metadata().clone();
    let mut pipeline = build_video_pipeline(args)?;

    while let Some(frame) = source.next_video_frame()? {
        let frame_ref = frame.as_frame();
        let record = DataRecord::video_frame("video", frame.position.frame_index, &frame_ref);
        push_record(aggregator, record, data_buckets)?;
        pipeline.process_frame(frame)?;
        if args
            .max_frames
            .map(|limit| pipeline.frames_processed() >= limit)
            .unwrap_or(false)
        {
            break;
        }
    }

    let result = pipeline.finish_analysis()?;
    let observations = result
        .observations
        .iter()
        .map(observation_report)
        .collect::<Vec<_>>();
    let scenes = result
        .detection
        .scenes
        .iter()
        .enumerate()
        .map(|(index, scene)| {
            let scene_observations = observations
                .iter()
                .filter(|observation| observation.scene_index == Some(index as u64))
                .cloned()
                .collect();
            SceneReport {
                index: index as u64,
                start_frame: scene.start.frame_index,
                end_frame: scene.end.frame_index,
                start_seconds: scene.start.timestamp.seconds(),
                end_seconds: scene.end.timestamp.seconds(),
                observations: scene_observations,
            }
        })
        .collect::<Vec<_>>();

    Ok((
        VideoReport {
            width: metadata.width,
            height: metadata.height,
            frame_rate: format!(
                "{}/{}",
                metadata.frame_rate.numer(),
                metadata.frame_rate.denom()
            ),
            duration_seconds: metadata.duration_seconds,
            frames_processed: result.frames_processed,
            scenes,
            observations: Vec::new(),
        },
        observations,
    ))
}

fn build_video_pipeline(args: &YoutubeVideoRequest) -> Result<RealtimeVideoPipeline> {
    let mut builder = RealtimeVideoPipeline::builder()
        .scene_detector(ContentDetector::new(
            args.scene_threshold,
            args.min_scene_len,
        ))
        .start_in_scene(true);

    if let Some(command) = &args.object_command {
        let backend = ExternalCommandModel::new(
            command,
            downloaded_external_model("object-command", ModelTask::ObjectDetection),
        )
        .args(args.object_args.clone());
        builder = builder.video_analyzer(SampledVideoAnalyzer::new(
            ModelVideoAnalyzer::new("object_person_detector", backend),
            args.visual_sample_every,
        ));
    }

    if let Some(command) = &args.ocr_command {
        let backend = ExternalCommandModel::new(
            command,
            downloaded_external_model("ocr-command", ModelTask::Custom("ocr".to_string())),
        )
        .args(args.ocr_args.clone());
        builder = builder.video_analyzer(SampledVideoAnalyzer::new(
            ModelVideoAnalyzer::new("ocr", backend),
            args.visual_sample_every,
        ));
    }

    builder.build()
}

fn transcribe_video(
    args: &YoutubeVideoRequest,
    video_path: &Path,
    progress: &mut dyn FnMut(WhisperCppProgressEvent),
) -> (TranscriptionReport, Option<PathBuf>) {
    if args.skip_transcription {
        return (
            TranscriptionReport {
                status: "skipped".to_string(),
                text: None,
                segments: Vec::new(),
                message: Some("disabled by --skip-transcription".to_string()),
            },
            None,
        );
    }

    match extract_audio_wav(video_path, &args.work_dir).and_then(|audio_path| {
        if args.transcriber_engine == TranscriptionEngine::WhisperCpp {
            run_whisper_cpp(&args.whisper_cpp, &audio_path, progress)
        } else {
            let command = match &args.transcriber_command {
                Some(command) => command.clone(),
                None => {
                    let default = PathBuf::from("whisper");
                    if resolve_command(&default).is_none() {
                        return Err(DetectError::Source(
                            "install whisper or pass --transcriber-command".to_string(),
                        ));
                    }
                    default
                }
            };
            run_whisper_command(&command, &args.transcriber_args, &audio_path)
        }
    }) {
        Ok((report, audio_path)) => (report, Some(audio_path)),
        Err(err) => (
            TranscriptionReport {
                status: "skipped".to_string(),
                text: None,
                segments: Vec::new(),
                message: Some(err.to_string()),
            },
            None,
        ),
    }
}

fn extract_audio_wav(video_path: &Path, work_dir: &Path) -> Result<PathBuf> {
    extract_wav(video_path, work_dir.join("audio.wav"), 16_000)
}

fn run_audio_separation(
    config: &AudioSeparationConfig,
    audio_path: &Path,
    work_dir: &Path,
) -> Result<AudioSeparationReport> {
    let model = config
        .model
        .as_deref()
        .map(DemucsModel::from_str)
        .transpose()?
        .unwrap_or_default();
    let output_dir = config
        .output_dir
        .clone()
        .unwrap_or_else(|| work_dir.join("separated"));
    let mut options = HtdemucsOptions::new(&output_dir).model(model.clone());
    if let Some(command) = &config.command {
        options = options.command(command.command.clone());
        for arg in &command.args {
            options = options.command_arg(arg.clone());
        }
    }
    if let Some(two_stems) = &config.two_stems {
        options = options.two_stems(Stem::from_str(two_stems)?);
    }
    if let Some(device) = &config.device {
        options = options.device(device.clone());
    }

    let separator = HtdemucsSeparator::new(options)?;
    let result = separator.separate(audio_path)?;
    Ok(AudioSeparationReport {
        status: "completed".to_string(),
        model: result.model.to_string(),
        output_dir: display_path(&result.output_dir),
        stems: result
            .stems
            .into_iter()
            .filter(|stem| stem.exists)
            .map(|stem| AudioStemReport {
                stem: stem.stem.to_string(),
                path: display_path(&stem.path),
                bytes: stem.bytes,
            })
            .collect(),
        message: None,
    })
}

fn run_whisper_cpp(
    config: &WhisperCppConfig,
    audio_path: &Path,
    progress: &mut dyn FnMut(WhisperCppProgressEvent),
) -> Result<(TranscriptionReport, PathBuf)> {
    let mut transcriber = WhisperCppTranscriber::new(config.clone());
    let parsed = transcriber
        .transcribe_with_progress(audio_path, progress)
        .map_err(|err| DetectError::Source(err.to_string()))?;
    let segments = parsed
        .segments
        .into_iter()
        .map(|segment| TranscriptSegmentReport {
            index: segment.index,
            start_seconds: segment.start_seconds,
            end_seconds: segment.end_seconds,
            text: segment.text.trim().to_string(),
        })
        .collect::<Vec<_>>();
    Ok((
        TranscriptionReport {
            status: "completed".to_string(),
            text: parsed.text.map(|text| text.trim().to_string()),
            segments,
            message: parsed.source,
        },
        audio_path.to_path_buf(),
    ))
}

fn run_whisper_command(
    command: &Path,
    extra_args: &[String],
    audio_path: &Path,
) -> Result<(TranscriptionReport, PathBuf)> {
    let output_dir = audio_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("transcript");
    fs::create_dir_all(&output_dir)?;

    let mut transcriber = WhisperCliTranscriber::new(command)
        .args(extra_args.iter().cloned())
        .output_dir(output_dir);
    let parsed = transcriber
        .transcribe(audio_path)
        .map_err(|err| DetectError::Source(err.to_string()))?;
    let segments = parsed
        .segments
        .into_iter()
        .map(|segment| TranscriptSegmentReport {
            index: segment.index,
            start_seconds: segment.start_seconds,
            end_seconds: segment.end_seconds,
            text: segment.text.trim().to_string(),
        })
        .collect::<Vec<_>>();
    Ok((
        TranscriptionReport {
            status: "completed".to_string(),
            text: parsed.text.map(|text| text.trim().to_string()),
            segments,
            message: parsed.source,
        },
        audio_path.to_path_buf(),
    ))
}

fn analyze_audio(
    video_path: &Path,
    aggregator: &mut BucketAggregator,
    data_buckets: &mut Vec<DataBucketReport>,
) -> Result<AudioReport> {
    let mut source = FfmpegAudioSource::open_path_with_options(
        video_path,
        FfmpegAudioSourceOptions::recorded().samples_per_chunk(16_384),
    )?;
    let mut pipeline = video_analysis_core::AudioPipeline::builder()
        .analyzer(AudioEnergyAnalyzer::default())
        .build()?;
    while let Some(frame) = source.next_audio_frame()? {
        let frame_ref = frame.as_frame()?;
        let record = DataRecord::audio_frame("audio", pipeline.frames_processed(), &frame_ref);
        push_record(aggregator, record, data_buckets)?;
        pipeline.process_frame(frame)?;
    }

    let result = pipeline.finish_analysis()?;
    Ok(AudioReport {
        status: "completed".to_string(),
        frames_processed: result.frames_processed,
        events: result.events.iter().map(event_report).collect(),
        separation: None,
        message: None,
    })
}

fn analyze_transcript_text(
    args: &YoutubeVideoRequest,
    transcription: &TranscriptionReport,
    aggregator: &mut BucketAggregator,
    data_buckets: &mut Vec<DataBucketReport>,
) -> Result<TextReport> {
    if transcription.status != "completed" {
        return Ok(TextReport {
            status: "skipped".to_string(),
            segments_processed: 0,
            events: Vec::new(),
            message: Some("no transcript available".to_string()),
        });
    }

    let mut builder =
        video_analysis_core::TextPipeline::builder().analyzer(TranscriptHeuristicAnalyzer);
    if let Some(command) = &args.text_command {
        let backend = ExternalCommandModel::new(
            command,
            downloaded_external_model(
                "text-command",
                ModelTask::Custom("text_classification".to_string()),
            ),
        )
        .args(args.text_args.clone());
        builder = builder.analyzer(ModelTextAnalyzer::new("transcript_text_model", backend));
    }
    let mut pipeline = builder.build()?;

    for segment in &transcription.segments {
        let owned = segment_to_owned_text_segment(&TranscriptSegment {
            index: segment.index,
            start_seconds: segment.start_seconds,
            end_seconds: segment.end_seconds,
            text: segment.text.clone(),
            language: None,
            speaker: None,
            confidence: None,
            is_final: true,
        });
        let segment_ref = owned.as_segment();
        push_record(
            aggregator,
            DataRecord::text_segment("transcript", &segment_ref),
            data_buckets,
        )?;
        pipeline.process_segment(owned)?;
    }

    let result = pipeline.finish_analysis()?;
    Ok(TextReport {
        status: "completed".to_string(),
        segments_processed: result.segments_processed,
        events: result.events.iter().map(event_report).collect(),
        message: None,
    })
}

fn downloaded_external_model(name: &str, task: ModelTask) -> DownloadedModel {
    DownloadedModel {
        spec: HuggingFaceModelSpec::new(name, task).name(name),
        files: BTreeMap::new(),
    }
}

fn observation_report(observation: &Observation) -> ObservationReport {
    ObservationReport {
        timestamp_seconds: observation.timestamp.map(|timestamp| timestamp.seconds()),
        frame_index: observation.frame.map(|position| position.frame_index),
        scene_index: observation.scene_index,
        analyzer: observation.analyzer.clone(),
        kind: observation_kind_name(&observation.kind),
        label: observation.label.clone(),
        text: observation.text.clone(),
        score: observation.score,
        region: observation.region.map(region_report),
        track_id: observation.track_id.clone(),
        attributes: observation.attributes.clone(),
    }
}

fn observation_kind_name(kind: &ObservationKind) -> String {
    match kind {
        ObservationKind::Text => "text".to_string(),
        ObservationKind::Face => "face".to_string(),
        ObservationKind::Object => "object".to_string(),
        ObservationKind::Scene => "scene".to_string(),
        ObservationKind::Custom(value) => value.clone(),
    }
}

fn region_report(region: BoundingBox) -> RegionReport {
    RegionReport {
        x: region.x,
        y: region.y,
        width: region.width,
        height: region.height,
    }
}

fn event_report(event: &AnalysisEvent) -> EventReport {
    EventReport {
        timestamp_seconds: event.timestamp.map(|timestamp| timestamp.seconds()),
        analyzer: event.analyzer.clone(),
        label: event.label.clone(),
        score: event.score,
    }
}

fn bucket_report(bucket: &DataBucket) -> DataBucketReport {
    DataBucketReport {
        bucket_index: bucket.bucket_index,
        records: bucket.records,
        estimated_bytes: bucket.estimated_bytes,
        streams: bucket
            .streams
            .iter()
            .map(|(stream_id, summary)| stream_bucket_report(stream_id, summary))
            .collect(),
    }
}

fn push_record(
    aggregator: &mut BucketAggregator,
    record: DataRecord<'_>,
    data_buckets: &mut Vec<DataBucketReport>,
) -> Result<()> {
    for bucket in aggregator.push(record)? {
        data_buckets.push(bucket_report(&bucket));
    }
    Ok(())
}

fn stream_bucket_report(stream_id: &str, summary: &StreamSummary) -> StreamBucketReport {
    StreamBucketReport {
        stream_id: stream_id.to_string(),
        records: summary.records,
        estimated_bytes: summary.estimated_bytes,
        payload_counts: summary
            .payload_counts
            .iter()
            .map(|(kind, count)| (data_stream_kind_name(*kind), *count))
            .collect(),
        video_frames: summary.video.frames,
        audio_frames: summary.audio.frames,
        text_segments: summary.text.segments,
    }
}

fn data_stream_kind_name(kind: DataStreamKind) -> String {
    match kind {
        DataStreamKind::Video => "video".to_string(),
        DataStreamKind::Audio => "audio".to_string(),
        DataStreamKind::Text => "text".to_string(),
        DataStreamKind::Number => "number".to_string(),
        DataStreamKind::Vector => "vector".to_string(),
        DataStreamKind::Custom => "custom".to_string(),
    }
}

fn video_report_with_observations(
    mut video_report: VideoReport,
    observations: Vec<ObservationReport>,
) -> VideoReport {
    video_report.observations = observations;
    video_report
}

fn default_scene_threshold() -> f32 {
    27.0
}

fn default_min_scene_len() -> u64 {
    15
}

fn default_visual_sample_every() -> u64 {
    30
}

fn write_json_report<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|err| DetectError::Source(format!("failed to encode report JSON: {err}")))?;
    fs::write(path, bytes)?;
    Ok(())
}

fn resolve_command(command: &Path) -> Option<PathBuf> {
    if command.components().count() > 1 {
        return command.exists().then(|| command.to_path_buf());
    }

    let name = command.to_str()?;
    env::var_os("PATH").and_then(|paths| {
        env::split_paths(&paths)
            .map(|dir| dir.join(name))
            .find(|path| path.is_file())
    })
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use video_analysis_core::TextAnalyzer;

    #[test]
    fn normalizes_legacy_whisper_without_custom_command_to_whisper_cpp() {
        let config = TranscriptionConfig {
            enabled: true,
            engine: TranscriptionEngine::Whisper,
            command: None,
            whisper_cpp: WhisperCppConfig::default(),
        }
        .normalized();

        assert_eq!(config.engine, TranscriptionEngine::WhisperCpp);
    }

    #[test]
    fn preserves_legacy_whisper_when_custom_command_is_present() {
        let config = TranscriptionConfig {
            enabled: true,
            engine: TranscriptionEngine::Whisper,
            command: Some(ExternalCommandConfig {
                command: PathBuf::from("/usr/local/bin/whisper"),
                args: vec!["--model".to_string(), "base.en".to_string()],
            }),
            whisper_cpp: WhisperCppConfig::default(),
        }
        .normalized();

        assert_eq!(config.engine, TranscriptionEngine::Whisper);
    }

    #[test]
    fn whisper_cpp_config_round_trips_through_serde() {
        let config = TranscriptionConfig {
            enabled: true,
            engine: TranscriptionEngine::WhisperCpp,
            command: None,
            whisper_cpp: WhisperCppConfig {
                model: WhisperCppModel::LargeV3Turbo,
                language: Some("de".to_string()),
                translate: true,
                threads: Some(6),
            },
        };

        let json = serde_json::to_string(&config).unwrap();
        let decoded: TranscriptionConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded, config);
    }

    #[test]
    fn audio_separation_config_validates_known_values() {
        let config = AudioSeparationConfig {
            enabled: true,
            command: None,
            model: Some("htdemucs".to_string()),
            two_stems: Some("vocals".to_string()),
            device: Some("cpu".to_string()),
            output_dir: Some(PathBuf::from("separated")),
        };
        config.validate().unwrap();
        assert!(AudioSeparationConfig {
            model: Some(" ".to_string()),
            ..config.clone()
        }
        .validate()
        .is_err());
    }

    #[test]
    fn parses_transcript_whisper_json_segments() {
        let parsed = text_analysis_transcription::parse_whisper_json(
            br#"{"text":"hello world","segments":[{"start":0.0,"end":1.5,"text":" hello"}]}"#,
        )
        .unwrap();

        assert_eq!(parsed.text.as_deref(), Some("hello world"));
        assert_eq!(parsed.segments.len(), 1);
        assert_eq!(parsed.segments[0].start_seconds, Some(0.0));
    }

    #[test]
    fn transcript_heuristics_identify_common_events() {
        let mut analyzer = TranscriptHeuristicAnalyzer;
        let segment = video_analysis_core::OwnedTextSegment::new(0, "Visit https://example.com?");

        let events = analyzer.process_segment(&segment.as_segment()).unwrap();

        assert_eq!(events.len(), 2);
        assert_eq!(events[0].label, "speech:question");
        assert_eq!(events[1].label, "speech:url");
    }

    #[test]
    fn youtube_video_request_runs_local_video_without_clap() {
        if !(video_analysis_ffmpeg::is_ffmpeg_available()
            && video_analysis_ffmpeg::is_ffprobe_available())
        {
            eprintln!("skipping use-case integration test because ffmpeg/ffprobe is unavailable");
            return;
        }

        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("two-scenes.mp4");
        video_analysis_ffmpeg::write_two_scene_test_video(&input).unwrap();

        let report = run_youtube_video(YoutubeVideoRequest {
            input: Some(input),
            work_dir: dir.path().join("work"),
            output: Some(dir.path().join("analysis.json")),
            skip_transcription: true,
            max_frames: Some(8),
            ..YoutubeVideoRequest::default()
        })
        .unwrap();

        assert_eq!(report.use_case, YOUTUBE_VIDEO_USE_CASE);
        assert_eq!(report.transcription.status, "skipped");
        assert!(report.video.frames_processed > 0);
        assert!(report.audio.separation.is_none());
    }
}
