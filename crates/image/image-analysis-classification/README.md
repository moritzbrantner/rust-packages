# image-analysis-classification

Concrete image classification contracts and presets for `video-analysis`.

Model download and bundle handling belong to `model-runtime`; this crate owns
classification request/response types, classifier backend traits, and
classification catalog metadata.

## Runtime Surface

- `image.classification.models` lists catalog entries.
- `image.classification.schema` returns task and preset schema metadata.
- `image.classification.imported` validates caller-supplied labels and scores
  without running a classifier.
