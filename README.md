# Rust Packages

This repository keeps the ComfyUI interoperability crates, the `vanalyze` CLI,
the use-case prototypes and the remaining spatial incubator crates. The other
capability families moved to their own repositories (see
[`ADR 0012`](docs/adr/0012-capability-repository-split-and-agent-releases.md)):

| Family | Owner |
| --- | --- |
| Runtime, jobs, model runtime, ONNX, tensor/dense/graph/number data, math, vectors, media and audio contracts | [`moenarch-foundation`](https://github.com/moritzbrantner/moenarch-foundation) |
| Text and NLP crates | [`nlp-stack`](https://github.com/moritzbrantner/nlp-stack) |
| Image, video and vision crates | [`visual-analysis`](https://github.com/moritzbrantner/visual-analysis) |
| Audio analysis and generation | [`audio-analysis`](https://github.com/moritzbrantner/audio-analysis) |
| Scene detection | [`scenedetect-rs`](https://github.com/moritzbrantner/scenedetect-rs) |

Crates here consume those owners through exact crates.io versions, or through a
single pinned `visual-analysis` revision where a crate is not published yet.

Reviewed ownership and destinations are in
[`REPOSITORY_DESTINATION_MATRIX.md`](docs/REPOSITORY_DESTINATION_MATRIX.md) and
[`CONSUMER_RELEASE_MATRIX.md`](docs/CONSUMER_RELEASE_MATRIX.md). See
[AGENTS.md](AGENTS.md), [docs/development.md](docs/development.md) and
[CONTRIBUTING.md](CONTRIBUTING.md) for local setup, and
[SECURITY.md](SECURITY.md) for vulnerability reporting. The workspace is
licensed under [LICENSE-MIT](LICENSE-MIT) or [LICENSE-APACHE](LICENSE-APACHE).

## Crates

- ComfyUI: [`moenarch-comfyui-data`](crates/comfyui/comfyui-data/README.md),
  [`moenarch-comfyui-latents`](crates/comfyui/comfyui-latents/README.md),
  [`moenarch-comfyui-models`](crates/comfyui/comfyui-models/README.md) and
  [`moenarch-image-analysis-comfyui`](crates/image/image-analysis-comfyui/README.md).
- CLI: [`moenarch-video-analysis-cli`](crates/video/video-analysis-cli/README.md)
  (`vanalyze`).
- Spatial incubator, being reconciled with 3d-lab and video-to-3d:
  `animation-core`, `three-d-processing-*`, `three-d-scene-svg`,
  `video-analysis-{sfm,mvs,reconstruction}`,
  `video-analysis-radiance-{fields,io,pipeline}`,
  `video-analysis-gaussian-splatting` and `video-analysis-posture{,-io}`.
- Root facade `moenarch-video-analysis`: re-exports the crates above behind the
  `comfyui`, `animation`, `three-d` and `spatial` features, and ships the
  `video-analysis-overview-server` used by the web prototype.

Each library has `-cli`, `-server` and `crates/bindings/*-wasm` adapters plus
`packages/*-app` and `packages/*-wasm` frontends that delegate to the library's
`surface::run_surface_operation`.

## Repository Layout

- `crates/`: Rust libraries and their adapters.
- `packages/`: frontend packages, including `@moritzbrantner/video-analysis-ui`.
- `prototypes/`: the `video-analysis-use-cases` workflows and the
  `video-analysis-web` Vite app.

## Workspace Checks

```bash
scripts/check-fast.sh      # changed-aware contributor gate
scripts/check-preflight.sh # broad local CI mirror
scripts/check.sh           # full baseline, including external tools
```

The generated snapshots ([dependency graph](docs/DEPENDENCY_GRAPH.md),
[package surface matrix](docs/PACKAGE_SURFACE_MATRIX.md),
[crate progress ledger](docs/CRATE_PROGRESS_LEDGER.md) and the destination
matrices) are checked by `scripts/check_generated_snapshots.sh`.

## Web Prototype

```bash
bun install
bun run ui:build
bun run web:dev
```

`web:dev` starts `video-analysis-overview-server` and the Vite app, which lists
every package wrapper in this workspace.
