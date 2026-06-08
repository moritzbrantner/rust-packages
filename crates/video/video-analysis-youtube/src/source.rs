use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use text_retrieval::SearchDocument;

use crate::yt_dlp::{YtDlpClient, YtDlpConfig};
use crate::YoutubeError;

#[derive(Debug, Clone)]
/// A discovered or single YouTube video item.
pub struct YoutubeVideoItem {
    pub item_id: String,
    pub youtube_id: Option<String>,
    pub title: Option<String>,
    pub source_url: String,
    pub duration_seconds: Option<f64>,
    pub upload_date: Option<String>,
    pub metadata: YoutubeVideoMetadata,
    pub local_video_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
/// Metadata extracted from yt-dlp for a YouTube video.
pub struct YoutubeVideoMetadata {
    pub youtube_id: Option<String>,
    pub title: Option<String>,
    pub webpage_url: Option<String>,
    pub original_url: Option<String>,
    pub duration_seconds: Option<f64>,
    pub upload_date: Option<String>,
    pub description: Option<String>,
    pub channel: Option<String>,
    pub channel_id: Option<String>,
    pub channel_url: Option<String>,
    pub uploader: Option<String>,
    pub uploader_id: Option<String>,
    pub uploader_url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub view_count: Option<i64>,
    pub like_count: Option<i64>,
    pub comment_count: Option<i64>,
    pub categories: Vec<String>,
    pub tags: Vec<String>,
    pub yt_dlp_args_redacted: Vec<String>,

