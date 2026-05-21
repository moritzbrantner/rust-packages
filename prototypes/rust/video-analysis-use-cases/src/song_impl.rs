//! Internal module support for song impl.

use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use audio_analysis_recognition::FingerprintRecord;
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use text_transcripts::WhisperCppProgressEvent;
use video_analysis_core::{DetectError, Result};

use crate::workflow_support::{
    display_path, download_youtube_video, download_youtube_video_to_dir,
    extract_music_analysis_wav, resolve_command, transcribe_media, validate_local_file,
    validate_youtube_url, write_json_report,
};
use crate::{
    CapabilityReport, DataBucketReport, ExternalCommandConfig, SourceSpec, TranscriptionConfig,
    TranscriptionReport,
};

const DEFAULT_IDENTIFICATION_MIN_SCORE: f32 = 0.82;
const DEFAULT_IDENTIFICATION_AMBIGUOUS_MARGIN: f32 = 0.03;
const DEFAULT_IDENTIFICATION_MAX_CANDIDATES: usize = 5;
const DEFAULT_WINDOW_SECONDS: f64 = 30.0;
const DEFAULT_HOP_SECONDS: f64 = 15.0;
const DEFAULT_MIN_BPM: f32 = 60.0;
const DEFAULT_MAX_BPM: f32 = 200.0;
const FINGERPRINT_CHUNK_SECONDS: u64 = 120;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for song analysis run request.
pub struct SongAnalysisRunRequest {
    /// The source value.
    pub source: SourceSpec,
    /// The work dir value.
    pub work_dir: Option<PathBuf>,
    #[serde(default)]
    /// The catalog value.
    pub catalog: SongCatalogConfig,
    #[serde(default)]
    /// The identification value.
    pub identification: SongIdentificationConfig,
    #[serde(default)]
    /// The transcription value.
    pub transcription: TranscriptionConfig,
    #[serde(default)]
    /// The music value.
    pub music: MusicAnalysisConfig,
}

impl SongAnalysisRunRequest {
    /// Validates this value.
    pub fn validate(&self) -> Result<()> {
        match &self.source {
            SourceSpec::YoutubeUrl { url } => validate_youtube_url(url),
            SourceSpec::LocalFile { path } => validate_local_file(path),
        }?;
        self.identification.validate()?;
        self.music.validate()?;
        Ok(())
    }

