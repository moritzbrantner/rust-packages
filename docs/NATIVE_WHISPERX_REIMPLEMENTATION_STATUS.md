# Native WhisperX Reimplementation Status

## Baseline

- Current commit: `4450d7af Tighten native transcription runtime validation`
- Native transcription baseline commit: `8876ed7f Add native audio transcription, diarization, and alignment`
- Smoke-test tightening commit: `fd1af803 Add native audio smoke tests and disconnect handling`
- Default tests remain hermetic: no Python, WhisperX, Hugging Face token, network, CUDA, or local model files.

## Goal

Replace WhisperX runtime dependency over time with Rust-native transcription, alignment, and diarization while preserving WhisperX import and command execution as compatibility/parity tooling.

## Implemented

### Native ASR

- Candle Whisper provider behind `candle`.
- CUDA selection behind `cuda`.
- Local bundle validation behind `model-bundles`.
- WAV path input and direct samples.
- Prompt construction validates language, task, decoder, EOS, no-timestamps, and forced decoder IDs.
- Native Whisper attempts timestamp-token segment timing automatically when tokenizer metadata is present, and falls back to chunk/window timing when no bounded timestamp segments are produced.
- Approximate word timing projection from Whisper timestamp-token segments exists.
- Real CUDA smoke passes on the RTX 3060 Ti host with the local CUDA 12 shim.

### WhisperX Compatibility

- Existing WhisperX JSON import is supported through `text-transcripts`.
- External WhisperX command provider remains available for parity checks and workflows needing Python WhisperX.

### Alignment

- Deterministic CTC primitives exist.
- wav2vec2 bundle/config/tokenizer validation exists.
- Local Candle wav2vec2 CTC emission execution exists for supported
  `Wav2Vec2ForCTC` safetensors bundles.
- Bundle-backed alignment feeds wav2vec2 emissions through the native CTC
  trellis/backtracking path and returns word timings.
- Unsupported wav2vec2 architectures or safetensors layouts return typed
  `unsupported_runtime` errors instead of falling back silently.

### Diarization

- Deterministic native baseline exists.
- Transcript assignment supports majority overlap, nearest start, and strict contained policies.
- Baseline diarization is heuristic and not production speaker recognition.

## Missing For WhisperX Parity

- Word-level alignment through the full native pipeline.
- Production-grade diarization.
- Native pyannote replacement.
- Native container/video decode.
- Full batched ASR/alignment/diarization pipeline behavior.
- Broader wav2vec2 layout coverage and performance parity with WhisperX.

Projected word timing is approximate and is not WhisperX wav2vec2 alignment
parity. The local wav2vec2 CTC path is closer to WhisperX alignment behavior,
but it is not full WhisperX parity yet.

## Local Smoke Assets

Current machine-local assets used for native Whisper smoke:

- Whisper bundle: `/home/moenarch/.local/share/video-analysis-smoke/whisper-tiny`
- Speech WAV: `/home/moenarch/.local/share/video-analysis-smoke/audio/native-transcription-smoke.wav`
- CUDA 12 shim: `/home/moenarch/.local/share/video-analysis-smoke/cuda12-libs`

`/usr/local/cuda` points at CUDA 13.3 on this host. The passing smoke uses CUDA 12.3 libraries through `RUSTFLAGS`, `LIBRARY_PATH`, and `LD_LIBRARY_PATH`.

## Recommended Next Order

1. Wire the full native ASR + alignment + diarization pipeline.
2. Upgrade diarization seams without claiming pyannote parity.
3. Add parity tests against imported WhisperX JSON and external WhisperX command output.
4. Expand supported wav2vec2 bundle layouts and performance coverage.