    #[serde(default)]
    pub duration_string: Option<String>,
    #[serde(default)]
    pub timestamp: Option<i64>,
    #[serde(default)]
    pub release_timestamp: Option<i64>,
    #[serde(default)]
    pub live_status: Option<String>,
    #[serde(default)]
    pub availability: Option<String>,
    #[serde(default)]
    pub age_limit: Option<i64>,
    #[serde(default)]
    pub yt_dlp_version: Option<String>,
    #[serde(default)]
    pub metadata_downloaded_at: Option<String>,
    #[serde(default)]
    pub metadata_source: Option<String>,
    #[serde(default)]
    pub media_probe: Option<MediaProbeMetadata>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
/// Metadata from probing a downloaded media file.
pub struct MediaProbeMetadata {
    pub input: String,
    pub path: Option<PathBuf>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub frame_rate: Option<String>,
    pub duration_seconds: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq)]
/// Filter for discovered YouTube items.
pub struct YoutubeItemFilter {
    pub title_contains: Option<String>,
    pub title_excludes: Vec<String>,
    pub duration_min: Option<f64>,
    pub duration_max: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct YtDlpCollectionJson {
    entries: Option<Vec<Option<YtDlpEntryJson>>>,
}

#[derive(Debug, Deserialize)]
struct YtDlpEntryJson {
    #[serde(rename = "_type")]
    entry_type: Option<String>,
    ie_key: Option<String>,
    id: Option<String>,
    title: Option<String>,
    url: Option<String>,
    webpage_url: Option<String>,
    original_url: Option<String>,
    duration: Option<f64>,
    duration_string: Option<String>,
    upload_date: Option<String>,
    timestamp: Option<i64>,
    release_timestamp: Option<i64>,
    description: Option<String>,
    channel: Option<String>,
    channel_id: Option<String>,
    channel_url: Option<String>,
    uploader: Option<String>,
    uploader_id: Option<String>,
    uploader_url: Option<String>,
    thumbnail: Option<String>,
    view_count: Option<i64>,
    like_count: Option<i64>,
    comment_count: Option<i64>,
    live_status: Option<String>,
    availability: Option<String>,
    age_limit: Option<i64>,
    categories: Option<Vec<String>>,
    tags: Option<Vec<String>>,
    entries: Option<Vec<Option<YtDlpEntryJson>>>,
}

impl YoutubeVideoMetadata {
    fn from_yt_dlp_entry(entry: &YtDlpEntryJson) -> Self {
        Self {
            youtube_id: clean_string(entry.id.clone()),
            title: clean_string(entry.title.clone()),
            webpage_url: clean_string(entry.webpage_url.clone()),
            original_url: clean_string(entry.original_url.clone()),
            duration_seconds: entry.duration,
            upload_date: clean_string(entry.upload_date.clone()),
            description: clean_string(entry.description.clone()),
            channel: clean_string(entry.channel.clone()),
            channel_id: clean_string(entry.channel_id.clone()),
            channel_url: clean_string(entry.channel_url.clone()),
            uploader: clean_string(entry.uploader.clone()),
            uploader_id: clean_string(entry.uploader_id.clone()),
            uploader_url: clean_string(entry.uploader_url.clone()),
            thumbnail_url: clean_string(entry.thumbnail.clone()),
            view_count: entry.view_count,
            like_count: entry.like_count,
            comment_count: entry.comment_count,
            categories: clean_strings(entry.categories.clone().unwrap_or_default()),
            tags: clean_strings(entry.tags.clone().unwrap_or_default()),
            yt_dlp_args_redacted: Vec::new(),
            duration_string: clean_string(entry.duration_string.clone()),
            timestamp: entry.timestamp,
            release_timestamp: entry.release_timestamp,
            live_status: clean_string(entry.live_status.clone()),
            availability: clean_string(entry.availability.clone()),
            age_limit: entry.age_limit,
            yt_dlp_version: None,
            metadata_downloaded_at: None,
            metadata_source: None,
            media_probe: None,
        }
    }
}

impl YoutubeVideoItem {
    fn merge_yt_dlp_entry(&mut self, entry: &YtDlpEntryJson) {
        if let Some(id) = clean_string(entry.id.clone()) {
            self.youtube_id = Some(id);
        }
        if let Some(title) = clean_string(entry.title.clone()) {
            self.title = Some(title);
        }
        if entry.duration.is_some() {
            self.duration_seconds = entry.duration;
        }
        if let Some(upload_date) = clean_string(entry.upload_date.clone()) {
            self.upload_date = Some(upload_date);
        }
        self.metadata = YoutubeVideoMetadata::from_yt_dlp_entry(entry);
    }
}

/// Discovers videos from a YouTube collection URL.
pub async fn discover_youtube_collection(
    url: &str,
    max_items: Option<u64>,
    config: &YtDlpConfig,
) -> Result<Vec<YoutubeVideoItem>, YoutubeError> {
    let client = YtDlpClient::new(config.clone());
    let (stdout, _) = client.discover_collection_json(url, max_items).await?;
    parse_collection_json(&stdout, max_items)
}

/// Parses yt-dlp collection JSON into video items.
pub fn parse_collection_json(
    bytes: &[u8],
    max_items: Option<u64>,
) -> Result<Vec<YoutubeVideoItem>, YoutubeError> {
    let parsed: YtDlpCollectionJson = serde_json::from_slice(bytes)?;
    let limit = max_items.unwrap_or(u64::MAX) as usize;
    let mut items = Vec::new();
    collect_video_items(parsed.entries.unwrap_or_default(), limit, &mut items);
    Ok(items)
}

fn collect_video_items(
    entries: Vec<Option<YtDlpEntryJson>>,
    limit: usize,
    items: &mut Vec<YoutubeVideoItem>,
) {
    for mut entry in entries.into_iter().flatten() {
        if items.len() >= limit {
            return;
        }
        if let Some(children) = entry.entries.take() {
            collect_video_items(children, limit, items);
            continue;
        }
        let Some(source_url) = source_url_from_entry(&entry) else {
            continue;
        };
        let fallback_id = format!("item-{}", items.len() + 1);
        let item_id = entry
            .id
            .as_deref()
            .map(sanitize_id)
            .filter(|value| !value.is_empty())
            .unwrap_or(fallback_id);
        let metadata = YoutubeVideoMetadata::from_yt_dlp_entry(&entry);
        items.push(YoutubeVideoItem {
            item_id,
            youtube_id: entry.id,
            title: entry.title,
            source_url,
            duration_seconds: entry.duration,
            upload_date: entry.upload_date,
            metadata,
            local_video_path: None,
        });
    }
}

/// Creates a single URL item.
pub fn single_url_item(url: String) -> YoutubeVideoItem {
    YoutubeVideoItem {
        item_id: sanitize_id(&url),
        youtube_id: None,
        title: None,
        source_url: url,
        duration_seconds: None,
        upload_date: None,
        metadata: YoutubeVideoMetadata::default(),
        local_video_path: None,
    }
}

/// Creates a local file item.
pub fn local_file_item(path: PathBuf) -> YoutubeVideoItem {
    let item_id = path
        .file_stem()
        .and_then(|value| value.to_str())
        .map(sanitize_id)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "local-file".to_string());
    YoutubeVideoItem {
        item_id,
        youtube_id: None,
        title: path
            .file_name()
            .and_then(|value| value.to_str())
            .map(str::to_string),
        source_url: path.to_string_lossy().into_owned(),
        duration_seconds: None,
        upload_date: None,
        metadata: YoutubeVideoMetadata::default(),
        local_video_path: Some(path),
    }
}

/// Enriches an item with yt-dlp metadata and writes a metadata snapshot.
pub async fn enrich_youtube_metadata(
    item: &mut YoutubeVideoItem,
    metadata_dir: &Path,
    config: &YtDlpConfig,
) -> Result<(), YoutubeError> {
    if item.local_video_path.is_some() {
        return Ok(());
    }
    tokio::fs::create_dir_all(metadata_dir).await?;
    let client = YtDlpClient::new(config.clone());
    let (stdout, report) = client.fetch_metadata_json(&item.source_url).await?;
    let entry: YtDlpEntryJson = serde_json::from_slice(&stdout)?;
    item.merge_yt_dlp_entry(&entry);
    item.metadata.yt_dlp_version = report.yt_dlp_version;
    item.metadata.metadata_downloaded_at = Some(chrono::Utc::now().to_rfc3339());
    item.metadata.metadata_source = Some("yt-dlp --dump-json".to_string());
    item.metadata.yt_dlp_args_redacted = report.args_redacted;

    let metadata_path = metadata_dir.join(format!("{}.metadata.json", item.item_id));
    let metadata = serde_json::to_vec_pretty(&item.metadata)?;
    tokio::fs::write(metadata_path, metadata).await?;
    Ok(())
}

/// Downloads media into `media_dir` and refuses paths outside its temporary directory.
pub async fn download_youtube_media(
    item: &mut YoutubeVideoItem,
    media_dir: &Path,
    config: &YtDlpConfig,
) -> Result<PathBuf, YoutubeError> {
    if let Some(path) = &item.local_video_path {
        return Ok(path.clone());
    }
    tokio::fs::create_dir_all(media_dir).await?;
    let temp_dir = media_dir.join(format!(".tmp-{}", item.item_id));
    if temp_dir.exists() {
        tokio::fs::remove_dir_all(&temp_dir).await?;
    }
    tokio::fs::create_dir_all(&temp_dir).await?;
    let output_template = temp_dir.join(format!("{}-%(id)s.%(ext)s", item.item_id));
    let archive = media_dir.parent().and_then(Path::parent).map(|work_dir| {
        work_dir
            .join("archives")
            .join("yt-dlp-media-download-archive.txt")
    });
    if let Some(archive_dir) = archive.as_ref().and_then(|path| path.parent()) {
        tokio::fs::create_dir_all(archive_dir).await?;
    }

    let client = YtDlpClient::new(config.clone());
    let (stdout, report) = client
        .download_media(&item.source_url, &output_template, archive)
        .await?;
    let printed_path = printed_download_path(&stdout).ok_or_else(|| {
        YoutubeError::Invalid("yt-dlp completed but did not print after_move:filepath".to_string())
    })?;
    let temp_root = temp_dir.canonicalize()?;
    let canonical_path = printed_path.canonicalize()?;
    if !canonical_path.starts_with(&temp_root) {
        return Err(YoutubeError::Invalid(format!(
            "yt-dlp reported output outside temporary directory: {}",
            canonical_path.display()
        )));
    }
    let probe = probe_downloaded_media(&canonical_path)?;
    let file_name = canonical_path.file_name().ok_or_else(|| {
        YoutubeError::Invalid("yt-dlp reported output without a file name".to_string())
    })?;
    let final_path = media_dir.join(file_name);
    if final_path.exists() {
        tokio::fs::remove_file(&final_path).await?;
    }
    tokio::fs::rename(&canonical_path, &final_path).await?;

    if let Some(info_path) = info_json_path(&canonical_path) {
        if info_path.exists() {
            if let Ok(bytes) = tokio::fs::read(&info_path).await {
                if let Ok(entry) = serde_json::from_slice::<YtDlpEntryJson>(&bytes) {
                    item.merge_yt_dlp_entry(&entry);
                }
            }
            if let Some(file_name) = info_path.file_name() {
                let final_info_path = media_dir.join(file_name);
                if final_info_path.exists() {
                    tokio::fs::remove_file(&final_info_path).await?;
                }
                tokio::fs::rename(&info_path, final_info_path).await?;
            }
        }
    }
    item.metadata.yt_dlp_version = report.yt_dlp_version;
    item.metadata.metadata_downloaded_at = Some(chrono::Utc::now().to_rfc3339());
    item.metadata.metadata_source = Some("yt-dlp --write-info-json".to_string());
    item.metadata.yt_dlp_args_redacted = report.args_redacted;
    item.metadata.media_probe = Some(probe);
    let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    Ok(final_path)
}

/// Applies a title and duration filter.
pub fn filter_youtube_item(item: &YoutubeVideoItem, filter: &YoutubeItemFilter) -> bool {
    let title = item.title.as_deref().unwrap_or("").to_ascii_lowercase();
    if let Some(needle) = &filter.title_contains {
        if !title.contains(&needle.to_ascii_lowercase()) {
            return false;
        }
    }
    if filter
        .title_excludes
        .iter()
        .any(|needle| !needle.is_empty() && title.contains(&needle.to_ascii_lowercase()))
    {
        return false;
    }
    if let Some(min) = filter.duration_min {
        if item
            .duration_seconds
            .map(|value| value < min)
            .unwrap_or(false)
        {
            return false;
        }
    }
    if let Some(max) = filter.duration_max {
        if item
            .duration_seconds
            .map(|value| value > max)
            .unwrap_or(false)
        {
            return false;
        }
    }
    true
}

/// Stable UUID for a YouTube source URL.
pub fn stable_youtube_source_id(source_url: &str) -> uuid::Uuid {
    uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_URL, source_url.as_bytes())
}

