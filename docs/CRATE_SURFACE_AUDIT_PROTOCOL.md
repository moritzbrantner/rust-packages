# Crate Surface Audit Protocol

Use this protocol when a thread asks:

```text
Apply the Crate Surface Audit Protocol to <crate-name>.
```

The goal is to audit one crate and its package UI the same way the COLMAP
package was audited: verify the operations actually work, make the primary
workflow obvious, move debug and inspection helpers into a Debug tab, and make
all outputs concrete enough to understand without reading the implementation.

## Scope

Audit one library crate at a time:

- Rust library crate: `crates/**/<crate-name>`
- server wrapper: `crates/**/<crate-name>-server`, if present
- CLI wrapper: `crates/**/<crate-name>-cli`, if present
- WASM binding crate/package, if present
- app package: `packages/<crate-name>-app`, if present
- shared UI package only when a general workbench capability is needed

Do not batch multiple crates unless the user explicitly asks for a batch.

## Investigation

Before editing, collect repo facts:

```bash
git status --short --branch
rg "<crate-name>" crates packages tests docs
cargo metadata --no-deps
```

Inspect these files when present:

- `crates/**/<crate-name>/src/lib.rs`
- `crates/**/<crate-name>/src/surface.rs`
- `crates/**/<crate-name>/README.md`
- `crates/**/<crate-name>-server/src/lib.rs`
- `crates/**/<crate-name>-server/tests`
- `crates/**/<crate-name>-cli/tests`
- `crates/bindings/<crate-name>-wasm/src/lib.rs`
- `packages/<crate-name>-app/src/App.tsx`
- package-surface tests that cover the affected UI behavior

List every operation from `package_surface()` and record:

- operation ID
- display name
- description
- example request
- runtime support flags
- observed output from `run_surface_operation`
- whether the operation is workflow, debug, or support

Run or test every operation with its example request. Confirm whether it returns
real domain output, debug metadata, a command/processing plan, a generic echo, a
placeholder, or an error.

## Operation Classification

Classify every operation exactly once.

### Workflow

Use for operations that perform the package's main user-facing job and produce
domain output, such as analysis, detection, decoding, generation, conversion,
export, reconstruction, processing, or report creation.

### Debug

Use for operations that inspect, preview, validate, summarize, list, describe,
or return metadata without performing the main job. This includes:

- `describe`
- `*.plan`
- `*.summary` when it only summarizes supplied data
- `*.validate`
- `*.catalog`
- command previews
- fixture/sample generators
- schema, model, or inventory helpers

### Support

Use only when a crate has a clearly separate reusable support category that is
not debug and not the main workflow. Otherwise put those operations under Debug.

## Rust Surface Fixes

Replace generic placeholder outputs with operation-specific responses.

Avoid generic output like:

```json
{
  "deterministic": true,
  "request": {},
  "plan": {
    "accepts": "metadata",
    "produces": "summary"
  }
}
```

Prefer concrete domain output:

```json
{
  "title": "Human-readable report title",
  "operation": "crate.operation.id",
  "summary": {
    "status": "ok",
    "primaryCount": 0
  },
  "message": "Clear explanation of what this operation does and does not do.",
  "details": {},
  "request": {}
}
```

For primary workflow operations, use this shape when practical:

```json
{
  "title": "Workflow result title",
  "operation": "crate.operation.id",
  "summary": {},
  "result": {},
  "diagnostics": [],
  "artifacts": []
}
```

Keep compatibility fields unless they are actively wrong. If replacing a map
with a list, keep the old map and add a clearer list field. Keep operation IDs
stable unless the old ID is incorrect and there are no known callers.

Validation rules:

- Reject malformed input with explicit errors.
- Do not silently return empty results for invalid required input.
- For optional empty input, either use a meaningful default or say no input was
  provided.
- Native and external-tool operations must clearly report missing tools, missing
  files, unsupported runtimes, and setup commands where applicable.

## UI Fixes

For `packages/<crate-name>-app/src/App.tsx`, configure operation groups:

```ts
operationGroups: [
  {
    id: "workflow",
    label: "Workflow",
    description: "Run the main package workflow.",
    operations: ["<main-operation-id>"],
  },
  {
    id: "debug",
    label: "Debug",
    description: "Inspect inputs, plans, metadata, and diagnostic helpers.",
    operations: ["describe", "...debug operation ids"],
  },
]
```

Set:

- `defaultOperation` to the primary workflow operation
- `featuredOperations` with workflow operations first
- presets only for useful user-facing workflows

Rename display names honestly:

- `Plan ...` -> `Preview ... plan`
- `List ...` -> `Inspect ... JSON` when it only reads inline JSON
- `Summarize ...` -> `Inspect ... summary` when it only summarizes supplied data

Do not imply filesystem scans, media decoding, model execution, or external-tool
execution unless the operation really performs that work.

Keep custom result tabs only for genuinely visual or domain-specific outputs.
The default Summary tab should expose `title`, `message`, and `summary` when
operations provide them.

## Tests

Add or update focused tests.

Rust library tests:

- `package_surface()` exposes expected operation IDs.
- Primary operation returns meaningful domain output for its example request.
- Debug operations return operation-specific outputs, not generic placeholders.
- Invalid input fails clearly.
- Native/server-only operations are marked correctly.

Server wrapper tests:

- `/api/package` exposes operation metadata.
- `/api/run` dispatches to the library surface.
- Native/server-only operations use the native dispatch path when applicable.

UI tests:

- App defaults to the primary workflow operation.
- `Workflow` and `Debug` tabs render when configured.
- Debug operations are under Debug and hidden from the Workflow select.
- Primary workflow remains runnable from the default view.
- Result summary shows `title`, `message`, and `summary`.

Run Playwright or component visual checks when the app has custom visual output:
canvas, media preview, graphs, maps, timelines, 3D viewers, or other non-JSON
views.

## Verification

Run the focused checks for the audited crate:

```bash
cargo test -p <crate-name> --lib
cargo test -p <crate-name>-server --tests
bun run --cwd packages/<crate-name>-app typecheck
bun run --cwd packages/video-analysis-ui test:unit
```

Run when relevant:

```bash
cargo test -p <crate-name>-cli --tests
cargo test -p <crate-name>-wasm --lib
bun run --cwd packages/video-analysis-ui typecheck
bun run snapshot:check
bun run hygiene:generated
git diff --check
```

For larger or cross-package work, run:

```bash
bun run test
```

## Docs

Update:

- `crates/**/<crate-name>/README.md`
- `docs/API_CONTRACTS.md` if public behavior or semantics changed
- `docs/PACKAGE_SURFACE_MATRIX.md` if representative operations changed
- app README only when app usage changed materially

Each crate README should state:

- primary workflow operations
- debug operations
- native/server-only or external-tool requirements
- sample data and setup commands
- what the operations do not do when that could be confusing

## Commit And Handoff

Use one focused commit per crate audit unless shared UI infrastructure needs a
separate commit.

Suggested commit messages:

```text
Audit <crate-name> package surface
Add package operation groups and audit <crate-name>
```

Do not push unless explicitly requested.

Final handoff must include:

- the crate audited
- key workflow/debug classification decisions
- files changed
- checks run
- checks skipped, with reasons
- final `git status --short --branch`

## Reusable Future Prompt

```text
Apply the Crate Surface Audit Protocol to <crate-name>.

Audit whether every package-surface operation actually works, fix placeholder or
misleading behavior, make the primary workflow the default UI path, move
debug/inspection helpers into a Debug operation tab, improve outputs so they
have useful title/summary/message fields, update docs and tests, and run the
focused verification commands.
```
