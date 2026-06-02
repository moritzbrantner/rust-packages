#![doc = include_str!("../README.md")]

pub mod surface;
use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use audio_analysis_core::OwnedAudioWaveformBatch;
use serde::Deserialize;
use text_core::{tokenize, tokenize_words, TextProcessingOptions, TokenKind};

mod whisper_cpp;

use thiserror::Error;
use video_analysis_core::{AnalysisEvent, OwnedTextSegment, TextAnalyzer, TextSegment, Timestamp};
use video_analysis_ingest::{
    MediaSourceInfo, SourceMode, TextFormat as IngestTextFormat, TextSegmentSource, TextStreamInfo,
};
pub mod contracts;
pub use contracts::{TranscriptSegmentContract, TranscriptWordContract, TranscriptionContract};
/// Re-exports the text transcript native whisper.cpp API.
pub use whisper_cpp::{
    transcription_catalog as whisper_cpp_catalog, whisper_cpp_system_info,
    ModelStore as WhisperCppModelStore, WhisperCppCatalog, WhisperCppConfig, WhisperCppError,
    WhisperCppModel, WhisperCppModelStatus, WhisperCppPhase, WhisperCppProgressEvent,
    WhisperCppSegment, WhisperCppTranscriber as NativeWhisperCppTranscriber,
};

