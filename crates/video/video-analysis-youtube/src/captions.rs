use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::source::YoutubeVideoItem;
use crate::yt_dlp::{YtDlpClient, YtDlpConfig};
use crate::YoutubeError;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// Caption download request.
pub struct CaptionDownloadRequest {
    pub enabled: bool,
    pub include_auto_captions: bool,
    pub languages: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
/// YouTube caption track kind.
pub enum YoutubeCaptionKind {
    Manual,
    Auto,
}

impl YoutubeCaptionKind {
    /// Stable string form.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Manual => "caption_manual",
            Self::Auto => "caption_auto",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
/// Downloaded and parsed caption track.
pub struct YoutubeCaptionTrack {
    pub kind: YoutubeCaptionKind,
    pub language: Option<String>,
    pub source_path: PathBuf,
    pub transcript: text_transcripts::TranscriptionContract,
}

/// Downloads caption files and parses them into transcript contracts.
pub async fn download_youtube_captions(
    item: &YoutubeVideoItem,
    output_dir: &Path,
    request: CaptionDownloadRequest,
    config: &YtDlpConfig,
) -> Result<Vec<YoutubeCaptionTrack>, YoutubeError> {
    if !request.enabled || item.local_video_path.is_some() {
        return Ok(Vec::new());
    }
    tokio::fs::create_dir_all(output_dir).await?;
    let languages = if request.languages.is_empty() {
        "en".to_string()
    } else {
        request.languages.join(",")
    };
    let template = output_dir.join(format!("{}-subs.%(id)s.%(ext)s", item.item_id));
    let client = YtDlpClient::new(config.clone());

    let _ = client
        .download_captions(&item.source_url, &template, &languages, false)
        .await;
    if request.include_auto_captions {
        let _ = client
            .download_captions(&item.source_url, &template, &languages, true)
            .await;
    }

    parse_youtube_caption_files(output_dir).await
}

/// Alias for callers that want the parsed transcript result explicitly.
pub async fn download_and_parse_youtube_captions(
    item: &YoutubeVideoItem,
    output_dir: &Path,
    request: CaptionDownloadRequest,
    config: &YtDlpConfig,
) -> Result<Vec<text_transcripts::TranscriptionContract>, YoutubeError> {
    Ok(download_youtube_captions(item, output_dir, request, config)
        .await?
        .into_iter()
        .map(|track| track.transcript)
        .collect())
}

/// Parses existing SRT/WebVTT caption files in a directory.
pub async fn parse_youtube_caption_files(
    dir: &Path,
) -> Result<Vec<YoutubeCaptionTrack>, YoutubeError> {
    let mut tracks = Vec::new();
    let mut entries = tokio::fs::read_dir(dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        let Some(ext) = path.extension().and_then(|value| value.to_str()) else {
            continue;
        };
        if ext != "vtt" && ext != "srt" {
            continue;
        }
        let text = tokio::fs::read_to_string(&path).await?;
        let mut parsed = if ext == "vtt" {
            text_transcripts::parse_webvtt(&text)
        } else {
            text_transcripts::parse_srt(&text)
        }?;
        normalize_transcription(&mut parsed);
        if parsed.segments.is_empty() && parsed.text.as_deref().unwrap_or_default().is_empty() {
            continue;
        }
        let kind = if path
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|name| name.contains(".auto.") || name.contains("auto"))
        {
            YoutubeCaptionKind::Auto
        } else {
            YoutubeCaptionKind::Manual
        };
        let language = parsed
            .language
            .clone()
            .or_else(|| infer_language_from_path(&path));
        tracks.push(YoutubeCaptionTrack {
            kind,
            language,
            source_path: path,
            transcript: text_transcripts::TranscriptionContract::from(parsed),
        });
    }
    tracks.sort_by(|left, right| {
        left.kind
            .cmp(&right.kind)
            .then_with(|| left.language.cmp(&right.language))
            .then_with(|| left.source_path.cmp(&right.source_path))
    });
    Ok(tracks)
}

fn normalize_transcription(transcript: &mut text_transcripts::TranscriptionResult) {
    let options = text_transcripts::SubtitleNormalizationOptions::default();
    for segment in &mut transcript.segments {
        segment.text = text_transcripts::normalize_subtitle_text(&segment.text, options.clone());
    }
    transcript
        .segments
        .retain(|segment| !segment.text.is_empty());
    transcript.text = transcript
        .text
        .as_deref()
        .map(|text| text_transcripts::normalize_subtitle_text(text, options))
        .filter(|text| !text.is_empty())
        .or_else(|| {
            (!transcript.segments.is_empty()).then(|| {
                transcript
                    .segments
                    .iter()
                    .map(|segment| segment.text.as_str())
                    .collect::<Vec<_>>()
                    .join(" ")
            })
        });
}

fn infer_language_from_path(path: &Path) -> Option<String> {
    let file_name = path.file_name()?.to_str()?;
    let parts = file_name.split('.').collect::<Vec<_>>();
    parts
        .iter()
        .rev()
        .skip(1)
        .find(|part| part.len() == 2 || part.len() == 5)
        .map(|value| (*value).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn parses_downloaded_caption_files_through_text_transcripts() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("video.en.vtt");
        tokio::fs::write(
            &path,
            "WEBVTT\n\n00:00:01.000 --> 00:00:02.000\n<v Speaker>Hello &amp; <c>world</c>\n",
        )
        .await
        .unwrap();

        let tracks = parse_youtube_caption_files(dir.path()).await.unwrap();

        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].language.as_deref(), Some("en"));
        assert_eq!(tracks[0].transcript.segments[0].text, "Hello & world");
    }
}
