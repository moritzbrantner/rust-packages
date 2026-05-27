# image-analysis-embeddings

Concrete image and face embedding contracts and presets for `video-analysis`.

Model download and bundle handling belong to `model-runtime`; this crate owns
embedding values, embedder backend traits, and embedding catalog metadata.

## Runtime Surface

- `image.embeddings.models` lists image and face embedding catalog entries.
- `image.embeddings.schema` returns task and preset schema metadata.
- `image.embeddings.validate` validates imported image or face vectors and
  optional face regions without computing learned embeddings.
