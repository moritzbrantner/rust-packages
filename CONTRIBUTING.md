# Contributing

## Verification Levels

- Fast baseline: `scripts/check-fast.sh`
- Full release baseline: `scripts/check.sh`
- Release-readiness doc pass: `cargo doc --workspace --no-deps`
- Frontend-only checks: `bun run ui:build`, `bun run ui:test`, `bun run web:typecheck`, `bun run web:build`, `bun run web:test`

Use the fast baseline for normal code changes. Use the full baseline before release-oriented changes or when you touch external-tool integrations.
Before tagging or publishing crates, also require `cargo doc --workspace --no-deps`
and the package dry-run checklist in [docs/RELEASE_CHECKLIST.md](docs/RELEASE_CHECKLIST.md).

## Local Setup

Install the JavaScript workspace dependencies:

```bash
bun install
```

Run the fast workspace baseline:

```bash
scripts/check-fast.sh
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
