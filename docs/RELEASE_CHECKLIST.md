# Release Checklist

> ADR 0012 and [`AGENT_DRIVEN_RELEASES.md`](AGENT_DRIVEN_RELEASES.md) supersede
> this checklist's historical manual-publication policy. Package-specific
> command sequences below remain useful gate/order references, but publication
> requires an exact live issue authorization, validated manifest, clean exact
> release commit, and passing local gates. Local Cargo publication is allowed.

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
cargo test --test contract_ownership --test dependency_layers --test package_structure
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

Foundation, NLP, visual and audio crates are released from their owning
repositories (`moenarch-foundation`, `nlp-stack`, `visual-analysis`,
`audio-analysis`). Publish ComfyUI crates in dependency order:
`comfyui-data`, `comfyui-models`, `comfyui-latents`, then
`image-analysis-comfyui`, each followed by its `-cli` and `-server` adapters.

Do not publish:

- `video-analysis-test-support`
- `video-analysis-use-cases`

## Contract Rules

- Treat the core contract crates as additive-only within this release cycle.
- Do not introduce new warnings in publishable crates.
- Keep external-tool coverage opt-in and non-blocking for normal contributor
  workflows.
- Runtime surface DTOs, jobs and model bundles are owned by
  `moenarch-foundation` (`runtime-core`, `jobs-core`, `model-runtime`).
