# Release Checklist

Use this checklist before tagging a release or publishing any crate wave.

## Required Gates

Run the workspace Rust gate:

```bash
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo doc --workspace --no-deps
```

Run the frontend gate when the published surface or docs reference the UI/web
packages:

```bash
bun install
bun run ui:build
bun run ui:test
bun run web:typecheck
bun run web:build
bun run web:test
```

This gate includes UI browser e2e tests and web API integration tests.

Run the external checks only for crates or docs that depend on those toolchains:

```bash
scripts/check.sh
scripts/check_e2e_external_tools.sh
```

## Package Dry Runs

Every publishable crate in the release wave must pass a package dry run before
tagging:

```bash
cargo package --allow-dirty -p <crate-name>
```

Recommended publish order:

1. Wave 1 foundations: `video-analysis-core`, `jobs-core`, `model-runtime`,
   `audio-analysis-core`, `image-analysis-core`, `text-core`,
   `vector-analysis-core`, `three-d-processing-core`, `data-inversion-core`,
   `numbers-core`, `tensor-data`, `math-linear`, `math-statistics`,
   `dense-data`, `video-analysis-ingest`
2. Wave 2 pure-Rust leaf crates on top of the foundations
3. Wave 3 external/runtime crates
4. Wave 4 `video-analysis` and `video-analysis-cli`

For a numerics and dense-data release slice, publish and dry-run packages in
this order so crates.io can resolve each internal dependency from the registry:

1. Confirm `video-analysis-core` is already available at the required version,
   or publish it first.
2. Publish `tensor-data`.
3. Publish `vector-analysis-core`.
4. Publish `numbers-core`.
5. Publish `math-linear`.
6. Publish `math-statistics`.
7. Publish `dense-data`.

For a text-only release wave, publish and dry-run packages in this order so
crates.io can resolve each internal dependency from the registry:

1. Prerequisites: `video-analysis-core`, `numbers-core`, `math-signal-core`,
   `tensor-data`, `audio-analysis-core`, `data-inversion-core`, `jobs-core`,
   `vector-analysis-core`, `math-sparse-data`, `model-runtime`,
   `vector-analysis-index`, `video-analysis-ingest`
2. Text foundations: `text-core`, `text-lexical`, `text-model-runtime`
3. Text feature crates: `text-embeddings`, `text-transcripts`,
   `text-linguistics`, `text-retrieval`, `text-analysis`,
   `text-classification`, `text-generation`, `text-generation-linguistics`,
   `text-question-answering`
4. Text adapters: publish each `crates/text/*-cli` and
   `crates/text/*-server` crate after its paired library crate is published

For an audio-only release wave, publish and dry-run packages in this order so
crates.io can resolve each internal dependency from the registry:

1. Prerequisites: confirm these crates are already available at the required
   versions, or publish them first: `moritzbrantner-video-analysis-core`,
   `moritzbrantner-runtime-core`, `moritzbrantner-tensor-data`,
   `moritzbrantner-math-signal-core`, `moritzbrantner-video-analysis-ingest`,
   `moritzbrantner-video-analysis-ffmpeg`, `moritzbrantner-model-runtime`,
   `moritzbrantner-text-transcripts`, `moritzbrantner-jobs-core`, and
   `moritzbrantner-data-inversion-core`.
2. Audio foundation: `moritzbrantner-audio-analysis-core`.
3. Pure audio feature crates that depend directly on the foundation:
   `moritzbrantner-audio-analysis-fourier`,
   `moritzbrantner-audio-analysis-pitch`,
   `moritzbrantner-audio-analysis-rhythm`,
   `moritzbrantner-audio-analysis-processing`, and
   `moritzbrantner-audio-analysis-io`.
4. Recognition layer: `moritzbrantner-audio-analysis-recognition`.
5. Recognition-backed feature crates: `moritzbrantner-audio-analysis-separation`,
   `moritzbrantner-audio-analysis-speakers`, and
   `moritzbrantner-audio-analysis-synthesis`.
6. Generation layer: `moritzbrantner-audio-generation-midi`.
7. Audio adapters: dry-run and publish each paired `-cli`, `-server`, and
   Rust `-wasm` crate after its paired library crate is published.

Do not publish:

- `runtime-artifacts`
- `runtime-jobs`
- `audio-analysis-test-support`
- `video-analysis-test-support`
- `video-analysis-use-cases`

## Contract Rules

- Treat the core contract crates as additive-only within this release cycle.
- Do not introduce new warnings in publishable crates.
- Keep external-tool coverage opt-in and non-blocking for normal contributor
  workflows.
- Runtime surface DTOs live in `video-analysis-core::runtime`.
- Generic jobs, result envelopes, and artifact storage live in `jobs-core`.
- Model-specific downloads, bundle manifests, and model artifact metadata live
  in `model-runtime`.
