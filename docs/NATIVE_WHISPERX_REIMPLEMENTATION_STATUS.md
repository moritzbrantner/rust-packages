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
- Hermetic parity fixtures cover WhisperX-style segment words, flat
  `word_segments`, speakers, confidences, and unknown-field preservation.
- Mock external WhisperX command output is compared against imported WhisperX
  JSON so command compatibility and import normalization stay aligned without
  requiring Python in default tests.
- External WhisperX command provider remains available for parity checks and workflows needing Python WhisperX.

### Alignment

- Deterministic CTC primitives exist.
- wav2vec2 bundle/config/tokenizer validation exists.
- wav2vec2 tokenizer discovery accepts `tokenizer.json` and `vocab.json`
  layouts, and CTC blank resolution can use `pad_token_id` when tokenizer
  metadata does not name a pad token.
- Local Candle wav2vec2 CTC emission execution exists for supported
  `Wav2Vec2ForCTC` safetensors bundles.
- wav2vec2 positional convolution supports plain weights, legacy
  `weight_g`/`weight_v` weight norm, and PyTorch parametrization
  `parametrizations.weight.original0/original1` layouts.
- Bundle-backed alignment feeds wav2vec2 emissions through the native CTC
  trellis/backtracking path and returns word timings.
- Native `transcribe(...)` wires `CtcForcedAligner` for native providers when
  `alignment.enabled=true`.
- The full native transcription entrypoint can run VAD, ASR, CTC alignment, and
  diarization assignment in order. Model-backed alignment still depends on a
  supported local wav2vec2 bundle.
- Unsupported wav2vec2 architectures, stable-layer-norm configs, inconsistent
  positional-convolution weight-norm tensors, or other unsupported safetensors
  layouts return typed errors instead of falling back silently.

### Decode

- Default native tests remain hermetic and use direct samples or native WAV
  reads only.
- Native `.wav` paths always use the existing hound/native WAV reader.
- Non-WAV path decode is available only with the explicit `audio-io` feature,
  where the transcription loader calls FFmpeg-backed `audio-analysis-io` decode
  and normalizes/resamples to 16 kHz through the same native boundary.
- `audio.transcription.decodePlan` explains the selected decode path without
  opening files or executing FFmpeg.

### Diarization

- Deterministic native baseline exists.
- Transcript assignment supports majority overlap, nearest start, and strict contained policies.
- Baseline diarization is heuristic and not production speaker recognition or
  pyannote parity.
- Native diarization uses transcript word timings, or segment timings when word
  timings are absent, as speech-span hints before falling back to energy VAD.
- Native transcription diagnostics report diarization provider, runtime, model,
  segment count, speaker count, assignment policy, and heuristic-baseline use.
- `min_speakers` and `max_speakers` are validated and reported in diagnostics,
  but the heuristic baseline does not force pyannote-style speaker cardinality.
- `audio.transcription.diarizationPlan` exposes the current heuristic runtime,
  assignment policies, speaker-bound semantics, and future model-backed
  provider directions without claiming pyannote parity.

### Batch Semantics

- Candle Whisper batch options are validated: `max_batch_size=0` is rejected as
  `invalid_request`.
- Native diagnostics report `chunkCount`, `batchChunks`, `maxBatchSize`,
  `batchCount`, and actual `batchExecution`.
- Batch grouping preserves chunk order and global timing. Execution remains
  deterministic and non-concurrent.

## Missing For WhisperX Parity

- Production-grade diarization.
- Native pyannote replacement.
- Default native container/video decode. Non-WAV native decode is opt-in via
  `audio-io` and still depends on local FFmpeg availability.
- Full performance parity for batched ASR/alignment/diarization behavior.
- Broader wav2vec2 architecture coverage and performance parity with WhisperX.

Projected word timing is approximate and is not WhisperX wav2vec2 alignment
parity. The local wav2vec2 CTC path is closer to WhisperX alignment behavior,
but it is not full WhisperX parity yet.

## Local Smoke Assets

Current machine-local assets used for native Whisper smoke:

- Whisper bundle: `/home/moenarch/.local/share/video-analysis-smoke/whisper-tiny`
- Speech WAV: `/home/moenarch/.local/share/video-analysis-smoke/audio/native-transcription-smoke.wav`
- CUDA 12 shim: `/home/moenarch/.local/share/video-analysis-smoke/cuda12-libs`

`/usr/local/cuda` points at CUDA 13.3 on this host. The passing smoke uses CUDA 12.3 libraries through `RUSTFLAGS`, `LIBRARY_PATH`, and `LD_LIBRARY_PATH`.

Optional external WhisperX parity can be run manually when Python WhisperX and
input media are configured:

```bash
RUN_WHISPERX_PARITY_TESTS=1 \
WHISPERX_COMMAND=whisperx \
WHISPERX_AUDIO_PATH=/path/to/audio.wav \
cargo test -p moritzbrantner-audio-analysis-transcription external_whisperx_parity_when_requested -- --ignored --nocapture
```

Set `WHISPERX_EXPECTED_JSON=/path/to/expected.json` to compare command output
against an imported WhisperX fixture. Set `WHISPERX_DIARIZE=1` only when
`HF_TOKEN` is available.

Optional local native media decode can be run when the crate is built with
`audio-io` and FFmpeg can decode the local input:

```bash
RUN_NATIVE_MEDIA_DECODE_TESTS=1 \
TRANSCRIPTION_MEDIA_PATH=/path/to/video-or-audio-container \
cargo test -p moritzbrantner-audio-analysis-transcription \
  --features audio-io \
  native_media_decode_when_requested -- --ignored --nocapture
```

## Recommended Next Order

1. Expand wav2vec2 architecture and performance coverage beyond the currently
   supported layouts.
2. Exercise opt-in `audio-io` media decode on reviewed local smoke assets.
3. Revisit diarization quality with real model-backed speaker embeddings.
