# Rust Linguistics Packages

This repository is a Cargo workspace for small Rust crates focused on
linguistics tooling.

## Packages

- `linguistics-core`: shared text spans, tokens, and language tags.
- `linguistics-phonetics`: IPA-oriented phoneme and feature utilities.
- `linguistics-morphology`: morpheme models and a deterministic segmenter.
- `linguistics-syntax`: parts of speech and dependency tree helpers.

## Development

Run the full test suite with:

```sh
cargo test --workspace
```
