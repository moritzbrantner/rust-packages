# Runtime Core

Domain-neutral runtime surface DTOs and shared CLI/server adapter helpers for
the workspace package surfaces.

## Runtime surface contracts

`runtime-core` owns the shared package-surface DTOs, structured success
responses, release-schema helpers, and typed `SurfaceError` envelope used by
transport wrappers. Successful operation values preserve crate-specific fields
and add `operation`, `title`, `message`, `summary`, and `result` for generic UI
rendering.

Typed surface errors serialize as JSON with `code`, `message`, `operation`, and
`details`, so CLI, HTTP, WASM, and app adapters can report validation,
unsupported-operation, unsupported-value, and resource-limit failures without
inventing transport-specific error shapes.

Execution planning metadata is additive. Crates can use
`surface_operation_with_execution_plan` to attach `xExecutionPlan` schema
extensions that describe whether an operation is in-memory, a planned job, a
background job, or an external command, plus side effects, cancellation,
progress units, artifact expectations, requirements, and recommended input
limits. This keeps package surfaces runtime-aware without adding required fields
to `SurfaceOperation`.
