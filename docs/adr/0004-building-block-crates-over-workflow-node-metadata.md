# Building Block Crates over Workflow Node Metadata

## Status

Accepted.

## Context

This repository exists to provide reusable multimodal building blocks: Rust
library APIs plus audited CLI, REST, WASM, and web app adapter surfaces over the
same library-owned contracts.

External package consumers may compose these capabilities into workflow graphs,
but graph authoring, node layout, connection semantics, and graph execution are
separate concerns. ComfyUI remains an interoperability target and useful
composition reference, not the internal architecture for the Rust crates.

## Decision

Keep crate capabilities as composable building blocks rather than workflow-node
definitions.

External workflow graph projects own node, edge, port, layout, and execution
metadata. This repository will not add explicit node or port metadata to
`runtime-core::SurfaceOperation`.

The enforcement mechanism is shared contract ownership, adapter parity, and
tests:

- the crate owning the most general semantic form owns the stable contract;
- specialized crates may enrich contracts, but must preserve conversion paths
  back to the general contract;
- CLI, REST, WASM, and web app adapters delegate to `package_surface()` and
  `run_surface_operation()` from the library crate;
- default package-surface operations remain deterministic and side-effect free
  unless explicitly documented otherwise.

## Consequences

Workflow graph tools must map package operations to their own node models.

The trade-off is deliberate: crate APIs stay cleaner, reusable outside graph
contexts, and testable through normal Rust contracts and adapter parity instead
of workflow-aware metadata.
