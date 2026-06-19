# ADR 0009: Native Alignment Inherits Workflow Device

## Status

Accepted.

## Context

The native transcription workflow can run Whisper ASR on CUDA, but wav2vec2 CTC
alignment previously loaded and executed only on CPU. That made full-workflow
throughput CPU-bound whenever alignment was enabled, even when the ASR phase was
already faster than the WhisperX reference.

Direct lower-level alignment callers may still rely on CPU behavior for local
compatibility and deterministic development environments.

## Decision

Add a `device: NativeDevicePreference` field to `AlignmentOptions`.
`AlignmentOptions::default()` keeps `device = Cpu` so direct lower-level
alignment calls preserve their previous runtime choice.

Workflow callers that own a native execution device may pass `Cuda` or `Auto`.
The CTC alignment provider resolves that preference with the shared native
device resolver, loads wav2vec2 safetensors onto the resolved Candle device, and
creates segment input tensors on the same device. Final CTC emissions remain
copied back into CPU vectors before trellis construction and backtracking.

Alignment responses include `alignmentDevice` and `alignmentCuda` diagnostics so
benchmark gates can prove the active runtime.

## Consequences

Native full-workflow benchmarks can keep alignment enabled while allowing ASR
and alignment to share the same CUDA workflow device.

CPU remains the compatibility default for direct `AlignmentOptions` callers.
If CUDA model execution is still slower than the WhisperX reference, the gate
should stay full-workflow and the next optimization should be batched wav2vec2
emission rather than disabling alignment.
