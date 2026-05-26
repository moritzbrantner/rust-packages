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
   `numbers-core`, `dense-data`, `video-analysis-ingest`
2. Wave 2 pure-Rust leaf crates on top of the foundations
3. Wave 3 external/runtime crates
4. Wave 4 `video-analysis` and `video-analysis-cli`

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
