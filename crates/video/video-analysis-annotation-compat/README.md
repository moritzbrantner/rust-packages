# video-analysis-annotation-compat

Compatibility adapter from the existing `video-analysis-dataset` retained-record
model into the neutral `media-annotations` interoperability model.

The dependency direction is intentionally one-way: this video-owned adapter may
depend on `media-annotations`, but the neutral annotation crate does not depend
on video, image, audio, text, pose, or tracking crates.

`annotation_dataset_from_video_dataset` maps the common timing, source,
selector, label, score, and analyzer fields where those concepts are explicit in
the legacy record. It also stores the complete serialized legacy record as the
annotation value, so conversion does not discard fields that remain
video-domain-specific.

Scene records become half-open media ranges using their existing start/end
positions. Tracks are anchored at their first timestamp when one is available;
the adapter does not invent an end-exclusive track duration from the legacy
`last_timestamp`, whose boundary semantics are observation-oriented rather than
range-oriented.

This crate is a migration surface, not a second canonical storage model. New
cross-media persistence and temporal queries should use `media-annotations`;
existing `video-analysis-dataset`, `video-analysis-storage`, and
`video-analysis-transform` consumers can migrate through this adapter without a
flag-day rewrite.
