# audio-analysis-transcription

Rust-native and compatibility transcription orchestration for `video-analysis`.

Native Candle Whisper ASR is available behind the `candle` feature. CUDA device
selection is available behind `cuda`, and local model bundle validation is
available behind `model-bundles`. Native path decoding defaults to direct
samples or readable WAV files. With the explicit `audio-io` feature, non-WAV
paths use FFmpeg-backed `audio-analysis-io` decode and then normalize/resample
through the same native 16 kHz mono boundary. This opt-in media decode is not
WhisperX parity and is not part of default tests.

Native Whisper tries model timestamp tokens automatically when the tokenizer
defines Whisper timestamp metadata. If timestamp decoding does not produce
bounded text segments, it falls back to chunk/window segment timing. Native
Whisper segment times are emitted as global transcript times. Timestamp-token
segments also include approximate projected word timings based on
character-weighted distribution inside each segment. wav2vec2/CTC alignment
can run through the same native transcription pipeline: `alignment.enabled=true`
with no bundle uses deterministic transcript timing alignment, while
`alignment.enabled=true` with a supported local wav2vec2 bundle uses Candle
wav2vec2 CTC alignment.

The external WhisperX command provider remains compatibility and parity tooling.
It keeps Python-based execution explicit for callers that still need WhisperX
decoding, batched ASR, alignment, or pyannote-backed diarization outside the
default Rust path. It is also the current path for video/container inputs.

Native deterministic diarization is available behind `diarization` as a
heuristic spectral baseline, not a pyannote replacement or production speaker
recognition model. Diarization assignment runs after alignment when both
options are enabled, so the native diarization seam can use aligned word
timings, or segment timings when word timings are absent, as speech-span hints.
When no transcript timing is available it falls back to the energy-VAD baseline.
`min_speakers` and `max_speakers` are validated and reported in diagnostics;
they are not pyannote-style clustering constraints in the native baseline.

CTC alignment validates local wav2vec2 bundle files, config, tokenizer
vocabulary, and preprocessor metadata. Tokenizer discovery accepts
`tokenizer.json` and `vocab.json`; CTC blank resolution can use `pad_token_id`
when tokenizer metadata does not name a pad token. Supported local
`Wav2Vec2ForCTC` `model.safetensors` bundles execute through a private Candle
implementation and feed native CTC trellis/backtracking. Unsupported
architectures, stable-layer-norm configs, inconsistent positional-convolution
weight-norm tensors, or other unsupported safetensors layouts return typed
errors instead of falling back to deterministic timing.

Candle Whisper batch options are deterministic rather than concurrent in this
phase. `max_batch_size=0` is rejected, chunk order and global timing are
preserved, and diagnostics report `chunkCount`, `batchChunks`, `maxBatchSize`,
`batchCount`, and `batchExecution`.

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
- `audio.transcription.modelPlan`: inspect ASR model requirements.
- `audio.transcription.vadPlan`: inspect deterministic VAD defaults.
- `audio.transcription.alignmentPlan`: inspect CTC alignment requirements.
- `audio.transcription.decodePlan`: explain source decode routing without
  opening files or running FFmpeg.
- `audio.transcription.diarizationPlan`: explain heuristic diarization status,
  assignment policies, and future model-backed provider directions.
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
- `tokenizer.json` or `vocab.json`
- `preprocessor_config.json`
- `model.safetensors`

Use `candle,model-bundles` for CPU local smoke tests and add `cuda` for
CUDA-backed Whisper smoke tests. No runtime downloads are performed by this
crate.

Use `audio-io` only when native non-WAV media/container decode is explicitly
needed and local FFmpeg support is available:

```bash
RUN_NATIVE_MEDIA_DECODE_TESTS=1 \
TRANSCRIPTION_MEDIA_PATH=/path/to/video-or-audio-container \
cargo test -p moritzbrantner-audio-analysis-transcription \
  --features audio-io \
  native_media_decode_when_requested -- --ignored --nocapture
```

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
access. Default tests also do not require Python, WhisperX, Hugging Face tokens,
external model files, or FFmpeg.

Optional external WhisperX parity can be run manually when local tools and media
are configured:

```bash
RUN_WHISPERX_PARITY_TESTS=1 \
WHISPERX_COMMAND=whisperx \
WHISPERX_AUDIO_PATH=/path/to/audio.wav \
cargo test -p moritzbrantner-audio-analysis-transcription external_whisperx_parity_when_requested -- --ignored --nocapture
```

Set `WHISPERX_EXPECTED_JSON=/path/to/expected.json` to compare command output
against a known WhisperX JSON fixture. Set `WHISPERX_DIARIZE=1` only when
`HF_TOKEN` is available.
