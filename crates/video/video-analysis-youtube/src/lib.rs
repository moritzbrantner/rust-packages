#![doc = include_str!("../README.md")]

pub mod captions;
pub mod source;
pub mod surface;
pub mod yt_dlp;

pub use captions::{
    download_and_parse_youtube_captions, download_youtube_captions, parse_youtube_caption_files,
    CaptionDownloadRequest, YoutubeCaptionKind, YoutubeCaptionTrack,
};
pub use source::{
    discover_youtube_collection, download_youtube_media, enrich_youtube_metadata,
    filter_youtube_item, local_file_item, single_url_item, stable_child_id,
    stable_youtube_source_id, youtube_transcript_segment_to_search_document, MediaProbeMetadata,
    YoutubeItemFilter, YoutubeVideoItem, YoutubeVideoMetadata,
};
pub use yt_dlp::{
    classify_failure, BrowserCookieSource, CommandRunner, CommandRunnerError, CommandRunnerOutput,
    TokioCommandRunner, YtDlpClient, YtDlpCommandSpec, YtDlpConfig, YtDlpError, YtDlpErrorKind,
    YtDlpOperation, YtDlpRunReport,
};

use thiserror::Error;

#[derive(Debug, Error)]
/// Error type for reusable YouTube acquisition operations.
pub enum YoutubeError {
    #[error("{0}")]
    /// yt-dlp command error.
    YtDlp(#[from] YtDlpError),
    #[error("I/O error: {0}")]
    /// I/O error.
    Io(#[from] std::io::Error),
    #[error("invalid yt-dlp JSON: {0}")]
    /// JSON parse error.
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    /// Transcript parse error.
    Transcript(#[from] text_transcripts::TranscriptionError),
    #[error("ffmpeg probe failed: {0}")]
    /// Media probe error.
    Probe(#[from] video_analysis_ffmpeg::FfmpegError),
    #[error("{0}")]
    /// Invalid data or unsafe path.
    Invalid(String),
}
