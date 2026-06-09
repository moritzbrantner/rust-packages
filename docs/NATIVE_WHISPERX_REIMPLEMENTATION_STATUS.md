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
- Private Whisper timestamp-token parsing can map model token output into segment-level timings; the default provider path still uses chunk/window timing.
- Real CUDA smoke passes on the RTX 3060 Ti host with the local CUDA 12 shim.

### WhisperX Compatibility

- Existing WhisperX JSON import is supported through `text-transcripts`.
- External WhisperX command provider remains available for parity checks and workflows needing Python WhisperX.

### Alignment

- Deterministic CTC primitives exist.
- wav2vec2 bundle/config/tokenizer validation exists.
- Real wav2vec2 emissions are not implemented yet and return typed `unsupported_runtime`.

### Diarization

- Deterministic native baseline exists.
- Transcript assignment supports majority overlap, nearest start, and strict contained policies.
- Baseline diarization is heuristic and not production speaker recognition.

## Missing For WhisperX Parity

- Public/default Whisper timestamp-token decoding and word timing projection.
- Real wav2vec2 CTC emission model execution.
- Word-level alignment through the full native pipeline.
- Production-grade diarization.
- Native pyannote replacement.
- Native container/video decode.
- Full batched ASR/alignment/diarization pipeline behavior.

## Local Smoke Assets

Current machine-local assets used for native Whisper smoke:

- Whisper bundle: `/home/moenarch/.local/share/video-analysis-smoke/whisper-tiny`
- Speech WAV: `/home/moenarch/.local/share/video-analysis-smoke/audio/native-transcription-smoke.wav`
- CUDA 12 shim: `/home/moenarch/.local/share/video-analysis-smoke/cuda12-libs`

`/usr/local/cuda` points at CUDA 13.3 on this host. The passing smoke uses CUDA 12.3 libraries through `RUSTFLAGS`, `LIBRARY_PATH`, and `LD_LIBRARY_PATH`.

## Recommended Next Order

1. Enable Whisper timestamp-token decoding in the native provider path when model output proves stable.
2. Add native word timing projection from timestamp tokens.
3. Implement or unblock wav2vec2 Candle emissions.
4. Upgrade diarization beyond the deterministic spectral baseline.
5. Add parity tests against imported WhisperX JSON and external WhisperX command output.