/// Stable child UUID under a parent UUID.
pub fn stable_child_id(parent: uuid::Uuid, label: &str) -> uuid::Uuid {
    uuid::Uuid::new_v5(&parent, label.as_bytes())
}

/// Converts a YouTube transcript segment into a generic retrieval document.
pub fn youtube_transcript_segment_to_search_document(
    video: &YoutubeVideoMetadata,
    segment: &text_core::TextSegmentContract,
) -> SearchDocument {
    let mut document = SearchDocument::from_text_segment_contract(segment);
    document.title = video.title.clone();
    if let Some(youtube_id) = &video.youtube_id {
        document
            .metadata
            .insert("youtube_id".to_string(), youtube_id.clone());
    }
    if let Some(channel) = &video.channel {
        document
            .metadata
            .insert("channel".to_string(), channel.clone());
    }
    if let Some(url) = video.webpage_url.as_ref().or(video.original_url.as_ref()) {
        document
            .metadata
            .insert("source_url".to_string(), url.clone());
    }
    document
}

/// Checks whether a command exists on PATH.
pub fn require_command(command: &str) -> Result<(), YoutubeError> {
    if resolve_command(command).is_some() {
        Ok(())
    } else {
        Err(YoutubeError::Invalid(format!(
            "required command `{command}` was not found on PATH"
        )))
    }
}