#[derive(Debug, Error)]
/// Variants describing transcription error.
pub enum TranscriptionError {
    #[error("I/O error: {0}")]
    /// The I/O variant.
    Io(#[from] std::io::Error),
    #[error("invalid transcript JSON: {0}")]
    /// The JSON variant.
    Json(#[from] serde_json::Error),
    #[error("invalid transcript: {0}")]
    /// The invalid transcript variant.
    InvalidTranscript(String),
    #[error("transcriber command `{0}` failed")]
    /// The command failed variant.
    CommandFailed(String),
    #[error("{0}")]
    /// The detect variant.
    Detect(#[from] video_analysis_core::DetectError),
    #[error("{0}")]
    /// The whisper cpp variant.
    WhisperCpp(#[from] WhisperCppError),
}

/// Type alias for result.
pub type Result<T> = std::result::Result<T, TranscriptionError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Variants describing transcript format.
pub enum TranscriptFormat {
    /// The plain variant.
    Plain,
    /// The lines variant.
    Lines,
    /// The whisper JSON variant.
    WhisperJson,
    /// The srt variant.
    Srt,
    /// The web vtt variant.
    WebVtt,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for transcript segment.
pub struct TranscriptSegment {
    /// The index value.
    pub index: u64,
    /// The start seconds value.
    pub start_seconds: Option<f64>,
    /// The end seconds value.
    pub end_seconds: Option<f64>,
    /// Text content for this value.
    pub text: String,
    /// Language tag for this value.
    pub language: Option<String>,
    /// The speaker value.
    pub speaker: Option<String>,
    /// Confidence score for this value.
    pub confidence: Option<f32>,
    /// The is final value.
    pub is_final: bool,
}

impl TranscriptSegment {
    /// Returns metadata.
    pub fn metadata(&self) -> BTreeMap<String, String> {
        let mut metadata = BTreeMap::new();
        insert_optional(&mut metadata, "language", self.language.as_deref());
        insert_optional(&mut metadata, "speaker", self.speaker.as_deref());
        insert_optional_number(&mut metadata, "start_seconds", self.start_seconds);
        insert_optional_number(&mut metadata, "end_seconds", self.end_seconds);
        insert_optional_display(&mut metadata, "confidence", self.confidence);
        metadata
    }

    /// Returns metadata with source.
    pub fn metadata_with_source(&self, source: impl Into<String>) -> BTreeMap<String, String> {
        let mut metadata = self.metadata();
        let source = source.into();
        if !source.is_empty() {
            metadata.insert("source".to_string(), source);
        }
        metadata
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for transcription result.
pub struct TranscriptionResult {
    /// Text content for this value.
    pub text: Option<String>,
    /// Language tag for this value.
    pub language: Option<String>,
    /// The segments value.
    pub segments: Vec<TranscriptSegment>,
    /// The source value.
    pub source: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
/// Optional word-level transcript timing.
pub struct TranscriptWord {
    /// Word text.
    pub text: String,
    /// Optional start time in seconds.
    pub start_seconds: Option<f64>,
    /// Optional end time in seconds.
    pub end_seconds: Option<f64>,
    /// Optional confidence.
    pub confidence: Option<f32>,
}

/// Trait for transcriber implementations.
pub trait Transcriber {
    /// Returns transcribe.
    fn transcribe(&mut self, input: &Path) -> Result<TranscriptionResult>;
}

#[derive(Debug, Default, Clone)]
/// Transcript-specific deterministic analyzer.
pub struct TranscriptHeuristicAnalyzer;

impl TextAnalyzer for TranscriptHeuristicAnalyzer {
    fn name(&self) -> &str {
        "transcript_heuristics"
    }

    fn process_segment(
        &mut self,
        segment: &TextSegment<'_>,
    ) -> video_analysis_core::Result<Vec<AnalysisEvent>> {
        let mut events = Vec::new();
        let text = segment.text.trim();
        if text.ends_with(['?', '؟', '？']) {
            events.push(event_at(self.name(), "speech:question", segment.timestamp));
        }
        if has_token_kind(text, TokenKind::Url) {
            events.push(event_at(self.name(), "speech:url", segment.timestamp));
        }
        if has_token_kind(text, TokenKind::Number) {
            events.push(event_at(self.name(), "speech:number", segment.timestamp));
        }
        if tokenize_words(text).len() >= 30 {
            events.push(event_at(
                self.name(),
                "speech:long_segment",
                segment.timestamp,
            ));
        }
        Ok(events)
    }
}

#[derive(Debug, Clone)]
/// Data type for command transcriber.
pub struct CommandTranscriber {
    command: PathBuf,
    args: Vec<String>,
    format: TranscriptFormat,
}

impl CommandTranscriber {
    /// Creates a new value.
    pub fn new(command: impl Into<PathBuf>, format: TranscriptFormat) -> Self {
        Self {
            command: command.into(),
            args: Vec::new(),
            format,
        }
    }

    /// Returns args.
    pub fn args(mut self, args: impl IntoIterator<Item = String>) -> Self {
        self.args.extend(args);
        self
    }
}

impl Transcriber for CommandTranscriber {
    fn transcribe(&mut self, input: &Path) -> Result<TranscriptionResult> {
        let output = Command::new(&self.command)
            .args(&self.args)
            .arg(input)
            .stdin(Stdio::null())
            .output()?;
        if !output.status.success() {
            return Err(TranscriptionError::CommandFailed(
                self.command.display().to_string(),
            ));
        }
        parse_transcript_bytes(&output.stdout, self.format)
    }
}

#[derive(Debug, Clone)]
/// Data type for whisper cli transcriber.
pub struct WhisperCliTranscriber {
    command: PathBuf,
    args: Vec<String>,
    output_dir: Option<PathBuf>,
}

impl WhisperCliTranscriber {
    /// Creates a new value.
    pub fn new(command: impl Into<PathBuf>) -> Self {
        Self {
            command: command.into(),
            args: Vec::new(),
            output_dir: None,
        }
    }

    /// Returns args.
    pub fn args(mut self, args: impl IntoIterator<Item = String>) -> Self {
        self.args.extend(args);
        self
    }

    /// Returns output dir.
    pub fn output_dir(mut self, output_dir: impl Into<PathBuf>) -> Self {
        self.output_dir = Some(output_dir.into());
        self
    }
}

impl Transcriber for WhisperCliTranscriber {
    fn transcribe(&mut self, input: &Path) -> Result<TranscriptionResult> {
        let output_dir = self.output_dir.clone().unwrap_or_else(|| {
            input
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join("transcript")
        });
        fs::create_dir_all(&output_dir)?;

        let status = Command::new(&self.command)
            .arg(input)
            .args(&self.args)
            .arg("--output_format")
            .arg("json")
            .arg("--output_dir")
            .arg(&output_dir)
            .stdin(Stdio::null())
            .status()?;
        if !status.success() {
            return Err(TranscriptionError::CommandFailed(
                self.command.display().to_string(),
            ));
        }

        let transcript_path = find_transcript_json(&output_dir).ok_or_else(|| {
            TranscriptionError::InvalidTranscript(
                "transcriber completed but no JSON transcript was found".to_string(),
            )
        })?;
        let bytes = fs::read(&transcript_path)?;
        let mut result = parse_whisper_json(&bytes)?;
        result.source = Some(transcript_path.to_string_lossy().into_owned());
        Ok(result)
    }
}

/// Data type for whisper cpp transcriber.
pub struct WhisperCppTranscriber {
    inner: NativeWhisperCppTranscriber,
}

impl WhisperCppTranscriber {
    /// Creates a new value.
    pub fn new(config: WhisperCppConfig) -> Self {
        Self {
            inner: NativeWhisperCppTranscriber::new(config),
        }
    }

    /// Returns this value with model store.
    pub fn with_model_store(mut self, store: WhisperCppModelStore) -> Self {
        self.inner = self.inner.with_model_store(store);
        self
    }

    /// Returns on progress.
    pub fn on_progress<F>(mut self, callback: F) -> Self
    where
        F: FnMut(WhisperCppProgressEvent) + 'static,
    {
        self.inner = self.inner.on_progress(callback);
        self
    }

    /// Returns transcribe with progress.
    pub fn transcribe_with_progress(
        &mut self,
        input: &Path,
        progress: &mut dyn FnMut(WhisperCppProgressEvent),
    ) -> Result<TranscriptionResult> {
        let transcript = self.inner.transcribe_file_with_progress(input, progress)?;
        Ok(whisper_cpp_result_to_transcription_result(transcript))
    }
}

impl Transcriber for WhisperCppTranscriber {
    fn transcribe(&mut self, input: &Path) -> Result<TranscriptionResult> {
        let transcript = self.inner.transcribe_file(input)?;
        Ok(whisper_cpp_result_to_transcription_result(transcript))
    }
}

fn whisper_cpp_result_to_transcription_result(
    transcript: whisper_cpp::WhisperCppTranscription,
) -> TranscriptionResult {
    TranscriptionResult {
        text: transcript.text,
        language: transcript.language.clone(),
        segments: transcript
            .segments
            .into_iter()
            .map(|segment| TranscriptSegment {
                index: segment.index,
                start_seconds: segment.start_seconds,
                end_seconds: segment.end_seconds,
                text: segment.text,
                language: transcript.language.clone(),
                speaker: None,
                confidence: segment.confidence,
                is_final: true,
            })
            .collect(),
        source: transcript.source,
    }
}

/// Data type for transcript segment source.
pub struct TranscriptSegmentSource {
    source_info: MediaSourceInfo,
    segments: Vec<TranscriptSegment>,
    next_index: usize,
}

impl TranscriptSegmentSource {
    /// Returns recorded.
    pub fn recorded(input: impl Into<String>, segments: Vec<TranscriptSegment>) -> Self {
        Self::new(SourceMode::Recorded, input, segments)
    }

    /// Returns live.
    pub fn live(input: impl Into<String>, segments: Vec<TranscriptSegment>) -> Self {
        Self::new(SourceMode::Live, input, segments)
    }

    fn new(mode: SourceMode, input: impl Into<String>, segments: Vec<TranscriptSegment>) -> Self {
        let language = segments.iter().find_map(|segment| segment.language.clone());
        let source_info = MediaSourceInfo {
            input: input.into(),
            mode,
            video: None,
            audio: Vec::new(),
            text: vec![TextStreamInfo {
                format: IngestTextFormat::Transcript,
                language,
            }],
        };
        Self {
            source_info,
            segments,
            next_index: 0,
        }
    }
}

impl TextSegmentSource for TranscriptSegmentSource {
    fn source_info(&self) -> &MediaSourceInfo {
        &self.source_info
    }

    fn next_text_segment(&mut self) -> video_analysis_core::Result<Option<OwnedTextSegment>> {
        let Some(segment) = self.segments.get(self.next_index) else {
            return Ok(None);
        };
        self.next_index += 1;
        Ok(Some(segment_to_owned_text_segment(segment)))
    }
}

#[derive(Debug, Deserialize)]
struct WhisperOutput {
    text: Option<String>,
    language: Option<String>,
    #[serde(default)]
    segments: Vec<WhisperSegment>,
}

#[derive(Debug, Deserialize)]
struct WhisperSegment {
    id: Option<u64>,
    start: Option<f64>,
    end: Option<f64>,
    text: String,
    #[serde(default)]
    avg_logprob: Option<f32>,
    #[serde(default)]
    no_speech_prob: Option<f32>,
}

/// Parses parse whisper JSON.
pub fn parse_whisper_json(bytes: &[u8]) -> Result<TranscriptionResult> {
    let parsed: WhisperOutput = serde_json::from_slice(bytes)?;
    let segments = parsed
        .segments
        .into_iter()
        .enumerate()
        .map(|(index, segment)| TranscriptSegment {
            index: segment.id.unwrap_or(index as u64),
            start_seconds: segment.start,
            end_seconds: segment.end,
            text: segment.text.trim().to_string(),
            language: parsed.language.clone(),
            speaker: None,
            confidence: whisper_confidence(segment.avg_logprob, segment.no_speech_prob),
            is_final: true,
        })
        .collect::<Vec<_>>();
    Ok(TranscriptionResult {
        text: parsed.text.map(|text| text.trim().to_string()),
        language: parsed.language,
        segments,
        source: None,
    })
}

/// Parses parse srt.
pub fn parse_srt(text: &str) -> Result<TranscriptionResult> {
    parse_subtitle_blocks(text, TranscriptFormat::Srt)
}

/// Parses parse webvtt.
pub fn parse_webvtt(text: &str) -> Result<TranscriptionResult> {
    parse_subtitle_blocks(text, TranscriptFormat::WebVtt)
}

/// Parses parse plain lines.
pub fn parse_plain_lines(text: &str) -> TranscriptionResult {
    let segments = text
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let line = line.trim();
            (!line.is_empty()).then(|| TranscriptSegment {
                index: index as u64,
                start_seconds: None,
                end_seconds: None,
                text: line.to_string(),
                language: None,
                speaker: None,
                confidence: None,
                is_final: true,
            })
        })
        .collect::<Vec<_>>();
    TranscriptionResult {
        text: Some(
            segments
                .iter()
                .map(|segment| segment.text.as_str())
                .collect::<Vec<_>>()
                .join("\n"),
        ),
        language: None,
        segments,
        source: None,
    }
}

/// Returns format srt.
pub fn format_srt(segments: &[TranscriptSegment]) -> String {
    let mut output = String::new();
    for (index, segment) in segments.iter().enumerate() {
        let start = segment.start_seconds.unwrap_or(0.0);
        let end = segment
            .end_seconds
            .unwrap_or_else(|| (start + 2.0).max(start));
        output.push_str(&(index + 1).to_string());
        output.push('\n');
        output.push_str(&format_srt_timestamp(start));
        output.push_str(" --> ");
        output.push_str(&format_srt_timestamp(end.max(start)));
        output.push('\n');
        output.push_str(segment.text.trim());
        output.push_str("\n\n");
    }
    output
}

/// Returns format webvtt.
pub fn format_webvtt(segments: &[TranscriptSegment]) -> String {
    let mut output = String::from("WEBVTT\n\n");
    for (index, segment) in segments.iter().enumerate() {
        if index > 0 {
            output.push('\n');
        }
        let start = segment.start_seconds.unwrap_or(0.0);
        let end = segment
            .end_seconds
            .unwrap_or_else(|| (start + 2.0).max(start));
        output.push_str(&format_webvtt_timestamp(start));
        output.push_str(" --> ");
        output.push_str(&format_webvtt_timestamp(end.max(start)));
        output.push('\n');
        output.push_str(segment.text.trim());
        output.push('\n');
    }
    output
}

/// Writes srt.
pub fn write_srt(path: impl AsRef<Path>, segments: &[TranscriptSegment]) -> Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, format_srt(segments))?;
    Ok(())
}

/// Returns transcribe waveform batch.
pub fn transcribe_waveform_batch<T: Transcriber>(
    transcriber: &mut T,
    batch: &OwnedAudioWaveformBatch,
    wav_path: &Path,
) -> Result<TranscriptionResult> {
    write_waveform_batch_as_wav(wav_path, batch)?;
    transcriber.transcribe(wav_path)
}

fn write_waveform_batch_as_wav(
    path: impl AsRef<Path>,
    batch: &OwnedAudioWaveformBatch,
) -> Result<()> {
    let view = batch.as_view()?;
    if view.batch_size() != 1 {
        return Err(video_analysis_core::DetectError::InvalidArgument(
            "waveform WAV export requires a batch size of 1".to_string(),
        )
        .into());
    }
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let spec = hound::WavSpec {
        channels: view.channel_count() as u16,
        sample_rate: view.sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(path, spec).map_err(|err| {
        video_analysis_core::DetectError::Source(format!(
            "failed to create WAV `{}`: {err}",
            path.display()
        ))
    })?;
    for time_index in 0..view.time_steps() {
        for channel_index in 0..view.channel_count() {
            let sample = view.waveform(0, channel_index)?[time_index];
            writer.write_sample(sample).map_err(|err| {
                video_analysis_core::DetectError::Source(format!(
                    "failed to write WAV sample `{}`: {err}",
                    path.display()
                ))
            })?;
        }
    }
    writer.finalize().map_err(|err| {
        video_analysis_core::DetectError::Source(format!(
            "failed to finalize WAV `{}`: {err}",
            path.display()
        ))
    })?;
    Ok(())
}

/// Returns format srt timestamp.
pub fn format_srt_timestamp(seconds: f64) -> String {
    let total_millis = (seconds.max(0.0) * 1_000.0).round() as u64;
    let millis = total_millis % 1_000;
    let total_seconds = total_millis / 1_000;
    let secs = total_seconds % 60;
    let total_minutes = total_seconds / 60;
    let minutes = total_minutes % 60;
    let hours = total_minutes / 60;
    format!("{hours:02}:{minutes:02}:{secs:02},{millis:03}")
}

fn format_webvtt_timestamp(seconds: f64) -> String {
    format_srt_timestamp(seconds).replace(',', ".")
}

/// Returns segment to owned text segment.
pub fn segment_to_owned_text_segment(segment: &TranscriptSegment) -> OwnedTextSegment {
    text_core::TextSegmentContract::from(TranscriptSegmentContract::from(segment))
        .to_owned_text_segment()
}

fn parse_transcript_bytes(bytes: &[u8], format: TranscriptFormat) -> Result<TranscriptionResult> {
    match format {
        TranscriptFormat::Plain | TranscriptFormat::Lines => {
            Ok(parse_plain_lines(&String::from_utf8_lossy(bytes)))
        }
        TranscriptFormat::WhisperJson => parse_whisper_json(bytes),
        TranscriptFormat::Srt => parse_srt(&String::from_utf8_lossy(bytes)),
        TranscriptFormat::WebVtt => parse_webvtt(&String::from_utf8_lossy(bytes)),
    }
}

fn parse_subtitle_blocks(text: &str, format: TranscriptFormat) -> Result<TranscriptionResult> {
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut segments = Vec::new();

    for block in normalized.split("\n\n") {
        let lines = block
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>();
        if lines.is_empty()
            || (format == TranscriptFormat::WebVtt && lines[0].starts_with("WEBVTT"))
        {
            continue;
        }

        let time_line_index = lines
            .iter()
            .position(|line| line.contains("-->"))
            .ok_or_else(|| {
                TranscriptionError::InvalidTranscript(
                    "subtitle block missing timestamp".to_string(),
                )
            })?;
        let (start_seconds, end_seconds) = parse_timestamp_range(lines[time_line_index])?;
        let text_lines = &lines[time_line_index + 1..];
        if text_lines.is_empty() {
            continue;
        }
        let index = if time_line_index > 0 {
            lines[0].parse::<u64>().unwrap_or(segments.len() as u64)
        } else {
            segments.len() as u64
        };
        segments.push(TranscriptSegment {
            index,
            start_seconds: Some(start_seconds),
            end_seconds: Some(end_seconds),
            text: text_lines.join(" "),
            language: None,
            speaker: None,
            confidence: None,
            is_final: true,
        });
    }

    Ok(TranscriptionResult {
        text: Some(
            segments
                .iter()
                .map(|segment| segment.text.as_str())
                .collect::<Vec<_>>()
                .join("\n"),
        ),
        language: None,
        segments,
        source: None,
    })
}

fn parse_timestamp_range(line: &str) -> Result<(f64, f64)> {
    let Some((start, end_with_settings)) = line.split_once("-->") else {
        return Err(TranscriptionError::InvalidTranscript(
            "timestamp range missing -->".to_string(),
        ));
    };
    let end = end_with_settings
        .split_whitespace()
        .next()
        .unwrap_or(end_with_settings);
    Ok((parse_timestamp(start.trim())?, parse_timestamp(end.trim())?))
}

fn parse_timestamp(value: &str) -> Result<f64> {
    let value = value.replace(',', ".");
    let pieces = value.split(':').collect::<Vec<_>>();
    let seconds = match pieces.as_slice() {
        [minutes, seconds] => {
            Some(parse_timestamp_component(minutes)? * 60.0 + parse_timestamp_component(seconds)?)
        }
        [hours, minutes, seconds] => Some(
            parse_timestamp_component(hours)? * 3600.0
                + parse_timestamp_component(minutes)? * 60.0
                + parse_timestamp_component(seconds)?,
        ),
        _ => None,
    };
    seconds.ok_or_else(|| {
        TranscriptionError::InvalidTranscript(format!("invalid timestamp `{value}`"))
    })
}

fn parse_timestamp_component(value: &str) -> Result<f64> {
    value.parse::<f64>().map_err(|_| {
        TranscriptionError::InvalidTranscript(format!("invalid timestamp component `{value}`"))
    })
}

fn whisper_confidence(avg_logprob: Option<f32>, no_speech_prob: Option<f32>) -> Option<f32> {
    avg_logprob.or(no_speech_prob.map(|probability| 1.0 - probability))
}

fn insert_optional(metadata: &mut BTreeMap<String, String>, key: &str, value: Option<&str>) {
    if let Some(value) = value {
        metadata.insert(key.to_string(), value.to_string());
    }
}

fn insert_optional_number(metadata: &mut BTreeMap<String, String>, key: &str, value: Option<f64>) {
    insert_optional_display(metadata, key, value);
}

fn insert_optional_display<T: fmt::Display>(
    metadata: &mut BTreeMap<String, String>,
    key: &str,
    value: Option<T>,
) {
    if let Some(value) = value {
        metadata.insert(key.to_string(), value.to_string());
    }
}

fn has_token_kind(text: &str, kind: TokenKind) -> bool {
    tokenize(text, &TextProcessingOptions::default())
        .into_iter()
        .any(|token| token.kind == kind)
}

fn event_at(analyzer: &str, label: &str, timestamp: Option<Timestamp>) -> AnalysisEvent {
    let event = AnalysisEvent::new(analyzer, label);
    if let Some(timestamp) = timestamp {
        event.at_timestamp(timestamp)
    } else {
        event
    }
}

fn find_transcript_json(output_dir: &Path) -> Option<PathBuf> {
    let mut candidates = fs::read_dir(output_dir)
        .ok()?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("json"))
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        let left_modified = fs::metadata(left)
            .and_then(|metadata| metadata.modified())
            .ok();
        let right_modified = fs::metadata(right)
            .and_then(|metadata| metadata.modified())
            .ok();
        left_modified
            .cmp(&right_modified)
            .then_with(|| left.cmp(right))
    });
    candidates.pop()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use video_analysis_core::{AudioBuffer, OwnedAudioFrame, Timebase, Timestamp};
    use video_analysis_ingest::TextSegmentSource;

    #[test]
    fn parses_whisper_json() {
        let parsed = parse_whisper_json(
            br#"{"text":"hello world","language":"en","segments":[{"id":7,"start":0.0,"end":1.5,"text":" hello"}]}"#,
        )
        .unwrap();

        assert_eq!(parsed.text.as_deref(), Some("hello world"));
        assert_eq!(parsed.language.as_deref(), Some("en"));
        assert_eq!(parsed.segments[0].index, 7);
        assert_eq!(parsed.segments[0].start_seconds, Some(0.0));
    }

    #[test]
    fn parses_srt() {
        let parsed = parse_srt("1\n00:00:01,000 --> 00:00:02,500\nHello\n\n").unwrap();
        assert_eq!(parsed.segments.len(), 1);
        assert_eq!(parsed.segments[0].start_seconds, Some(1.0));
        assert_eq!(parsed.segments[0].end_seconds, Some(2.5));
    }

    #[test]
    fn formats_srt() {
        let text = format_srt(&[
            TranscriptSegment {
                index: 0,
                start_seconds: Some(1.25),
                end_seconds: Some(3.5),
                text: "Hello".to_string(),
                language: None,
                speaker: None,
                confidence: None,
                is_final: true,
            },
            TranscriptSegment {
                index: 1,
                start_seconds: Some(63.0),
                end_seconds: Some(65.125),
                text: "World".to_string(),
                language: None,
                speaker: None,
                confidence: None,
                is_final: true,
            },
        ]);

        assert_eq!(
            text,
            "1\n00:00:01,250 --> 00:00:03,500\nHello\n\n2\n00:01:03,000 --> 00:01:05,125\nWorld\n\n"
        );
    }

    #[test]
    fn formats_webvtt() {
        let text = format_webvtt(&[TranscriptSegment {
            index: 0,
            start_seconds: Some(1.0),
            end_seconds: Some(0.5),
            text: "Hello.".to_string(),
            language: None,
            speaker: None,
            confidence: None,
            is_final: true,
        }]);

        assert_eq!(text, "WEBVTT\n\n00:00:01.000 --> 00:00:01.000\nHello.\n");
    }

    #[test]
    fn parses_webvtt() {
        let parsed = parse_webvtt("WEBVTT\n\ncue\n00:00:03.000 --> 00:00:04.250\nHi\n").unwrap();
        assert_eq!(parsed.segments.len(), 1);
        assert_eq!(parsed.segments[0].text, "Hi");
        assert_eq!(parsed.segments[0].start_seconds, Some(3.0));
    }

    #[test]
    fn parses_plain_lines() {
        let parsed = parse_plain_lines("one\n\ntwo\n");
        assert_eq!(parsed.segments.len(), 2);
        assert_eq!(parsed.text.as_deref(), Some("one\ntwo"));
    }

    #[test]
    fn converts_segment_timestamp() {
        let segment = TranscriptSegment {
            index: 2,
            start_seconds: Some(1.25),
            end_seconds: Some(2.0),
            text: "hello".to_string(),
            language: Some("en".to_string()),
            speaker: None,
            confidence: None,
            is_final: true,
        };
        let owned = segment_to_owned_text_segment(&segment);
        assert_eq!(owned.segment_index, 2);
        assert_eq!(owned.timestamp.unwrap().seconds(), 1.25);
        assert_eq!(owned.language.as_deref(), Some("en"));
    }

    #[test]
    fn transcript_segment_metadata_preserves_optional_fields() {
        let segment = TranscriptSegment {
            index: 2,
            start_seconds: Some(1.25),
            end_seconds: Some(2.0),
            text: "hello".to_string(),
            language: Some("en".to_string()),
            speaker: Some("speaker-1".to_string()),
            confidence: Some(0.75),
            is_final: true,
        };

        let metadata = segment.metadata_with_source("fixture.srt");

        assert_eq!(metadata["language"], "en");
        assert_eq!(metadata["speaker"], "speaker-1");
        assert_eq!(metadata["start_seconds"], "1.25");
        assert_eq!(metadata["end_seconds"], "2");
        assert_eq!(metadata["confidence"], "0.75");
        assert_eq!(metadata["source"], "fixture.srt");
    }

    #[test]
    fn transcript_segment_source_iterates() {
        let mut source = TranscriptSegmentSource::recorded(
            "test",
            vec![TranscriptSegment {
                index: 0,
                start_seconds: None,
                end_seconds: None,
                text: "hello".to_string(),
                language: None,
                speaker: None,
                confidence: None,
                is_final: true,
            }],
        );
        assert_eq!(source.next_text_segment().unwrap().unwrap().text, "hello");
        assert!(source.next_text_segment().unwrap().is_none());
    }

    #[test]
    fn transcript_heuristic_analyzer_emits_speech_events() {
        let segment = TextSegment {
            segment_index: 0,
            timestamp: None,
            text: "Visit https://example.com at 3?",
            language: None,
            is_final: true,
        };
        let mut analyzer = TranscriptHeuristicAnalyzer;

        let labels = analyzer
            .process_segment(&segment)
            .unwrap()
            .into_iter()
            .map(|event| event.label)
            .collect::<Vec<_>>();

        assert!(labels.iter().any(|label| label == "speech:question"));
        assert!(labels.iter().any(|label| label == "speech:url"));
        assert!(labels.iter().any(|label| label == "speech:number"));
    }

    #[test]
    fn command_transcriber_reports_failure() {
        let mut transcriber = CommandTranscriber::new("false", TranscriptFormat::Plain);
        let err = transcriber.transcribe(Path::new("missing")).unwrap_err();
        assert!(matches!(err, TranscriptionError::CommandFailed(_)));
    }

    #[test]
    fn transcribes_waveform_batches_via_existing_command_transcriber() {
        let dir = tempdir().unwrap();
        let script_path = dir.path().join("transcriber.sh");
        fs::write(&script_path, "#!/bin/sh\nprintf 'hello from batch\\n'\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let mut permissions = fs::metadata(&script_path).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&script_path, permissions).unwrap();
        }

        let mut transcriber = CommandTranscriber::new(&script_path, TranscriptFormat::Plain);
        let frame = OwnedAudioFrame::new(
            Timestamp::new(0, Timebase::new(1, 16_000)),
            16_000,
            1,
            AudioBuffer::F32(vec![0.0, 0.25, -0.25, 0.5]),
        )
        .unwrap();
        let batch = OwnedAudioWaveformBatch::from_audio_frames(&[frame]).unwrap();
        let wav_path = dir.path().join("input.wav");

        let result = transcribe_waveform_batch(&mut transcriber, &batch, &wav_path).unwrap();
        assert_eq!(result.text.as_deref(), Some("hello from batch"));
        assert!(wav_path.is_file());
    }

    #[test]
    fn srt_webvtt_plain_round_trip() {
        let srt = "1\n00:00:00,000 --> 00:00:01,000\nHello world\n";
        let parsed = parse_srt(srt).unwrap();
        let formatted = format_srt(&parsed.segments);
        assert!(formatted.contains("Hello world"));

        let webvtt =
            parse_webvtt("WEBVTT\n\n00:00:00.000 --> 00:00:01.000\nHello world\n").unwrap();
        let plain = parse_plain_lines("Hello world\n");

        assert_eq!(parsed.segments[0].text, webvtt.segments[0].text);
        assert_eq!(plain.text.as_deref(), Some("Hello world"));
    }
}
