# Agent Domain Context

This is a single-context repository.

Agents should read these files before planning domain-sensitive work:

- `AGENTS.md`
- `CONTEXT.md`
- `docs/adr/*.md`

The repository is a Rust-first multimodal analysis workspace for video, audio,
image, text, vector, data, math, animation, 3D, and ComfyUI interoperability.
Reusable Rust crates live under `crates/`, the root facade crate exports from
`src/lib.rs`, package surfaces live under `packages/`, and runnable prototypes
live under `prototypes/`.

Core crates should stay composable and free of prototype or runtime-specific
concerns. Package and prototype surfaces may exercise crate APIs, but they
should not define core crate architecture.
