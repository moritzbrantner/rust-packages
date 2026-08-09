//! Internal module support for subtitle impl.

use std::path::{Path, PathBuf};

use audio_analysis_transcription::WhisperCppProgressEvent;
use serde::{Deserialize, Serialize};
use text_transcripts::TranscriptSegment;
use video_analysis_core::{DetectError, Result};

use crate::workflow_support::{
    display_path, download_youtube_video, transcribe_media, validate_local_file,
    validate_youtube_url, write_json_report,
};
use crate::{
    CapabilityReport, SourceReport, SourceSpec, TranscriptSegmentReport, TranscriptionConfig,
    TranscriptionReport,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for subtitle generation run request.
pub struct SubtitleGenerationRunRequest {
    /// The source value.
    pub source: SourceSpec,
    /// The work dir value.
    pub work_dir: Option<PathBuf>,
    #[serde(default)]
    /// The transcription value.
    pub transcription: TranscriptionConfig,
    #[serde(default = "default_overwrite")]
    /// The overwrite value.
    pub overwrite: bool,
}

impl SubtitleGenerationRunRequest {
    /// Validates this value.
    pub fn validate(&self) -> Result<()> {
        match &self.source {
            SourceSpec::YoutubeUrl { url } => validate_youtube_url(url),
            SourceSpec::LocalFile { path } => validate_local_file(path),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for subtitle generation report.
pub struct SubtitleGenerationReport {
    #[serde(alias = "use_case")]
    /// The workflow value.
    pub workflow: String,
    /// The source value.
    pub source: SourceReport,
    /// The assets value.
    pub assets: SubtitleAssetReport,
    /// The capabilities value.
    pub capabilities: CapabilityReport,
    /// The transcription value.
    pub transcription: TranscriptionReport,
    /// The subtitle value.
    pub subtitle: SubtitleReport,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for subtitle asset report.
pub struct SubtitleAssetReport {
    /// The work dir value.
    pub work_dir: String,
    /// The report path value.
    pub report_path: String,
    /// The audio wav value.
    pub audio_wav: Option<String>,
    /// The subtitle path value.
    pub subtitle_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for subtitle report.
pub struct SubtitleReport {
    /// The format value.
    pub format: String,
    /// Filesystem path for this value.
    pub path: String,
    /// The segments value.
    pub segments: u64,
    /// The message value.
    pub message: Option<String>,
}

/// Runs subtitle generation workflow.
pub fn run_subtitle_generation_workflow(
    request: SubtitleGenerationRunRequest,
    work_dir: PathBuf,
    report_path: PathBuf,
    progress: &mut dyn FnMut(WhisperCppProgressEvent),
) -> Result<SubtitleGenerationReport> {
    request.validate()?;
    std::fs::create_dir_all(&work_dir)?;

    let mut completed = Vec::new();
    let mut skipped = Vec::new();

    let (source_url, video_path) = match &request.source {
        SourceSpec::LocalFile { path } => (None, path.clone()),
        SourceSpec::YoutubeUrl { url } => {
            let path = download_youtube_video(url, &work_dir)?;
            completed.push("youtube_download".to_string());
            (Some(url.clone()), path)
        }
    };

    let subtitle_path = subtitle_path_for_video(&video_path);
    if subtitle_path.exists() && !request.overwrite {
        return Err(DetectError::Source(format!(
            "subtitle file already exists: {}",
            subtitle_path.display()
        )));
    }

    let (transcription, audio_wav) =
        transcribe_media(&request.transcription, &video_path, &work_dir, progress);
    if transcription.status != "completed" {
        return Err(DetectError::Source(format!(
            "subtitle generation requires completed transcription: {}",
            transcription.message.as_deref().unwrap_or("not available")
        )));
    }
    completed.push("transcription".to_string());

    write_srt_file(&subtitle_path, &transcription.segments)?;
    completed.push("subtitle_srt".to_string());
    if transcription.segments.is_empty() {
        skipped.push("subtitle content: transcript contained no segments".to_string());
    }

    let report = SubtitleGenerationReport {
        workflow: "subtitle_generation".to_string(),
        source: SourceReport {
            url: source_url,
            local_video: display_path(&video_path),
        },
        assets: SubtitleAssetReport {
            work_dir: display_path(&work_dir),
            report_path: display_path(&report_path),
            audio_wav: audio_wav.as_ref().map(|path| display_path(path)),
            subtitle_path: display_path(&subtitle_path),
        },
        capabilities: CapabilityReport { completed, skipped },
        subtitle: SubtitleReport {
            format: "srt".to_string(),
            path: display_path(&subtitle_path),
            segments: transcription.segments.len() as u64,
            message: Some("written next to source video".to_string()),
        },
        transcription,
    };

    write_json_report(&report_path, &report)?;
    Ok(report)
}

fn default_overwrite() -> bool {
    true
}

fn subtitle_path_for_video(video_path: &Path) -> PathBuf {
    video_path.with_extension("srt")
}

fn write_srt_file(path: &Path, segments: &[TranscriptSegmentReport]) -> Result<()> {
    let segments = transcription_segments(segments);
    text_transcripts::write_srt(path, &segments).map_err(|err| DetectError::Source(err.to_string()))
}

#[cfg(test)]
fn srt_text(segments: &[TranscriptSegmentReport]) -> String {
    text_transcripts::format_srt(&transcription_segments(segments))
}

fn transcription_segments(segments: &[TranscriptSegmentReport]) -> Vec<TranscriptSegment> {
    segments
        .iter()
        .map(|segment| TranscriptSegment {
            index: segment.index,
            start_seconds: segment.start_seconds,
            end_seconds: segment.end_seconds,
            text: segment.text.clone(),
            language: None,
            speaker: None,
            confidence: None,
            is_final: true,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subtitle_path_reuses_video_folder_and_stem() {
        assert_eq!(
            subtitle_path_for_video(Path::new("/tmp/source/video.final.mp4")),
            PathBuf::from("/tmp/source/video.final.srt")
        );
    }

    #[test]
    fn srt_text_formats_segments() {
        let text = srt_text(&[
            TranscriptSegmentReport {
                index: 0,
                start_seconds: Some(1.25),
                end_seconds: Some(3.5),
                text: "Hello".to_string(),
            },
            TranscriptSegmentReport {
                index: 1,
                start_seconds: Some(63.0),
                end_seconds: Some(65.125),
                text: "World".to_string(),
            },
        ]);

        assert_eq!(
            text,
            "1\n00:00:01,250 --> 00:00:03,500\nHello\n\n2\n00:01:03,000 --> 00:01:05,125\nWorld\n\n"
        );
    }
}
