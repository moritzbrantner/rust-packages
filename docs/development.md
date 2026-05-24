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

The workspace uses GitHub Packages for `@moritzbrantner/ui`; set
`GH_PACKAGES_TOKEN` if Bun cannot read that package.

## Daily Commands

```bash
bun run dev           # Vite prototype app
bun run test          # fastest meaningful Rust + frontend unit/API tests
bun run lint          # clippy and TypeScript type checks
bun run format:check  # Rust formatting check
bun run build         # Rust workspace build and production frontend builds
bun run verify        # full baseline through scripts/check.sh
bun run hygiene       # git status, upstream, and ignore audit
```

For the normal contributor gate, use:

```bash
scripts/check-fast.sh
```

For the full local baseline before release-oriented changes, use:

```bash
scripts/check.sh
```

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

## Release Notes

Release work is checklist-driven, not automated. Before tagging or publishing,
run the gates in `docs/RELEASE_CHECKLIST.md`, including:

```bash
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo doc --workspace --no-deps
```

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

- If Bun install fails against `npm.pkg.github.com`, verify `GH_PACKAGES_TOKEN`
  and `.npmrc`.
- If browser tests fail because Chromium is missing, run
  `bun run --cwd packages/video-analysis-ui playwright install --with-deps chromium`.
- If `scripts/check.sh` fails before tests run, verify external prerequisites
  with `scripts/check_e2e_external_tools.sh`.
- If generated dependency graph checks fail, run
  `python3 scripts/generate_dependency_chart.py` and review the docs diff.
