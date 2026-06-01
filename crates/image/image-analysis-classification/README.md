# image-analysis-classification

Concrete image classification contracts and presets for `moritzbrantner-video-analysis`.

Model download and bundle handling belong to `moritzbrantner-model-runtime`; this crate owns
classification request/response types, classifier backend traits, and
classification catalog metadata.

## Runtime Surface

- Workflow operations: `image.classification.imported` validates caller-supplied
  labels and scores into normalized classification values.
- Debug operations: `image.classification.models`,
  `image.classification.schema`, and `describe` inspect catalogs, schemas, and
  package metadata.
- The surface does not download models or run classifier inference.
