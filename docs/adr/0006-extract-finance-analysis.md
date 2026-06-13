# ADR 0006: Extract Finance Analysis

## Status

Accepted

## Context

The finance crates reached L4 usable maturity with library, CLI, REST, WASM,
and app surfaces. They are coherent and useful, but their primary purpose is a
standalone finance analytics domain rather than this repository's core video,
audio, image, text, vector, animation, 3D, runtime, and interoperability scope.

Finance is therefore an Adjacent Domain Package Family: useful for some
multimodal workflows, but best owned and released outside this workspace.

## Decision

Extract the finance package family to the sibling `finance-analysis`
repository as a clean copy with one new repository history.

The extracted repository keeps the existing Rust and npm package names. The
finance Rust crates depend on published foundation crates from this repository,
not local path dependencies back into this workspace.

`rust-packages` removes finance runtime, package-surface, app, and WASM
implementation ownership. It keeps only deprecated empty doc modules at
`video_analysis::finance` and `video_analysis::finance_data` as migration
signposts.

## Consequences

- `rust-packages` loses finance runtime and package surfaces.
- Finance packages can evolve on their own release cadence.
- `rust-packages` keeps only deprecated doc stubs for finance module paths.
- Future geo/map extraction can reuse the Adjacent Domain Package Family
  language.