fn resolve_command(command: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|dir| dir.join(command))
            .find(|path| path.is_file())
    })
}

pub(crate) fn printed_download_path(stdout: &[u8]) -> Option<PathBuf> {
    String::from_utf8_lossy(stdout)
        .lines()
        .rev()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(PathBuf::from)
}

fn info_json_path(media_path: &Path) -> Option<PathBuf> {
    let parent = media_path.parent()?;
    let stem = media_path.file_stem()?.to_str()?;
    Some(parent.join(format!("{stem}.info.json")))
}

fn probe_downloaded_media(path: &Path) -> Result<MediaProbeMetadata, YoutubeError> {
    let metadata = video_analysis_ffmpeg::probe(path)?;
    if metadata.duration_seconds.unwrap_or(0.0) <= 0.0 {
        return Err(YoutubeError::Invalid(format!(
            "ffprobe returned no positive duration for {}",
            path.display()
        )));
    }
    if metadata.width == 0 || metadata.height == 0 {
        return Err(YoutubeError::Invalid(format!(
            "ffprobe returned no video stream for {}",
            path.display()
        )));
    }
    Ok(MediaProbeMetadata {
        input: metadata.input,
        path: metadata.path,
        width: Some(metadata.width),
        height: Some(metadata.height),
        frame_rate: Some(format!(
            "{}/{}",
            metadata.frame_rate.numer(),
            metadata.frame_rate.denom()
        )),
        duration_seconds: metadata.duration_seconds,
    })
}

