# model-runtime

Generic model infrastructure for the workspace.

This crate owns model identity, sources, bundle materialization, downloads,
model-specific artifact metadata, and job helpers. Generic artifact storage and
validation live in `jobs-core`; model bundles and Hugging Face download logic
stay here. Domain crates should hide this layer behind operations such as object
detection, transcription, OCR, segmentation, embeddings, and classification.

## Runtime Surface

- `model.presets` lists preset ids and model spec summaries.
- `model.spec` validates a model spec and returns safe names, task, source,
  requested files, and revision.
- `model.bundlePlan` plans bundle manifest layout and artifact references
  without downloading or materializing files.
