# ADR 0003: Native Speaker Diarization Provider

## Status

Accepted.

## Context

Native transcription currently has a deterministic heuristic diarization
baseline. It is useful for hermetic tests and prototypes, but it is not
production pyannote parity and it is not a native pyannote replacement.

Default tests must remain hermetic: no Python, pyannote auth, Hugging Face
tokens, network access, CUDA, or checked-in model files.

## Decision

The first model-backed diarization provider target is an ONNX speaker embedding
adapter in `audio-analysis-speakers`.

The crate will keep heuristic diarization as the default. Model-backed
diarization must be opt-in and must validate a caller-owned local bundle before
execution.

The provider abstraction should be:

- `SpeakerEmbeddingProvider`
- `SpeakerEmbeddingRequest`
- `SpeakerEmbeddingResponse`

The adapter output type remains the existing `SpeakerEmbedding`. Runtime should
prefer the workspace ONNX runtime infrastructure rather than adding a separate
runtime stack.

Local bundle validation should mirror wav2vec2 bundle validation:

- required model file
- expected input and output names, or explicit configurable mapping
- embedding dimensionality
- typed `setup_error`, `invalid_request`, `model_output_mismatch`, and
  `unsupported_runtime` failures

Tests should include hermetic shape validation with mock embeddings and an
ignored local ONNX speaker model smoke. The ignored smoke may require a local
caller-owned model and local WAV fixture, but default tests must not.

## Out Of Scope

- Hugging Face downloads.
- Pyannote authentication.
- Network access.
- Default model files.
- Replacing the heuristic default.
- Implementing production diarization before this design gate is reviewed.

## Consequences

`audio.transcription.diarizationPlan` continues to report
`currentRuntime=heuristic-native` and should list ONNX speaker embeddings as a
future opt-in provider. Production diarization work can proceed after review by
adding an ONNX adapter, feeding embeddings into `WindowedSpeakerDiarizer`, and
emitting diagnostics such as `diarizationRuntime=onnx`,
`speakerEmbeddingProvider=onnx`, `speakerEmbeddingDimension=N`, and
`diarizationBaseline=false`.

## Accepted Constraints

- First production target: ONNX speaker embedding provider.
- Default diarization remains the deterministic heuristic baseline.
- ONNX speaker embeddings are opt-in.
- No downloads, Hugging Face auth, pyannote auth, network, CUDA, Python, or
  default model files are introduced.
- ONNX execution uses `runtime-onnx`; `audio-analysis-speakers` must not depend
  directly on ONNX Runtime crates.
- Bundle validation can use `model-runtime` only behind an explicit feature.
