# Runtime Contract Standard

This workspace uses a small shared contract standard so crates can incubate in
`rust-packages` and later move to their own repositories without changing the
JSON shape consumed by apps.

## Operation Shape

One operation is one request type plus one result type, returned through the
shared operation envelope:

```text
operationName
  input: OperationRequest
  output: OperationResult
  diagnostics: Diagnostic[]
  artifacts: ArtifactRef[]
  capabilities: RuntimeCapabilities
```

The core Rust function should stay transport-agnostic:

```rust
pub fn analyze(request: AnalyzeRequest) -> OperationResult<AnalyzeResult>
```

Transport wrappers call the same function and serialize the same result shape.
They may add authentication, routing, logging, or cancellation around the call,
but they should not invent a second DTO for the operation result.

## Suggested Crate Layout

Mature and incubating crates should converge on this local convention when they
need stable contracts:

```text
src/contracts.rs
src/operations.rs
contracts/examples/*.json
```

`src/contracts.rs` contains request, result, error, artifact, and capability
DTOs. `src/operations.rs` contains thin pure-Rust functions over those DTOs.
Examples are checked in when they clarify compatibility or frontend behavior.

## Naming Conventions

- `*Contract` names stable, serializable DTOs that cross crate, process, or
  frontend boundaries.
- Borrowed runtime views such as `VideoFrame<'_>`, `AudioFrame<'_>`,
  `TextSegment<'_>`, and `TextDocument<'_>` remain Rust-native library types.
- `*Request` and `*Result` names are operation-specific DTOs and should be
  returned through `OperationResult<T>` when exposed through runtime surfaces.
- `*Report` names UI/report projections. Reports may omit fields from the
  underlying contract, but shared fields should be generated from or tested
  against the owning Rust contract.
- Compatibility shims for renamed or moved contracts should be marked
  deprecated and route into the owning `*Contract` type.

## Generated Clients

Packages that need generated clients should use committed or reproducible output
under:

```text
contracts/generated/json-schema/
contracts/generated/typescript/
contracts/generated/openapi/
```

The `farm-game-engine` repository already has a useful precedent:
`tools/contract_codegen` uses `schemars` and `ts-rs` to generate JSON Schema and
TypeScript declarations into `contracts/generated`. Reuse that idea when a crate
is ready for generated clients, but do not require every crate to adopt it before
the operation DTOs are stable.

## Shared Runtime Crates

- `runtime-contracts`: diagnostics, operation metadata, runtime capabilities,
  and stable string identifiers.
- `runtime-artifacts`: artifact references and minimal memory/local filesystem
  stores.
- `runtime-jobs`: serializable job DTOs and `OperationResult<T>`.

Keep these crates dependency-light. Add schema or TypeScript generation behind
features only when this workspace has a concrete generator for the package.
