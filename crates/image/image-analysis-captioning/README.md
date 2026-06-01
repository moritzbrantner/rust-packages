# image-analysis-captioning

Concrete image captioning contracts and presets for `moritzbrantner-video-analysis`.

Model download and bundle handling belong to `moritzbrantner-model-runtime`; this crate owns
caption outputs, captioner backend traits, and captioning catalog metadata.

## Runtime Surface

- Workflow operations: `image.captioning.imported` validates caller-supplied
  captions and scores into normalized caption values.
- Debug operations: `image.captioning.models`, `image.captioning.schema`, and
  `describe` inspect catalogs, schemas, and package metadata.
- The surface does not download models or run captioning inference.
