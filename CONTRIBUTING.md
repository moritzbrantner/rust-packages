# Contributing

## Repository split checks

Changes to manifests, ownership, capability boundaries, or release tooling must
keep these narrow checks green:

```bash
python3 scripts/generate_repository_split_inventory.py --check
python3 scripts/test_generate_repository_split_inventory.py
python3 scripts/test_check_repository_boundaries.py
python3 scripts/check_repository_boundaries.py --check
python3 scripts/test_check_release_plan.py
python3 scripts/check_release_plan.py --check docs/repository-split/release-plan.example.json
```

Edit `docs/repository-split/package-ownership.json` directly for a reviewed
classification decision, then regenerate its destination matrices:

```bash
python3 scripts/generate_repository_split_inventory.py
```

## Verification Levels

- Fast local baseline: `scripts/check-fast.sh` (no browser e2e, production web
  build, or benchmarks)
- Broad local CI/preflight mirror: `scripts/check-preflight.sh`
- Full release/external baseline: `scripts/check.sh`
- Release-readiness doc pass: `cargo doc --workspace --no-deps`
- Frontend-only checks: `bun run ui:build`, `bun run ui:test`, `bun run web:typecheck`, `bun run web:build`, `bun run web:test`

Package surfaces use matching test layers: Rust libraries keep unit tests close
to implementation code, CLI and API adapters use integration tests, and UI
packages use browser e2e tests.

The generated all-crate dependency chart lives in
[docs/DEPENDENCY_GRAPH.md](docs/DEPENDENCY_GRAPH.md). Regenerate it after
changing workspace crate membership or internal dependencies:

```bash
python3 scripts/generate_dependency_chart.py
```

Use the fast baseline for normal code changes. Use the preflight mirror before
PR/release-oriented changes or when touching UI routing, production builds, or
browser e2e behavior. Use the full baseline when you touch external-tool
integrations.
Before tagging or publishing crates, also require `cargo doc --workspace --no-deps`
and the package dry-run checklist in [docs/RELEASE_CHECKLIST.md](docs/RELEASE_CHECKLIST.md).
Benchmark checks belong to `bun run bench`, `performance-ci`, or explicit
benchmark commands, not the default fast local gate.

## Local Build Cache

Large local checkouts are usually dominated by ignored build artifacts, not by
tracked source files. Inspect the local caches before treating repository size
as an extraction or source-layout problem:

```bash
du -sh .cargo-target target 2>/dev/null || true
du -sh packages/*-wasm/pkg packages/video-analysis-ui/dist prototypes/web/video-analysis-web/dist 2>/dev/null || true
```

It is safe to remove the local Rust build cache when you need disk back. The
next Rust command will rebuild what it needs:

```bash
rm -rf .cargo-target
```

Use `rm -rf target` only if you have explicitly built into Cargo's default
target directory. Generated WASM and frontend outputs are also local build
products; prefer regenerating them with the package build flow instead of
checking them in.

Keep day-to-day checks narrow so `.cargo-target` grows only for the crates you
are changing:

```bash
cargo check -p <crate>
cargo test -p <crate>
scripts/check-fast.sh
```

Use broad workspace checks when the change affects shared public APIs, workspace
membership, dependency versions or feature flags, generated package surfaces,
UI routing/build behavior, release readiness, or external-tool integration.

## Local Setup

Install the Rust WASM build prerequisites used by `@moritzbrantner/text-core-wasm`:

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-pack --locked --version 0.14.0
```

Install the JavaScript workspace dependencies:

```bash
bun install
```

Run the fast workspace baseline:

```bash
scripts/check-fast.sh
```

Run the broad local preflight mirror before release-oriented handoff:

```bash
scripts/check-preflight.sh
```

Run the full baseline after external tools are installed:

```bash
scripts/check.sh
```

Run the release-readiness documentation pass before publishing:

```bash
cargo doc --workspace --no-deps
```

## External Tools

Verify external prerequisites without installing anything:

```bash
scripts/check_e2e_external_tools.sh
```

Install the default end-to-end tool set into gitignored local paths:

```bash
scripts/setup_e2e_external_tools.sh fast
```

Model-specific helpers are available through:

```bash
scripts/setup_model_external_tools.sh
scripts/check_model_external_tools.sh
```

Audio-specific helpers are available through:

```bash
scripts/setup_audio_external_tools.sh
```

## CI Expectations

- `workspace-ci` is the primary pull request gate for Rust and frontend changes.
- `audio-ci` covers scheduled audio-specific perf and external-tool jobs.
- `external-ci` is non-blocking scheduled coverage for ignored and tool-heavy checks.
