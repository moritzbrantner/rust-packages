# audio-analysis-speakers

Speaker-domain APIs for `moritzbrantner-video-analysis`.

This crate keeps `moritzbrantner-audio-analysis-recognition` focused on generic embeddings and reference search while adding speaker-specific concepts:

- speaker IDs and labels
- model-versioned speaker embeddings
- speaker profiles and library snapshots
- enrollment and thresholded identification
- baseline energy VAD
- diarization traits and a simple VAD/window/cluster diarizer

`SpectralSpeakerEmbedder` and the native diarization baseline are deterministic
heuristics intended for tests and prototypes. They are not production-grade
speaker verification or diarization. Production systems should use a
model-backed embedder such as ECAPA-TDNN, x-vector, pyannote-style, or
SpeechBrain-compatible speaker verification models.

The first reviewed production direction is an opt-in ONNX speaker embedding
provider, documented in
`docs/adr/0003-native-speaker-diarization-provider.md`. Heuristic diarization
remains the default and no default test requires model files, network access,
Python, pyannote auth, Hugging Face tokens, or CUDA.

Model-backed embedders can implement `SpeakerEmbeddingProvider`, which returns
`SpeakerEmbeddingResponse` with model id, runtime, normalized
`SpeakerEmbedding`, and diagnostics. Existing `SpeakerEmbeddingExtractor`
callers remain supported.

## Feature flags

- `external-tests`: enables ignored local smoke tests that require caller-owned
  WAV fixtures.
- `onnx`: enables `runtime-onnx` execution for caller-owned local ONNX speaker
  embedding models.
- `model-bundles`: enables model-runtime manifest lookup for local ONNX bundles.

Ignored ONNX smoke:

```bash
RUN_NATIVE_SPEAKER_MODEL_TESTS=1 \
SPEAKER_EMBEDDING_MODEL_BUNDLE=/path/to/onnx-speaker-model \
DIARIZATION_AUDIO_PATH=/path/to/meeting.wav \
cargo test -p moritzbrantner-audio-analysis-speakers \
  --features onnx,model-bundles \
  onnx_speaker_embedding_smoke_when_requested -- --ignored --nocapture
```

Set `SPEAKER_EMBEDDING_DIMENSION` when the local model output dimension is not
192. The smoke expects an already-local 16 kHz WAV and never downloads models.

2026-06-10 validation: the default smoke WAV was present, but the default
caller-owned ONNX speaker bundle directory was missing. The ignored smoke was
classified as `setup_error` before ONNX Runtime execution, input/output shape
inspection, or embedding normalization checks.

## Package surface

Primary workflow: `audio.speakers.vad`.

Workflow operations:

- `audio.speakers.embed`: Computes a deterministic spectral speaker embedding from normalized samples.
- `audio.speakers.identify`: Builds a transient enrolled-speaker library and identifies a query embedding.
- `audio.speakers.assignTranscript`: Applies diarization segments to an existing transcription contract.
- `audio.speakers.vad`: Detects speech-like regions with a deterministic RMS voice activity detector.
- `audio.speakers.diarize`: Runs deterministic VAD/window/spectral speaker diarization or normalizes imported diarization segments.

Debug operations:

- `describe`: inspect package metadata and runtime support.

Runtime support: library, CLI, server, and WASM wrappers expose these operations.

Run the primary workflow through the CLI:

```bash
cargo run -p moritzbrantner-audio-analysis-speakers-cli -- run \
  --operation audio.speakers.vad \
  --json '{"channels":1,"frameSize":2,"hopSize":1,"minSilenceSeconds":0.0,"minSpeechSeconds":0.0,"sampleRate":4,"samples":[0.0,0.2,0.2,0.0],"threshold":0.01}'
```

Successful responses use the shared package-surface shape with `operation`,
`title`, `message`, `summary`, and `result`. Default surface calls are
deterministic, local-first, and do not download models, write persistent files,
or execute external tools unless an operation explicitly documents native or
external-tool execution.