    /// Returns resolved catalog path.
    pub fn resolved_catalog_path(&self, data_dir: &Path) -> PathBuf {
        self.catalog.resolved_path(data_dir)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
/// Data type for song catalog config.
pub struct SongCatalogConfig {
    /// The db path value.
    pub db_path: Option<PathBuf>,
}

impl SongCatalogConfig {
    /// Returns resolved path.
    pub fn resolved_path(&self, data_dir: &Path) -> PathBuf {
        self.db_path
            .clone()
            .unwrap_or_else(|| data_dir.join("song-catalog").join("catalog.sqlite3"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for song identification config.
pub struct SongIdentificationConfig {
    #[serde(default = "default_true")]
    /// The enabled value.
    pub enabled: bool,
    #[serde(default = "default_identification_min_score")]
    /// The min score value.
    pub min_score: f32,
    #[serde(default = "default_identification_ambiguous_margin")]
    /// The ambiguous margin value.
    pub ambiguous_margin: f32,
    #[serde(default = "default_identification_max_candidates")]
    /// The max candidates value.
    pub max_candidates: usize,
}

impl Default for SongIdentificationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_score: DEFAULT_IDENTIFICATION_MIN_SCORE,
            ambiguous_margin: DEFAULT_IDENTIFICATION_AMBIGUOUS_MARGIN,
            max_candidates: DEFAULT_IDENTIFICATION_MAX_CANDIDATES,
        }
    }
}

impl SongIdentificationConfig {
    /// Validates this value.
    pub fn validate(&self) -> Result<()> {
        if !self.min_score.is_finite() || !(0.0..=1.0).contains(&self.min_score) {
            return Err(DetectError::InvalidArgument(
                "identification.min_score must be between 0 and 1".to_string(),
            ));
        }
        if !self.ambiguous_margin.is_finite() || self.ambiguous_margin < 0.0 {
            return Err(DetectError::InvalidArgument(
                "identification.ambiguous_margin must be non-negative".to_string(),
            ));
        }
        if self.max_candidates == 0 {
            return Err(DetectError::InvalidArgument(
                "identification.max_candidates must be positive".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for music analysis config.
pub struct MusicAnalysisConfig {
    #[serde(default = "default_true")]
    /// The enabled value.
    pub enabled: bool,
    /// The command value.
    pub command: Option<ExternalCommandConfig>,
    #[serde(default = "default_window_seconds")]
    /// The window seconds value.
    pub window_seconds: f64,
    #[serde(default = "default_hop_seconds")]
    /// The hop seconds value.
    pub hop_seconds: f64,
    #[serde(default = "default_min_bpm")]
    /// The min BPM value.
    pub min_bpm: f32,
    #[serde(default = "default_max_bpm")]
    /// The max BPM value.
    pub max_bpm: f32,
}

impl Default for MusicAnalysisConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            command: None,
            window_seconds: DEFAULT_WINDOW_SECONDS,
            hop_seconds: DEFAULT_HOP_SECONDS,
            min_bpm: DEFAULT_MIN_BPM,
            max_bpm: DEFAULT_MAX_BPM,
        }
    }
}

impl MusicAnalysisConfig {
    /// Validates this value.
    pub fn validate(&self) -> Result<()> {
        if !self.window_seconds.is_finite() || self.window_seconds <= 0.0 {
            return Err(DetectError::InvalidArgument(
                "music.window_seconds must be positive".to_string(),
            ));
        }
        if !self.hop_seconds.is_finite() || self.hop_seconds <= 0.0 {
            return Err(DetectError::InvalidArgument(
                "music.hop_seconds must be positive".to_string(),
            ));
        }
        if !self.min_bpm.is_finite() || !self.max_bpm.is_finite() || self.min_bpm >= self.max_bpm {
            return Err(DetectError::InvalidArgument(
                "music.min_bpm must be lower than music.max_bpm".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for song catalog import request.
pub struct SongCatalogImportRequest {
    /// The catalog db path value.
    pub catalog_db_path: Option<PathBuf>,
    /// The items value.
    pub items: Vec<SongCatalogImportItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for song catalog import item.
pub struct SongCatalogImportItem {
    /// The source value.
    pub source: SourceSpec,
    /// Metadata associated with this value.
    pub metadata: SongCatalogMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for song catalog metadata.
pub struct SongCatalogMetadata {
    /// The title value.
    pub title: String,
    /// The artist value.
    pub artist: Option<String>,
    /// The album value.
    pub album: Option<String>,
    /// The isrc value.
    pub isrc: Option<String>,
    /// The source URI value.
    pub source_uri: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for song catalog import response.
pub struct SongCatalogImportResponse {
    /// The catalog path value.
    pub catalog_path: String,
    /// The imported value.
    pub imported: u64,
    /// The failed value.
    pub failed: u64,
    /// The results value.
    pub results: Vec<SongCatalogImportResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for song catalog import result.
pub struct SongCatalogImportResult {
    /// The index value.
    pub index: u64,
    /// The track identifier value.
    pub track_id: Option<String>,
    /// The title value.
    pub title: String,
    /// The status value.
    pub status: String,
    /// The message value.
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for song analysis report.
pub struct SongAnalysisReport {
    #[serde(alias = "use_case")]
    /// The workflow value.
    pub workflow: String,
    /// The source value.
    pub source: SongSourceReport,
    /// The assets value.
    pub assets: SongAssetReport,
    /// The capabilities value.
    pub capabilities: CapabilityReport,
    /// The identification value.
    pub identification: SongIdentificationReport,
    /// The lyrics value.
    pub lyrics: TranscriptionReport,
    /// The music value.
    pub music: MusicAnalysisReport,
    /// The data buckets value.
    pub data_buckets: Vec<DataBucketReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for song source report.
pub struct SongSourceReport {
    /// URL associated with this value.
    pub url: Option<String>,
    /// The local media value.
    pub local_media: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for song asset report.
pub struct SongAssetReport {
    /// The work dir value.
    pub work_dir: String,
    /// The report path value.
    pub report_path: String,
    /// The transcription wav value.
    pub transcription_wav: Option<String>,
    /// The music analysis wav value.
    pub music_analysis_wav: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for song identification report.
pub struct SongIdentificationReport {
    /// The status value.
    pub status: String,
    /// The query value.
    pub query: Option<SongFingerprintQueryReport>,
    /// The catalog path value.
    pub catalog_path: String,
    /// The top match value.
    pub top_match: Option<SongMatchCandidateReport>,
    /// The candidates value.
    pub candidates: Vec<SongMatchCandidateReport>,
    /// The message value.
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for song fingerprint query report.
pub struct SongFingerprintQueryReport {
    /// Duration in seconds.
    pub duration_seconds: f64,
    /// The fingerprint value.
    pub fingerprint: String,
    /// The chunk count value.
    pub chunk_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for song match candidate report.
pub struct SongMatchCandidateReport {
    /// The track identifier value.
    pub track_id: String,
    /// The title value.
    pub title: String,
    /// The artist value.
    pub artist: Option<String>,
    /// The album value.
    pub album: Option<String>,
    /// The isrc value.
    pub isrc: Option<String>,
    /// The source URI value.
    pub source_uri: Option<String>,
    /// Duration in seconds.
    pub duration_seconds: f64,
    /// Score assigned to this value.
    pub score: f32,
    /// The matched segment value.
    pub matched_segment: Option<SongMatchedSegmentReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for song matched segment report.
pub struct SongMatchedSegmentReport {
    /// The query start seconds value.
    pub query_start_seconds: Option<f64>,
    /// The catalog start seconds value.
    pub catalog_start_seconds: Option<f64>,
    /// Duration in seconds.
    pub duration_seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for music analysis report.
pub struct MusicAnalysisReport {
    /// The status value.
    pub status: String,
    /// The global value.
    pub global: Option<MusicFeatureReport>,
    /// The segments value.
    pub segments: Vec<MusicSegmentReport>,
    /// The message value.
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for music feature report.
pub struct MusicFeatureReport {
    /// The BPM value.
    pub bpm: Option<f32>,
    /// The BPM confidence value.
    pub bpm_confidence: Option<f32>,
    /// The key value.
    pub key: Option<String>,
    /// The mode value.
    pub mode: Option<String>,
    /// The key confidence value.
    pub key_confidence: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for music segment report.
pub struct MusicSegmentReport {
    /// The index value.
    pub index: u64,
    /// The start seconds value.
    pub start_seconds: f64,
    /// The end seconds value.
    pub end_seconds: f64,
    /// The BPM value.
    pub bpm: Option<f32>,
    /// The BPM confidence value.
    pub bpm_confidence: Option<f32>,
    /// The key value.
    pub key: Option<String>,
    /// The mode value.
    pub mode: Option<String>,
    /// The key confidence value.
    pub key_confidence: Option<f32>,
}

#[derive(Debug, Clone)]
struct CatalogTrack {
    track_id: String,
    title: String,
    artist: Option<String>,
    album: Option<String>,
    isrc: Option<String>,
    source_uri: Option<String>,
    duration_seconds: f64,
    raw: Vec<i64>,
}

#[derive(Debug, Clone)]
struct CatalogChunk {
    track_id: String,
    start_seconds: f64,
    duration_seconds: f64,
    raw: Vec<i64>,
}

/// Runs song analysis workflow.
pub fn run_song_analysis_workflow(
    request: SongAnalysisRunRequest,
    data_dir: PathBuf,
    work_dir: PathBuf,
    report_path: PathBuf,
    progress: &mut dyn FnMut(WhisperCppProgressEvent),
) -> Result<SongAnalysisReport> {
    request.validate()?;
    std::fs::create_dir_all(&work_dir)?;

    let mut completed = Vec::new();
    let mut skipped = Vec::new();
    let (source_url, media_path) = resolve_song_source(&request.source, &work_dir)?;
    if source_url.is_some() {
        completed.push("youtube_download".to_string());
    }

    let catalog_path = request.resolved_catalog_path(&data_dir);
    let identification = if request.identification.enabled {
        match identify_song(&media_path, &catalog_path, &request.identification) {
            Ok(report) => {
                if matches!(report.status.as_str(), "matched" | "ambiguous" | "no_match") {
                    completed.push("song_identification".to_string());
                }
                report
            }
            Err(error) => {
                skipped.push(format!("song identification: {error}"));
                SongIdentificationReport {
                    status: "failed".to_string(),
                    query: None,
                    catalog_path: display_path(&catalog_path),
                    top_match: None,
                    candidates: Vec::new(),
                    message: Some(error.to_string()),
                }
            }
        }
    } else {
        skipped.push("song identification: disabled".to_string());
        SongIdentificationReport {
            status: "skipped".to_string(),
            query: None,
            catalog_path: display_path(&catalog_path),
            top_match: None,
            candidates: Vec::new(),
            message: Some("disabled".to_string()),
        }
    };

    let (lyrics, transcription_wav) =
        transcribe_media(&request.transcription, &media_path, &work_dir, progress);
    if lyrics.status == "completed" {
        completed.push("lyrics_transcription".to_string());
    } else {
        skipped.push(format!(
            "lyrics transcription: {}",
            lyrics.message.as_deref().unwrap_or("not available")
        ));
    }

    let (music, music_analysis_wav) = analyze_music(&media_path, &work_dir, &request.music);
    if music.status == "completed" {
        completed.push("music_analysis".to_string());
    } else {
        skipped.push(format!(
            "music analysis: {}",
            music.message.as_deref().unwrap_or("not available")
        ));
    }

    let report = SongAnalysisReport {
        workflow: "song_analysis".to_string(),
        source: SongSourceReport {
            url: source_url,
            local_media: display_path(&media_path),
        },
        assets: SongAssetReport {
            work_dir: display_path(&work_dir),
            report_path: display_path(&report_path),
            transcription_wav: transcription_wav.as_ref().map(|path| display_path(path)),
            music_analysis_wav: music_analysis_wav.as_ref().map(|path| display_path(path)),
        },
        capabilities: CapabilityReport { completed, skipped },
        identification,
        lyrics,
        music,
        data_buckets: Vec::<DataBucketReport>::new(),
    };

    write_json_report(&report_path, &report)?;
    Ok(report)
}

/// Returns import song catalog items.
pub fn import_song_catalog_items(
    request: SongCatalogImportRequest,
    data_dir: PathBuf,
) -> Result<SongCatalogImportResponse> {
    if request.items.is_empty() {
        return Err(DetectError::InvalidArgument(
            "at least one catalog item is required".to_string(),
        ));
    }
    let catalog_path = request
        .catalog_db_path
        .clone()
        .unwrap_or_else(|| data_dir.join("song-catalog").join("catalog.sqlite3"));
    let work_dir = data_dir.join("song-catalog").join("imports");
    std::fs::create_dir_all(&work_dir)?;
    let conn = open_catalog(&catalog_path)?;

    let mut results = Vec::new();
    for (index, item) in request.items.into_iter().enumerate() {
        let title = item.metadata.title.trim().to_string();
        if title.is_empty() {
            results.push(SongCatalogImportResult {
                index: index as u64,
                track_id: None,
                title,
                status: "failed".to_string(),
                message: Some("title is required".to_string()),
            });
            continue;
        }
        let item_dir = work_dir.join(format!("item-{index}"));
        let result = import_catalog_item(&conn, &item_dir, item);
        results.push(match result {
            Ok(track_id) => SongCatalogImportResult {
                index: index as u64,
                track_id: Some(track_id),
                title,
                status: "imported".to_string(),
                message: None,
            },
            Err(error) => SongCatalogImportResult {
                index: index as u64,
                track_id: None,
                title,
                status: "failed".to_string(),
                message: Some(error.to_string()),
            },
        });
    }

    let imported = results
        .iter()
        .filter(|result| result.status == "imported")
        .count() as u64;
    let failed = results
        .iter()
        .filter(|result| result.status == "failed")
        .count() as u64;
    Ok(SongCatalogImportResponse {
        catalog_path: display_path(&catalog_path),
        imported,
        failed,
        results,
    })
}

fn import_catalog_item(
    conn: &Connection,
    item_dir: &Path,
    item: SongCatalogImportItem,
) -> Result<String> {
    let (_, media_path) = resolve_song_source_to_dir(&item.source, item_dir)?;
    let full = run_fpcalc(&media_path, false)?;
    let full = full.into_iter().next().ok_or_else(|| {
        DetectError::Source("fpcalc completed but no fingerprint was returned".to_string())
    })?;
    let chunks = run_fpcalc(&media_path, true)?;
    let track_id = format!(
        "track-{}",
        Utc::now()
            .timestamp_nanos_opt()
            .unwrap_or_else(|| Utc::now().timestamp_micros())
    );
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO tracks (
            track_id, title, artist, album, isrc, source_uri, duration_seconds,
            fingerprint, raw_fingerprint_json, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            track_id,
            item.metadata.title,
            item.metadata.artist,
            item.metadata.album,
            item.metadata.isrc,
            item.metadata.source_uri,
            full.duration,
            full.compressed
                .clone()
                .unwrap_or_else(|| fingerprint_preview(&full.raw)),
            full.raw_json,
            now,
            now
        ],
    )
    .map_err(sql_error)?;
    for (index, chunk) in chunks.iter().enumerate() {
        conn.execute(
            "INSERT INTO fingerprint_chunks (
                track_id, chunk_index, start_seconds, duration_seconds, raw_fingerprint_json
            ) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                track_id,
                index as i64,
                chunk
                    .timestamp
                    .unwrap_or(index as f64 * FINGERPRINT_CHUNK_SECONDS as f64),
                chunk.duration,
                chunk.raw_json
            ],
        )
        .map_err(sql_error)?;
    }
    Ok(track_id)
}

fn identify_song(
    media_path: &Path,
    catalog_path: &Path,
    config: &SongIdentificationConfig,
) -> Result<SongIdentificationReport> {
    let query_full = run_fpcalc(media_path, false)?;
    let query_full = query_full.into_iter().next().ok_or_else(|| {
        DetectError::Source("fpcalc completed but no fingerprint was returned".to_string())
    })?;
    let query_chunks = run_fpcalc(media_path, true)?;
    let conn = open_catalog(catalog_path)?;
    let candidates = match_candidates(&conn, &query_full, &query_chunks, config)?;
    let top_match = candidates.first().cloned();
    let status = match (&candidates.first(), candidates.get(1)) {
        (Some(top), Some(second))
            if top.score >= config.min_score
                && (top.score - second.score).abs() < config.ambiguous_margin =>
        {
            "ambiguous"
        }
        (Some(top), _) if top.score >= config.min_score => "matched",
        _ => "no_match",
    }
    .to_string();

    Ok(SongIdentificationReport {
        status,
        query: Some(SongFingerprintQueryReport {
            duration_seconds: query_full.duration,
            fingerprint: query_full
                .compressed
                .clone()
                .unwrap_or_else(|| fingerprint_preview(&query_full.raw)),
            chunk_count: query_chunks.len() as u64,
        }),
        catalog_path: display_path(catalog_path),
        top_match,
        candidates,
        message: None,
    })
}

fn match_candidates(
    conn: &Connection,
    query_full: &FingerprintRecord,
    query_chunks: &[FingerprintRecord],
    config: &SongIdentificationConfig,
) -> Result<Vec<SongMatchCandidateReport>> {
    let tracks = load_catalog_tracks(conn)?;
    if tracks.is_empty() {
        return Ok(Vec::new());
    }
    let chunks = load_catalog_chunks(conn)?;
    let use_chunk_matching = query_full.duration < 60.0;
    let mut candidates = Vec::new();

    for track in tracks {
        let mut score = 0.0_f32;
        let mut matched_segment = None;
        if !use_chunk_matching
            && audio_analysis_recognition::duration_prefilter(
                query_full.duration,
                track.duration_seconds,
            )
        {
            let fingerprint_score =
                audio_analysis_recognition::shifted_similarity(&query_full.raw, &track.raw);
            let duration_factor = (1.0
                - ((query_full.duration - track.duration_seconds).abs()
                    / query_full.duration.max(track.duration_seconds)) as f32)
                .clamp(0.0, 1.0);
            score = fingerprint_score * duration_factor;
            matched_segment = Some(SongMatchedSegmentReport {
                query_start_seconds: Some(0.0),
                catalog_start_seconds: Some(0.0),
                duration_seconds: query_full.duration.min(track.duration_seconds),
            });
        }

        let track_chunks = chunks
            .iter()
            .filter(|chunk| chunk.track_id == track.track_id);
        for catalog_chunk in track_chunks {
            let queries: Box<dyn Iterator<Item = &FingerprintRecord>> = if query_chunks.is_empty() {
                Box::new(std::iter::once(query_full))
            } else {
                Box::new(query_chunks.iter())
            };
            for query_chunk in queries {
                let chunk_score = audio_analysis_recognition::shifted_similarity(
                    &query_chunk.raw,
                    &catalog_chunk.raw,
                );
                if chunk_score > score {
                    score = chunk_score;
                    matched_segment = Some(SongMatchedSegmentReport {
                        query_start_seconds: query_chunk.timestamp,
                        catalog_start_seconds: Some(catalog_chunk.start_seconds),
                        duration_seconds: query_chunk.duration.min(catalog_chunk.duration_seconds),
                    });
                }
            }
        }

        if score > 0.0 {
            candidates.push(SongMatchCandidateReport {
                track_id: track.track_id,
                title: track.title,
                artist: track.artist,
                album: track.album,
                isrc: track.isrc,
                source_uri: track.source_uri,
                duration_seconds: track.duration_seconds,
                score,
                matched_segment,
            });
        }
    }

    candidates.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| left.title.cmp(&right.title))
    });
    candidates.truncate(config.max_candidates);
    Ok(candidates)
}

fn open_catalog(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(path).map_err(sql_error)?;
    migrate_catalog(&conn)?;
    Ok(conn)
}

fn migrate_catalog(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        PRAGMA foreign_keys = ON;
        CREATE TABLE IF NOT EXISTS tracks(
          track_id TEXT PRIMARY KEY,
          title TEXT NOT NULL,
          artist TEXT,
          album TEXT,
          isrc TEXT,
          source_uri TEXT,
          duration_seconds REAL NOT NULL,
          fingerprint TEXT NOT NULL,
          raw_fingerprint_json TEXT NOT NULL,
          created_at TEXT NOT NULL,
          updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS fingerprint_chunks(
          track_id TEXT NOT NULL,
          chunk_index INTEGER NOT NULL,
          start_seconds REAL NOT NULL,
          duration_seconds REAL NOT NULL,
          raw_fingerprint_json TEXT NOT NULL,
          PRIMARY KEY(track_id, chunk_index),
          FOREIGN KEY(track_id) REFERENCES tracks(track_id) ON DELETE CASCADE
        );
        ",
    )
    .map_err(sql_error)?;
    Ok(())
}

fn load_catalog_tracks(conn: &Connection) -> Result<Vec<CatalogTrack>> {
    let mut stmt = conn
        .prepare(
            "SELECT track_id, title, artist, album, isrc, source_uri, duration_seconds, raw_fingerprint_json FROM tracks",
        )
        .map_err(sql_error)?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, f64>(6)?,
                row.get::<_, String>(7)?,
            ))
        })
        .map_err(sql_error)?;
    let mut tracks = Vec::new();
    for row in rows {
        let (track_id, title, artist, album, isrc, source_uri, duration_seconds, raw_json) =
            row.map_err(sql_error)?;
        let raw = audio_analysis_recognition::parse_fpcalc_record(&raw_json)?.raw;
        tracks.push(CatalogTrack {
            track_id,
            title,
            artist,
            album,
            isrc,
            source_uri,
            duration_seconds,
            raw,
        });
    }
    Ok(tracks)
}

fn load_catalog_chunks(conn: &Connection) -> Result<Vec<CatalogChunk>> {
    let exists: Option<String> = conn
        .query_row(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'fingerprint_chunks'",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(sql_error)?;
    if exists.is_none() {
        return Ok(Vec::new());
    }
    let mut stmt = conn
        .prepare(
            "SELECT track_id, start_seconds, duration_seconds, raw_fingerprint_json FROM fingerprint_chunks",
        )
        .map_err(sql_error)?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, f64>(1)?,
                row.get::<_, f64>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(sql_error)?;
    let mut chunks = Vec::new();
    for row in rows {
        let (track_id, start_seconds, duration_seconds, raw_json) = row.map_err(sql_error)?;
        let raw = audio_analysis_recognition::parse_fpcalc_record(&raw_json)?.raw;
        chunks.push(CatalogChunk {
            track_id,
            start_seconds,
            duration_seconds,
            raw,
        });
    }
    Ok(chunks)
}

fn analyze_music(
    media_path: &Path,
    work_dir: &Path,
    config: &MusicAnalysisConfig,
) -> (MusicAnalysisReport, Option<PathBuf>) {
    if !config.enabled {
        return (
            MusicAnalysisReport {
                status: "skipped".to_string(),
                global: None,
                segments: Vec::new(),
                message: Some("disabled".to_string()),
            },
            None,
        );
    }
    let Some(command) = config.command.clone() else {
        return (
            MusicAnalysisReport {
                status: "skipped".to_string(),
                global: None,
                segments: Vec::new(),
                message: Some("music analyzer command is not configured".to_string()),
            },
            None,
        );
    };
    if resolve_command(&command.command).is_none() {
        return (
            MusicAnalysisReport {
                status: "skipped".to_string(),
                global: None,
                segments: Vec::new(),
                message: Some(format!("{} is not available", command.command.display())),
            },
            None,
        );
    }

    match extract_music_analysis_wav(media_path, work_dir)
        .and_then(|audio_path| run_music_command(&command, &audio_path, config))
    {
        Ok((report, audio_path)) => (report, Some(audio_path)),
        Err(error) => (
            MusicAnalysisReport {
                status: "failed".to_string(),
                global: None,
                segments: Vec::new(),
                message: Some(error.to_string()),
            },
            None,
        ),
    }
}

fn run_music_command(
    command: &ExternalCommandConfig,
    audio_path: &Path,
    config: &MusicAnalysisConfig,
) -> Result<(MusicAnalysisReport, PathBuf)> {
    let output = Command::new(&command.command)
        .args(&command.args)
        .arg("--input")
        .arg(audio_path)
        .arg("--window-seconds")
        .arg(config.window_seconds.to_string())
        .arg("--hop-seconds")
        .arg(config.hop_seconds.to_string())
        .arg("--min-bpm")
        .arg(config.min_bpm.to_string())
        .arg("--max-bpm")
        .arg(config.max_bpm.to_string())
        .stdin(Stdio::null())
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let message = if stderr.is_empty() { stdout } else { stderr };
        return Err(DetectError::Source(format!(
            "music analyzer command `{}` failed: {}",
            command.command.display(),
            message
        )));
    }
    let parsed: AnalyzerMusicReport = serde_json::from_slice(&output.stdout)
        .map_err(|err| DetectError::Source(err.to_string()))?;
    Ok((parsed.into_report(), audio_path.to_path_buf()))
}

#[derive(Debug, Deserialize)]
struct AnalyzerMusicReport {
    status: String,
    global: Option<MusicFeatureReport>,
    #[serde(default)]
    segments: Vec<MusicSegmentReport>,
    message: Option<String>,
}

impl AnalyzerMusicReport {
    fn into_report(self) -> MusicAnalysisReport {
        MusicAnalysisReport {
            status: self.status,
            global: self.global,
            segments: self.segments,
            message: self.message,
        }
    }
}

fn resolve_song_source(source: &SourceSpec, work_dir: &Path) -> Result<(Option<String>, PathBuf)> {
    resolve_song_source_to_dir(source, work_dir)
}

fn resolve_song_source_to_dir(
    source: &SourceSpec,
    work_dir: &Path,
) -> Result<(Option<String>, PathBuf)> {
    match source {
        SourceSpec::LocalFile { path } => {
            validate_local_file(path)?;
            Ok((None, path.clone()))
        }
        SourceSpec::YoutubeUrl { url } => {
            validate_youtube_url(url)?;
            let path = if work_dir.ends_with("imports") {
                download_youtube_video_to_dir(url, work_dir, "song")
            } else {
                download_youtube_video(url, work_dir)
            }?;
            Ok((Some(url.clone()), path))
        }
    }
}

fn run_fpcalc(media_path: &Path, chunked: bool) -> Result<Vec<FingerprintRecord>> {
    let command = PathBuf::from("fpcalc");
    if resolve_command(&command).is_none() {
        return Err(DetectError::Source(
            "fpcalc is required for song fingerprinting".to_string(),
        ));
    }
    audio_analysis_recognition::run_fpcalc(
        command,
        media_path,
        chunked.then_some(FINGERPRINT_CHUNK_SECONDS),
    )
}

fn fingerprint_preview(raw: &[i64]) -> String {
    raw.iter()
        .take(32)
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

fn sql_error(error: rusqlite::Error) -> DetectError {
    DetectError::Source(error.to_string())
}

fn default_true() -> bool {
    true
}

fn default_identification_min_score() -> f32 {
    DEFAULT_IDENTIFICATION_MIN_SCORE
}

fn default_identification_ambiguous_margin() -> f32 {
    DEFAULT_IDENTIFICATION_AMBIGUOUS_MARGIN
}

fn default_identification_max_candidates() -> usize {
    DEFAULT_IDENTIFICATION_MAX_CANDIDATES
}

fn default_window_seconds() -> f64 {
    DEFAULT_WINDOW_SECONDS
}

fn default_hop_seconds() -> f64 {
    DEFAULT_HOP_SECONDS
}

fn default_min_bpm() -> f32 {
    DEFAULT_MIN_BPM
}

fn default_max_bpm() -> f32 {
    DEFAULT_MAX_BPM
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_song_analysis_request() {
        let request = SongAnalysisRunRequest {
            source: SourceSpec::LocalFile {
                path: PathBuf::from("song.mp3"),
            },
            work_dir: Some(PathBuf::from("work")),
            catalog: SongCatalogConfig {
                db_path: Some(PathBuf::from("catalog.sqlite3")),
            },
            identification: SongIdentificationConfig::default(),
            transcription: TranscriptionConfig::default(),
            music: MusicAnalysisConfig::default(),
        };
        let json = serde_json::to_string(&request).unwrap();
        let parsed: SongAnalysisRunRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, request);
    }

    #[test]
    fn catalog_migration_creates_tables() {
        let conn = Connection::open_in_memory().unwrap();
        migrate_catalog(&conn).unwrap();
        let count: u64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name IN ('tracks', 'fingerprint_chunks')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn empty_catalog_returns_no_candidates() {
        let conn = Connection::open_in_memory().unwrap();
        migrate_catalog(&conn).unwrap();
        let query = FingerprintRecord {
            timestamp: None,
            duration: 120.0,
            compressed: None,
            raw: vec![1, 2, 3],
            raw_json: r#"{"duration":120.0,"fingerprint":[1,2,3]}"#.to_string(),
        };
        let candidates =
            match_candidates(&conn, &query, &[], &SongIdentificationConfig::default()).unwrap();
        assert!(candidates.is_empty());
    }
}
