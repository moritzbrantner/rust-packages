# video-analysis-runtime-contracts

Shared serializable DTOs for CLI, HTTP API, Tauri, WASM, and future mobile
operation surfaces.

This crate is published as `video-analysis-runtime-contracts` to avoid the
unrelated crates.io `runtime-contracts` package. Workspace crates depend on it
through the `runtime-contracts` Cargo dependency alias, so Rust imports remain
`runtime_contracts`.
