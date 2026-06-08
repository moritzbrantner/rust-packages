# video-analysis-youtube

Reusable YouTube acquisition helpers for `yt-dlp` based workflows.

This crate owns command construction, failure classification, YouTube collection
discovery, metadata enrichment, media download safety checks, and caption
download/parsing. Transcript parsing and subtitle cleanup are delegated to
`moritzbrantner-text-transcripts`.
