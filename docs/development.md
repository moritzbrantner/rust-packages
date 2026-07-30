# Development

## Setup

Install the pinned Rust toolchain from `rust-toolchain.toml`, then add the WASM
target and helper used by `packages/text-core-wasm`:

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-pack --locked --version 0.14.0
```

Install the Bun workspace dependencies:

```bash
bun install
```

## Daily Commands

```bash
bun run dev           # Vite prototype app
bun run test          # fastest meaningful Rust + frontend unit/API tests
bun run lint          # clippy and TypeScript type checks
bun run format:check  # Rust formatting check
bun run build         # Rust workspace build and production frontend builds
bun run preflight     # broad local CI/preflight mirror
bun run verify        # full baseline through scripts/check.sh
bun run hygiene       # git status, upstream, and ignore audit
bun run hygiene:generated # tracked generated/local artifact guard
bun run snapshot:check    # reviewed generated docs freshness check
bun run ui:build          # regenerate untracked UI package dist
bun run web:build         # regenerate untracked web app dist
bun run web:build:pages   # regenerate static Pages output and crate indexes
```

For the normal fast local baseline, use:

```bash
scripts/check-fast.sh
```

This gate is changed-aware for local iteration. It always checks generated
artifacts and Rust formatting, checks affected reviewed generated snapshots,
then scopes Rust test/clippy, package-surface progress comparisons, and
frontend package tests to touched files when it can. Force broader local
validation with:

```bash
CHECK_FAST_SCOPE=workspace scripts/check-fast.sh
CHECK_FAST_FRONTEND=all scripts/check-fast.sh
CHECK_FAST_PROGRESS=all scripts/check-fast.sh
```

Use `CHECK_FAST_FRONTEND=none` or `CHECK_FAST_PROGRESS=none` only for emergency
local loops. `BASE_REF` defaults to `origin/main` for changed-file detection,
and `CHECK_FAST_RUST_JOBS` defaults from `CARGO_BUILD_JOBS`, then
`TEST_MAX_WORKERS`, then a capped local CPU count. The fast gate intentionally
skips browser e2e, production web builds, and benchmarks. For the broad local
CI/preflight mirror before PR or release-oriented changes, use:

```bash
scripts/check-preflight.sh
```

For the full local baseline with external-tool checks, use:

```bash
scripts/check.sh
```

## Pull-request CI

`workspace-ci` always runs a lightweight planner and repository sanity gate.
The planner classifies the exact pull-request diff and selects only the affected
Rust, frontend, WASM, Storybook, browser, architecture, or full-workspace jobs.
Changed Rust crates include their workspace reverse-dependency closure. Root
manifests, broad lockfiles, ownership maps, and release-plan inputs select the
full workspace. A final always-running `ci-gate` is the single stable required
check: it accepts legitimately unselected jobs but fails closed when a selected
job is skipped, cancelled, or unsuccessful. Add the `full-ci` label when an
ordinary change needs the broad path.

New commits cancel obsolete ordinary pull-request runs. Release, publication,
and deployment workflows are deliberately outside that cancellation policy.
The weekly `full-workspace-ci` keeps broad Rust, frontend/WASM, Storybook,
browser, generated-inventory, and package checks off the critical path of
unrelated pull requests.

The pinned reusable workflows cache Cargo registry, Git, and target data using
the runner OS and `Cargo.lock` hash; changing the lockfile invalidates the exact
cache key, while the OS restore prefix can still reuse downloads safely.
`wasm-pack` remains pinned to `0.14.0`, and Playwright installs the browser
version locked by the workspace dependencies. A UI change coalesces changed
WASM builds, UI and web browser E2E, and Storybook into one browser job; the
full-workspace path does the same. This prevents simultaneous selected jobs from
independently installing the same pinned browser or `wasm-pack`. The job graph
does not share caches or credentials with unmanaged runners. No self-hosted
runner is assumed by this configuration.

## Local Build Cache

Cargo and package build outputs are local artifacts. If the checkout looks large,
measure ignored build directories before treating source layout as the problem:

```bash
du -sh .cargo-target target 2>/dev/null || true
du -sh packages/*-wasm/pkg packages/video-analysis-ui/dist prototypes/web/video-analysis-web/dist 2>/dev/null || true
```

`.cargo-target` is the repo-local Rust build cache used by contributor scripts.
It can be deleted whenever you need to reclaim space:

```bash
rm -rf .cargo-target
```

The next `cargo` or workspace check rebuilds the artifacts it needs. Delete
`target` only if you have also built into Cargo's default target directory.
Generated WASM packages and frontend `dist/` directories are regenerated through
their build commands and should stay untracked.

Prefer crate-scoped Rust checks while iterating:

```bash
cargo check -p <crate>
cargo test -p <crate>
scripts/check-fast.sh
```

Move to `CHECK_FAST_SCOPE=workspace scripts/check-fast.sh`,
`scripts/check-preflight.sh`, or `scripts/check.sh` when the change crosses
crate boundaries, changes workspace membership, dependencies, features, package
surface generation, UI routing/build behavior, release readiness, or
external-tool coverage.

## External Tools

External tests are opt-in. Check availability without installing:

```bash
scripts/check_e2e_external_tools.sh
```

Install the default local tool set into ignored directories only when needed:

```bash
scripts/setup_e2e_external_tools.sh fast
```

The ignored local roots are `.external-test-tools/`, `.audio-tools/`,
`.model-runtime/`, and `.test-corpora/`.

Generated build outputs stay local and ignored:
`packages/video-analysis-ui/dist/`, `packages/*-wasm/pkg/`, and
`prototypes/web/video-analysis-web/dist/`. The web architecture JSON under
`prototypes/web/video-analysis-web/public/` is generated by the web build. Large
use-case media under `use-case-output/` is local output; keep only small reviewed
report snapshots in Git.

## Release Notes

Release work is checklist-driven, not automated. Before tagging or publishing,
run the gates in `docs/RELEASE_CHECKLIST.md`, including:

```bash
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo doc --workspace --no-deps
```

Benchmark checks belong to `bun run bench`, `performance-ci`, or explicit
benchmark commands. They are not part of the fast local baseline.

Run frontend gates when UI packages, web packages, or docs that reference them
change:

```bash
bun run ui:build
bun run ui:test
bun run web:typecheck
bun run web:build
bun run web:test
```

Use `cargo package --allow-dirty -p <crate-name>` for crate dry runs in the
publish wave. Do not publish `audio-analysis-test-support`,
`video-analysis-test-support`, or `video-analysis-use-cases`.

## Troubleshooting

- If browser tests fail because Chromium is missing, run
  `bun run --cwd packages/video-analysis-ui playwright install --with-deps chromium`.
- If `scripts/check.sh` fails before tests run, verify external prerequisites
  with `scripts/check_e2e_external_tools.sh`.
- If reviewed generated snapshot checks fail, run `bun run snapshot:check` and
  then the regeneration command printed by the failing check.
