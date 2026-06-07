# Crate Progress Policy

This repository tracks crate progress as generated, reviewable maturity data.
The goal is to make forward movement visible, catch drift early, and keep each
library crate usable through the shared package-surface model.

## Maturity Levels

Each audited library crate is assigned exactly one level.

### L0 Scaffolded

The crate exists, but one or more core usability pieces are incomplete.
Typical causes include a missing `src/surface.rs`, missing companion packages,
missing operations, or app configuration that cannot identify a primary
workflow.

### L1 Discoverable

The crate exposes a discoverable package surface:

- `package_surface()` is present in `src/surface.rs`.
- `describe` is exposed.
- At least two crate-specific operations are listed.
- Operation IDs are unique and non-empty.

### L2 Executable

The crate declares executable operations with usable examples:

- Every operation has name, description, input schema, output schema, and
  example request metadata.
- Operation examples are structured JSON objects.
- The repository-wide `scripts/audit_package_surfaces.py --quality` gate runs
  the examples through the CLI and checks the shared response shape.
- Scaffold response text is absent from the crate-owned surface source.

### L3 Transport Complete

The crate has the expected runtime adapter set:

- Adjacent `<crate>-cli` package.
- Adjacent `<crate>-server` package.
- `crates/bindings/<crate>-wasm` Rust WASM binding crate.
- `packages/<crate>-wasm` Bun package.
- `packages/<crate>-app` Vite package.

Wrappers must delegate to the library-owned surface instead of owning package
behavior.

### L4 Usable

The crate is usable from the default package UI path:

- The app defaults to a real workflow operation, not `describe`.
- Workflow and Debug operation groups are configured.
- Featured operations prioritize workflow operations.
- The README documents package-surface usage or primary workflow operations.
- Focused tests cover the primary workflow or surface dispatch.
- Known scaffold/debug-only output has been replaced with domain-specific
  output.

## Regression Rule

For touched crates, maturity level and score are not allowed to decrease
relative to the configured base ref. Temporary exceptions must be explicit,
crate-specific, and non-expired in `scripts/crate_progress_regressions.allow`.

Shared infrastructure changes, including root workspace metadata, runtime-core,
package-surface UI code, audit scripts, generated progress policy, and CI
workflow files, trigger the touched-crate audit for every audited crate.
