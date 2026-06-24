# Text App Verification

Use this guide to verify deterministic text crate functionality through the
package workbench apps. These checks do not require model downloads, ONNX,
Candle, or external tools.

## Root Catalog

Start the overview server and Vite catalog from the workspace root:

```bash
bun run dev
```

Open these routes:

```text
/text/
/categories/text/
/wrappers/text-core/
/wrappers/text-index/
/wrappers/text-analysis/
/wrappers/text-retrieval/
```

`/categories/text/` is the Text Family UI launcher. Verify that it shows the
workflow tiers `Analyze`, `Search`, `Task APIs`, `Foundations`, and
`Runtime Setup` instead of mounting every package frontend inline. `Text
Analysis` should be the primary entry and link to
`/wrappers/text-analysis/?preset=document-deterministic`. The `Search` tier
should list `Text Index` before `Text Embeddings`.

The `Compatibility And Adapters` section should be collapsed by default. Expand
it to verify that `Text Retrieval` remains reachable there, and confirm direct
navigation to `/wrappers/text-retrieval/` still mounts the runnable focused
workbench.

For each wrapper, run the default workflow in both runtime modes:

- `Overview Server`
- `Client WASM`

Use the primary `Scenario` dropdown to switch curated examples. Verify the
default scenarios are selected on first load:

- `/wrappers/text-core/`: `Tokenize transcript notes`
- `/wrappers/text-index/`: `Hybrid search`
- `/wrappers/text-analysis/`: `Document: deterministic report`

Debug/support operations must remain available in the same dropdown under the
`Debug` or `Support` option groups.

The JSON result should include:

```json
{
  "operation": "...",
  "title": "...",
  "message": "...",
  "summary": {},
  "result": {}
}
```

The page should not show a dynamic import failure.

The formatted summary cards for the default text scenarios should show concrete
counts or scores. They should not show empty primary summaries or `n/a` for the
main document, corpus, or similarity metrics.

## Individual Text Apps

Run a package app directly when checking one text surface:

```bash
bun run --cwd packages/text-core-app dev
```

Direct app development uses the same `PackageSurfaceWorkbench` as the catalog.
Client WASM requires the package WASM artifacts to exist. Regenerate text WASM
packages from the root when needed:

```bash
bun run text-wasm:build:all
```

## Server-Backed Mode

Standalone server mode expects the matching server wrapper. Use full Cargo
package names:

```bash
cargo run -p moenarch-text-core-server
cargo run -p moenarch-text-index-server
cargo run -p moenarch-text-analysis-server
```

Then use the app runtime switch to select `Standalone Server`.

## Local Checks

Use focused checks while changing text apps or WASM packages:

```bash
bun run text-app:typecheck
bun run text-wasm:test:all
bun run web:test:e2e
```

Rust package-surface contracts are covered by:

```bash
cargo test --test text_surface_audit --test text_surface_operations
```

Model-backed/native checks remain opt-in. Keep default text app verification on
deterministic package-surface operations unless a task explicitly targets model
runtime behavior.