fn source_url_from_entry(entry: &YtDlpEntryJson) -> Option<String> {
    if !is_video_entry(entry) {
        return None;
    }
    for value in [&entry.webpage_url, &entry.original_url, &entry.url]
        .into_iter()
        .flatten()
    {
        if is_youtube_video_url(value) {
            return Some(value.clone());
        }
    }
    if let Some(id) = entry.id.as_deref().or(entry.url.as_deref()) {
        if is_probable_youtube_video_id(id) {
            return Some(format!("https://www.youtube.com/watch?v={id}"));
        }
    }
    None
}

fn is_video_entry(entry: &YtDlpEntryJson) -> bool {
    entry.ie_key.as_deref() == Some("Youtube")
        || entry.entry_type.as_deref() == Some("url")
        || entry
            .webpage_url
            .as_deref()
            .is_some_and(is_youtube_video_url)
        || entry.url.as_deref().is_some_and(is_youtube_video_url)
        || entry
            .id
            .as_deref()
            .is_some_and(is_probable_youtube_video_id)
}

fn is_youtube_video_url(value: &str) -> bool {
    value.starts_with("https://www.youtube.com/watch?")
        || value.starts_with("https://youtube.com/watch?")
        || value.starts_with("https://youtu.be/")
        || value.starts_with("https://www.youtube.com/shorts/")
        || value.starts_with("https://youtube.com/shorts/")
}

fn is_probable_youtube_video_id(value: &str) -> bool {
    value.len() == 11
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
}

fn sanitize_id(value: &str) -> String {
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
        "item".to_string()
    } else {
        sanitized
    }
}

fn clean_string(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn clean_strings(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .filter_map(|value| clean_string(Some(value)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_flat_video_entries() {
        let json = br#"{
            "entries": [
                {
                    "_type": "url",
                    "ie_key": "Youtube",
                    "id": "vA8-T5R8zDY",
                    "title": "Documentary",
                    "url": "https://www.youtube.com/watch?v=vA8-T5R8zDY",
                    "duration": 42730.0
                }
            ]
        }"#;

        let items = parse_collection_json(json, None).unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].youtube_id.as_deref(), Some("vA8-T5R8zDY"));
        assert_eq!(
            items[0].source_url,
            "https://www.youtube.com/watch?v=vA8-T5R8zDY"
        );
    }

    #[test]
    fn flattens_channel_tab_entries_and_ignores_collection_rows() {
        let json = br#"{
            "entries": [
                {
                    "_type": "playlist",
                    "id": "UCyvKnffD7Mh8t7kPMh70yIw",
                    "title": "Distinguo - Videos",
                    "entries": [
                        {
                            "_type": "url",
                            "ie_key": "Youtube",
                            "id": "P316q0D4q0Y",
                            "title": "A Complete DESTRUCTION of FAITH ALONE",
                            "url": "https://www.youtube.com/watch?v=P316q0D4q0Y"
                        }
                    ]
                },
                {
                    "_type": "playlist",
                    "id": "UCyvKnffD7Mh8t7kPMh70yIw",
                    "title": "Distinguo - Shorts"
                }
            ]
        }"#;

        let items = parse_collection_json(json, None).unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].youtube_id.as_deref(), Some("P316q0D4q0Y"));
    }

    #[test]
    fn filters_by_title_and_duration() {
        let item = YoutubeVideoItem {
            item_id: "one".to_string(),
            youtube_id: None,
            title: Some("Long Rust Talk".to_string()),
            source_url: "https://example.com".to_string(),
            duration_seconds: Some(600.0),
            upload_date: None,
            metadata: YoutubeVideoMetadata::default(),
            local_video_path: None,
        };

        assert!(filter_youtube_item(
            &item,
            &YoutubeItemFilter {
                title_contains: Some("rust".to_string()),
                duration_min: Some(60.0),
                duration_max: Some(700.0),
                ..YoutubeItemFilter::default()
            }
        ));
        assert!(!filter_youtube_item(
            &item,
            &YoutubeItemFilter {
                title_excludes: vec!["talk".to_string()],
                ..YoutubeItemFilter::default()
            }
        ));
    }

    #[test]
    fn derives_stable_source_ids() {
        assert_eq!(
            stable_youtube_source_id("https://www.youtube.com/watch?v=vA8-T5R8zDY"),
            stable_youtube_source_id("https://www.youtube.com/watch?v=vA8-T5R8zDY")
        );
    }

    #[test]
    fn parses_last_printed_download_path() {
        let path = printed_download_path(b"[download] progress\n/tmp/video.mp4\n").unwrap();

        assert_eq!(path, PathBuf::from("/tmp/video.mp4"));
    }
}
