# audio-analysis-transcription

Rust-native and compatibility transcription orchestration for `video-analysis`.

Native Candle Whisper ASR is available behind the `candle` feature. CUDA device
selection is available behind `cuda`, and local model bundle validation is
available behind `model-bundles`. Native path decoding is WAV-only for now; pass
samples directly or use the external compatibility provider for container or
video inputs.

Native Whisper tries model timestamp tokens automatically when the tokenizer
defines Whisper timestamp metadata. If timestamp decoding does not produce
bounded text segments, it falls back to chunk/window segment timing. Native
Whisper segment times are emitted as global transcript times. Timestamp-token
segments also include approximate projected word timings based on
character-weighted distribution inside each segment. wav2vec2/CTC alignment
is the authoritative native word timing path when a supported local alignment
bundle is provided.

The external WhisperX command provider remains compatibility and parity tooling.
It keeps Python-based execution explicit for callers that still need WhisperX
decoding, batched ASR, alignment, or pyannote-backed diarization outside the
default Rust path.

Native deterministic diarization is available behind `diarization`. Pyannote
integration is plan-only in the native path until explicitly implemented.

CTC alignment validates local wav2vec2 bundle files, config, tokenizer
vocabulary, and preprocessor metadata. Supported local `Wav2Vec2ForCTC`
`model.safetensors` bundles execute through a private Candle implementation and
feed native CTC trellis/backtracking. Unsupported architectures or safetensors
layouts return typed `unsupported_runtime` errors instead of falling back to
deterministic timing.

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

For native wav2vec2 CTC alignment, provide a local `Wav2Vec2ForCTC` bundle
containing:

- `config.json`
- `tokenizer.json`
- `preprocessor_config.json`
- `model.safetensors`

Use `candle,model-bundles` for CPU local smoke tests and add `cuda` for
CUDA-backed Whisper smoke tests. No runtime downloads are performed by this
crate.

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
