# image-analysis-captioning

Concrete image captioning contracts and presets for `video-analysis`.

Model download and bundle handling belong to `model-runtime`; this crate owns
caption outputs, captioner backend traits, and captioning catalog metadata.

## Runtime Surface

- `image.captioning.models` lists catalog entries.
- `image.captioning.schema` returns task and preset schema metadata.
- `image.captioning.imported` validates caller-supplied captions and scores
  without running captioning.
