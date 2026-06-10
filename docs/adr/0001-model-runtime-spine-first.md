# Publish the model runtime spine before the full foundation wave

## Status

Accepted.

## Context

The project is preparing its first consumer-facing foundation release for Rust
package consumers. The broader foundation wave includes many domain cores and
workflow surfaces, but waiting for all of them would delay publishing the
runtime contracts needed by downstream package consumers.

The near-term release needs a small publishable base that proves stable runtime
DTOs, job and artifact contracts, model bundle lifecycle behavior, and opt-in
native ONNX execution readiness without moving task inference semantics into the
runtime crates.

## Decision

Publish the Model Runtime Spine before the full foundation wave.

The spine includes:

- `moritzbrantner-runtime-core`
- `moritzbrantner-jobs-core`
- `moritzbrantner-video-analysis-core`
- `moritzbrantner-runtime-onnx`
- `moritzbrantner-model-runtime`

`model-runtime` owns model specs, bundle materialization, manifests, and
diagnostics. It does not own inference semantics. Task crates remain responsible
for preprocessing, decoding, labels, tokenization, image transforms, spans,
boxes, captions, embeddings, and other domain-specific model behavior.

`runtime-onnx` is included only as opt-in low-level runtime readiness. It stays
domain-neutral: session creation, metadata, tensor conversion, and raw execution
belong there; task-specific model semantics do not.

## Consequences

This creates a smaller publishable base now and lets package consumers build on
runtime/job/model-bundle contracts earlier.

The full foundation wave remains a later release step for broader domain cores
and complete workflows.

Native behavior remains opt-in. Default builds stay deterministic, local, and
side-effect free.
