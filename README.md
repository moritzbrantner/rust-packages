# Rust Packages

This repository is a Cargo workspace for small Rust crates.

## Packages

- `linguistics-core`: shared text spans, tokens, and language tags.
- `linguistics-phonetics`: IPA-oriented phoneme and feature utilities.
- `linguistics-morphology`: morpheme models and a deterministic segmenter.
- `linguistics-syntax`: parts of speech and dependency tree helpers.
- `huggingface-model-*`: one metadata crate for each top-level Hugging Face
  model task category.
- `huggingface-space-*`: one metadata crate for each Hugging Face Spaces
  app-directory category.

## Development

Run the full test suite with:

```sh
cargo test --workspace
```
