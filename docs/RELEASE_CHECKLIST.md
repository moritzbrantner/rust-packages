# Release Checklist

> ADR 0012 and [`AGENT_DRIVEN_RELEASES.md`](AGENT_DRIVEN_RELEASES.md) supersede
> this checklist's historical manual-publication policy. Package-specific
> command sequences below remain useful gate/order references, but publication
> requires an exact issue and validated manifest and runs through the authorized
> trusted-publishing workflow.

Use this checklist before tagging a release or publishing any crate wave.

## Required Gates

Run the broad local CI/preflight mirror. This intentionally includes
all-target Rust coverage, package surface quality checks, production frontend
builds, and browser e2e:

```bash
scripts/check-preflight.sh
```

Run the workspace Rust release gate when validating a publish wave directly:

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

The fast local gate, `scripts/check-fast.sh`, is for iteration and normal
handoff. It intentionally excludes browser e2e, production web builds, and
benchmarks. Benchmark checks belong to `bun run bench`, `performance-ci`, or
explicit benchmark commands.

## Contract-Readiness Gate

Run this gate before choosing any publish wave. It checks contract ownership,
adapter parity, deterministic default surfaces, generated artifact hygiene, and
release checklist wording; it does not publish crates.

```bash
cargo test --test contract_ownership --test dependency_layers --test foundation_surface_audit --test package_structure --test package_interop_pipeline
bun run snapshot:check
bun run hygiene:generated
cargo fmt --check
git diff --check
```

## Package Dry Runs

Every publishable crate in the release wave must pass a package dry run before
tagging:

```bash
cargo package --locked -p <crate-name>
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

For the Model Runtime Spine release slice, prepare and publish the smallest
runtime/model foundation before the full Wave 1 foundation release. This slice
is for package consumers that need stable runtime DTOs, job/artifact contracts,
model bundle lifecycle behavior, and opt-in ONNX Runtime readiness. Publication
is agent-driven only after the exact release issue, manifest, and verification
commands pass.

Run the release Rust gates plus the opt-in ONNX spine smoke:

```bash
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo doc --workspace --no-deps
bash scripts/check_model_external_tools.sh onnx
cargo test --no-default-features --test model_runtime_spine_onnx --features external-tests -- --ignored
```

If the ONNX Runtime dynamic library is not discoverable, first run
`bash scripts/setup_model_external_tools.sh onnx` or set `ORT_DYLIB_PATH` to a
local ONNX Runtime shared library.

Dry-run and publish the spine crates in dependency order. Because Cargo package
verification resolves registry dependencies, dry-run each dependent crate after
its prerequisite version has been published to crates.io.

```bash
cargo package --locked -p moenarch-runtime-core
cargo publish -p moenarch-runtime-core

cargo package --locked -p moenarch-jobs-core
cargo publish -p moenarch-jobs-core

cargo package --locked -p moenarch-video-analysis-core
cargo publish -p moenarch-video-analysis-core

cargo package --locked -p moenarch-runtime-onnx
cargo publish -p moenarch-runtime-onnx

cargo package --locked -p moenarch-model-runtime
cargo publish -p moenarch-model-runtime
```

`model-runtime` owns model specs, bundle materialization, manifests, and
diagnostics. It does not own task inference semantics; task crates own
preprocessing, decoding, labels, spans, boxes, captions, embeddings, and other
domain-specific model behavior. `runtime-onnx` is part of this slice only as a
domain-neutral low-level runtime with explicit `onnxruntime` feature support.
The ONNX spine smoke downloads
`onnx-internal-testing/tiny-random-BertModel-ONNX` and opens
`onnx/model.onnx` from the materialized bundle.

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
- Runtime surface DTOs live in `moritzbrantner-runtime-core` and are
  re-exported from `video-analysis-core::runtime` for compatibility.
- Generic jobs, result envelopes, and artifact storage live in `jobs-core`.
- Model-specific downloads, bundle manifests, and model artifact metadata live
  in `model-runtime`.
