# Contributing

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
