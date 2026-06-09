# audio-analysis-transcription

Rust-native and compatibility transcription orchestration for `video-analysis`.

Native Candle Whisper ASR is available behind the `candle` feature. CUDA device
selection is available behind `cuda`, and local model bundle validation is
available behind `model-bundles`. Native path decoding is WAV-only for now; pass
samples directly or use the external compatibility provider for container or
video inputs.

Native Whisper segment timing is currently chunk/window-level. It does not yet
perform true Whisper timestamp-token decoding, so word- or token-level native
Whisper timestamps remain future work.

The external WhisperX command provider remains compatibility and parity tooling.
It keeps Python-based execution explicit for callers that still need WhisperX
decoding, batched ASR, alignment, or pyannote-backed diarization outside the
default Rust path.

Native deterministic diarization is available behind `diarization`. Pyannote
integration is plan-only in the native path until explicitly implemented.

CTC alignment validates local wav2vec2 bundle files, config, and tokenizer
vocabulary, but real wav2vec2 Candle emissions are not implemented while
`candle-transformers 0.10.2` lacks a wav2vec2 model module. Requests with a
bundle return a typed `unsupported_runtime` at that execution boundary.

Transcript contracts, normalization, caption formatting, and WhisperX JSON
import remain owned by `text-transcripts`.

## Package Operations

- `audio.transcription.transcribe`: run real transcription through the selected
  provider. Native Candle Whisper uses local WAV input or direct samples; the
  external WhisperX provider remains available for compatibility.
- `audio.transcription.importWhisperX`: import existing WhisperX JSON without
  running external tools.
- `audio.transcription.providers`: inspect available provider families.
- `audio.transcription.plan`: describe runtime setup without execution.
- `describe`: inspect package metadata.

## Setup

For native Candle Whisper execution, provide a local model bundle containing:

- `config.json`
- `generation_config.json`
- `tokenizer.json`
- `preprocessor_config.json`
- `model.safetensors`

Use `candle,cuda,model-bundles` for CUDA-backed local smoke tests. No runtime
downloads are performed by this crate.

On the current RTX 3060 Ti smoke host, `/usr/local/cuda` points at CUDA 13.3
while the passing smoke uses a local CUDA 12.3 library shim at
`/home/moenarch/.local/share/video-analysis-smoke/cuda12-libs`. The local smoke
bundle and fixture used there are:

- `/home/moenarch/.local/share/video-analysis-smoke/whisper-tiny`
- `/home/moenarch/.local/share/video-analysis-smoke/audio/native-transcription-smoke.wav`

Install and configure external compatibility tools outside the default test
flow only when using the WhisperX provider:

```bash
whisperx --help
ffmpeg -version
python -c 'import whisperx'
```

WhisperX diarization requires a Hugging Face token accepted by pyannote:

```bash
export HF_TOKEN=...
```

No default build or test downloads models, requires CUDA, or requires network
access.
