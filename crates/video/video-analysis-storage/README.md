# video-analysis-storage

Dataset persistence and migration support for `moritzbrantner-video-analysis`.

## Feature flags

- No optional feature flags today.

## Legacy dataset persistence

The existing `AnalysisDataset` JSON/JSONL/directory formats remain supported for
compatibility. This crate does not rewrite those serialized shapes in place.

The `annotations` module provides the migration boundary to the neutral
`media-core::annotations` model. `annotation_dataset_from_video_dataset`
promotes common timing, source, selector, label, score, and analyzer data into
the neutral envelope and retains each complete legacy `DatasetRecord` as a JSON
payload, so fields that remain video-specific are not discarded.

Scene records map to `MediaRange` using their stored start/end positions. Track
records are anchored at their first timestamp rather than inventing an
end-exclusive range from a legacy last-observation timestamp. Consumers that
need richer track interval semantics can add them once the producing domain has
an explicit boundary contract.

New cross-media persistence and temporal queries should use
`media_core::annotations::AnnotationDataset`; the old dataset/storage/transform
surface remains available while consumers migrate.

## Example

```rust,ignore
use video_analysis_storage::{read_dataset_dir, write_dataset_dir};

write_dataset_dir("output/report", &Default::default())?;
let dataset = read_dataset_dir("output/report")?;

let _ = dataset;
```

## Package surface

Primary workflow: `video.storage.manifestPlan`.

Workflow operations:

- `video.storage.manifestPlan`: Builds a dataset manifest preview without writing files.
- `video.storage.jsonlPreview`: Serializes a capped preview of dataset records as JSON lines without writing files.

Debug operations:

- `describe`: inspect package metadata and runtime support.

Runtime support: library, CLI, server, and WASM wrappers expose these operations.

Run the primary workflow through the CLI:

```bash
cargo run -p moritzbrantner-video-analysis-storage-cli -- run \
  --operation video.storage.manifestPlan \
  --json '{"dataset":{"metadata":{"attributes":{},"created_at":null,"name":null,"schema_version":2,"source":null},"records":[]},"recordsPath":"records.jsonl"}'
```

Successful responses use the shared package-surface shape with `operation`,
`title`, `message`, `summary`, and `result`. Default surface calls are
deterministic, local-first, and do not download models, write persistent files,
or execute external tools unless an operation explicitly documents native or
external-tool execution.

## Related crates

- `media-core`
- `video-analysis-dataset`
- `video-analysis-output`
