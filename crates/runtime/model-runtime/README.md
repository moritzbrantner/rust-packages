# model-runtime

Generic model infrastructure for the workspace.

This crate owns model identity, sources, bundle materialization, downloads, and
job helpers. Domain crates should hide this layer behind operations such as
object detection, transcription, OCR, segmentation, embeddings, and
classification.
