# Agent Instructions

## Project Purpose

This repository is a Rust-first multimodal analysis workspace for video, audio,
image, text, vector, data, math, animation, 3D, and ComfyUI interoperability.
It also contains Bun-managed TypeScript packages and a Vite prototype app used
to exercise and display the Rust package surfaces.

## Orientation

- Start with `rg --files | sed -n '1,200p'` for a fast file inventory.
- Use `rg "<symbol-or-text>" crates packages prototypes tests scripts docs` for
  exact matches.
- Use Semble when semantic search is available, for example
  `semble search --repo . "how package reports are generated"` or the Codex
  Semble search tool with `repo="."`.
- Use `cargo metadata --no-deps` when workspace membership or crate dependency
  boundaries matter.
- Check `git status --short --branch` before edits and again before handoff.

## Ownership Boundaries

- `crates/`: reusable Rust crates. Keep library logic composable and avoid
  adding app/runtime concerns to core crates.
- `src/lib.rs`: root `video-analysis` facade crate.
- `tests/`: workspace integration and public API smoke tests.
- `packages/`: reusable frontend packages. `packages/video-analysis-ui/dist/`
  is checked in package output; regenerate it with `bun run ui:build`.
- `packages/text-core-wasm/`: WASM package generated through `wasm-pack`.
  `packages/text-core-wasm/pkg/` is generated output; regenerate through the
  package build flow.
- `prototypes/`: runnable experiments and local app surfaces. The web app lives
  in `prototypes/web/video-analysis-web`.
- `references/pyscenedetect/`: vendored upstream behavior reference. Treat as
  read-only unless the task is explicitly about syncing or documenting the
  reference.
- `vendor/whisper.cpp/`: vendored runtime source. Treat as read-only unless the
  task explicitly targets the vendor tree.
- `use-case-output/` and `tests/fixtures/`: checked-in fixtures and sample
  reports. Do not refresh them unless the requested behavior or test fixtures
  require it.

## Commands

Install prerequisites:

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-pack --locked --version 0.14.0
bun install
```

`bun install` needs access to the configured GitHub Packages registry for
`@moritzbrantner/ui`; set `GH_PACKAGES_TOKEN` when required.

Standard root commands:

```bash
bun run dev           # start the Vite prototype app
bun run test          # fastest meaningful Rust + frontend unit/API tests
bun run lint          # clippy and TypeScript type checks
bun run format        # write Rust formatting changes
bun run format:check  # check Rust formatting without mutation
bun run build         # Rust workspace build plus UI/web production builds
bun run verify        # full local baseline: scripts/check.sh
bun run hygiene       # lightweight repo status and ignore audit
```

Existing lower-level checks:

```bash
scripts/check-fast.sh               # normal contributor gate
scripts/check.sh                    # full release baseline with e2e checks
scripts/check_e2e_external_tools.sh # verify optional external tools
cargo doc --workspace --no-deps     # release-readiness docs pass
```

For big changes, run the relevant GitHub Actions workflow locally with `act`
before handoff. Prefer the workflow that matches the changed surface, for
example `act -W .github/workflows/workspace-ci.yml`.

Release and publish work is manual. Use `docs/RELEASE_CHECKLIST.md`; do not add
release automation or publish crates unless the task explicitly asks for that.

## Generated And Local Files

- Do not manually edit `target/`, `.cargo-target/`, `node_modules/`,
  `.external-test-tools/`, `.audio-tools/`, `.video-analysis-models/`,
  `.test-corpora/`, `coverage/`, Playwright reports, or test-result folders.
- Do not manually edit generated package outputs in
  `packages/video-analysis-ui/dist/`, `packages/text-core-wasm/pkg/`, or
  `prototypes/web/video-analysis-web/dist/`; regenerate them with the relevant
  build command when they are part of the requested change.
- Regenerate `docs/DEPENDENCY_GRAPH.md` after workspace crate membership or
  internal dependency changes with:

```bash
python3 scripts/generate_dependency_chart.py
```

## Expensive Or Opt-In Work

- `scripts/check.sh` includes browser e2e and external-tool coverage; run it
  before release-oriented handoff, but use `scripts/check-fast.sh` for normal
  iteration.
- `scripts/check-e2e.sh` requires local external tools. Run
  `scripts/setup_e2e_external_tools.sh fast` only when the task needs those
  checks.
- `scripts/check-e2e-slow.sh` is reserved for dedicated radiance/Nerfstudio
  runners.
- Feature flags named `external-tests` require real tools, models, or network
  access and are outside the default contributor gate.
