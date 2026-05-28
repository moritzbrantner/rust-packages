# Runtime Surfaces

The same operation should be callable through Rust library code, CLI, HTTP API,
Tauri, WASM, and a future mobile bridge while preserving the same request/result
contracts.

## Adapter Pattern

```text
core operation:
  analyze(request) -> OperationResult<T>

CLI:
  reads JSON or flags
  calls core operation
  writes JSON result

API:
  POST /api/<operation>
  calls core operation
  returns same JSON result

Tauri:
  invoke("<operation>", { request })
  calls core operation
  returns same JSON result

WASM:
  exported function accepts JsValue request
  returns JsValue result

Mobile:
  default: call API
  later: UniFFI or native bridge for selected crates
```

Wrappers can translate transport errors into the local app error system, but a
successful operation response should keep `OperationResult<T>` intact:

```text
{
  "value": ...,
  "diagnostics": [],
  "artifacts": []
}
```

## Required Package Surface

Every non-wrapper Rust library crate under `crates/*/*` owns a `surface` module
and every transport delegates to it:

```text
<crate>::surface::package_surface()
<crate>::surface::run_surface_operation(request)
```

The required companion surfaces are:

- `<crate>-cli`
- `<crate>-server`
- `crates/bindings/<crate>-wasm`
- `packages/<crate>-wasm`
- `packages/<crate>-app`

CLI, HTTP, WASM, and Vite app packages may translate transport concerns, but
they must not own operation behavior. Library crates own operation metadata,
example requests, validation, and execution.

The shared DTOs live in `video-analysis-core::runtime`:

- `PackageSurface`
- `SurfaceOperation`
- `SurfaceRequest`
- `SurfaceResponse`

Generic `OperationResult<T>`, `JobResult<T>`, `JobManifest`, `ArtifactRef`, and
artifact stores live in `jobs-core`.

Default surface calls must be side-effect free. They may validate inputs,
return deterministic summaries, build plans, and describe runtime capabilities,
but they must not perform network access, external command execution, native
model inference, large filesystem writes, or persistent index writes. Side
effects belong behind explicit CLI/server/job routes, with long-running model
work routed through `model-runtime::jobs`.

## Retired Runtime Surfaces

The older `runtime-artifacts` and `runtime-jobs` crates are intentionally
excluded from the active workspace while their responsibilities are consolidated
into the current ownership model. Generic job state, results, artifact
references, checksum validation, and artifact stores live in `jobs-core`.
Model-specific bundle metadata, model sources, downloads, and validators live in
`model-runtime`.

Do not add new dependencies on the retired runtime crates. Do not recreate the
retired frontend app packages; route generic job concepts to `jobs-core` and
model-specific runtime concepts to `model-runtime`.

The generated baseline operation for every crate is `describe`, which returns a
serializable summary of the library surface. Crates can add richer
representative operations in their own `surface` module without changing the
transport wrappers.

## Runtime Tiers

Tier 1: pure Rust core

- vector distance
- text statistics
- graph algorithms
- deterministic simulations

Tier 2: WASM-compatible

- tokenizer
- small image transforms
- vector search over small data
- lightweight simulation

Tier 3: native desktop/server

- FFmpeg workflows
- repo scanning
- OSM PBF processing
- SQLite-backed workflows

Tier 4: server-only/heavy

- Qdrant-backed search
- large video indexing
- model/GPU-heavy inference

## Capability Routing

Use `RuntimeCapabilities` to choose execution location:

```text
If an operation is light and WASM-compatible:
  run it in browser/mobile/client.

If it needs native dependencies:
  run it in Tauri/native desktop or server.

If it is heavy or needs Qdrant/FFmpeg/GPU/filesystem:
  run it on the server/API.

But the request/result contracts stay the same.
```

Artifact-producing operations should write files through an artifact store and
return `jobs_core::ArtifactRef[]` instead of leaking ad hoc output paths into
each result type. The first shared local layout is:

```text
.workflow-output/
  jobs/
    <job-id>/
      manifest.json
      artifacts/
        ...
      logs/
        run.log
```
