//! Internal module support for workflow support.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::Serialize;
use video_analysis_core::{DetectError, Result};

use crate::{ExternalCommandConfig, TranscriptSegmentReport, TranscriptionReport};

pub(crate) fn validate_local_file(path: &Path) -> Result<()> {
    if path.as_os_str().is_empty() {
        return Err(DetectError::InvalidArgument(
            "local file path is required".to_string(),
        ));
    }
    Ok(())
}

pub(crate) fn display_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

pub(crate) fn write_json_report<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|err| DetectError::Source(format!("failed to encode report JSON: {err}")))?;
    fs::write(path, bytes)?;
    Ok(())
}

/// Runs the legacy OpenAI Whisper CLI with its exact positional-argument
/// contract and imports the newest JSON output through `text-transcripts`.
pub(crate) fn run_whisper_cli_command(
    config: &ExternalCommandConfig,
    audio_path: &Path,
    output_dir: &Path,
) -> Result<(TranscriptionReport, PathBuf)> {
    fs::create_dir_all(output_dir)?;

    let status = Command::new(&config.command)
        .arg(audio_path)
        .args(&config.args)
        .arg("--output_format")
        .arg("json")
        .arg("--output_dir")
        .arg(output_dir)
        .stdin(Stdio::null())
        .status()?;
    if !status.success() {
        return Err(DetectError::Source(format!(
            "whisper transcriber command `{}` failed",
            config.command.display()
        )));
    }

    let transcript_path = find_newest_transcript_json(output_dir).ok_or_else(|| {
        DetectError::Source("transcriber completed but no JSON transcript was found".to_string())
    })?;
    let parsed = text_transcripts::parse_whisper_json(&fs::read(&transcript_path)?)
        .map_err(|error| DetectError::Source(error.to_string()))?;
    let segments = parsed
        .segments
        .into_iter()
        .map(|segment| TranscriptSegmentReport {
            index: segment.index,
            start_seconds: segment.start_seconds,
            end_seconds: segment.end_seconds,
            text: segment.text.trim().to_string(),
        })
        .collect();
    Ok((
        TranscriptionReport {
            status: "completed".to_string(),
            text: parsed.text.map(|text| text.trim().to_string()),
            segments,
            message: Some(display_path(&transcript_path)),
        },
        audio_path.to_path_buf(),
    ))
}

fn find_newest_transcript_json(output_dir: &Path) -> Option<PathBuf> {
    fs::read_dir(output_dir)
        .ok()?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .max_by(|left, right| {
            let left_modified = fs::metadata(left)
                .and_then(|metadata| metadata.modified())
                .ok();
            let right_modified = fs::metadata(right)
                .and_then(|metadata| metadata.modified())
                .ok();
            left_modified
                .cmp(&right_modified)
                .then_with(|| left.cmp(right))
        })
}
