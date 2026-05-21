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
return `ArtifactRef[]` instead of leaking ad hoc output paths into each result
type. The first shared local layout is:

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
