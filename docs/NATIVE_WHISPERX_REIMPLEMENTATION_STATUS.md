# Native WhisperX Reimplementation Status

## Current Snapshot

- Current repo commit when this status was refreshed: `de1e8b8f`
- Last native-transcription validation commit: `5ea7f627`
- Native transcription baseline commit:
  `8876ed7f Add native audio transcription, diarization, and alignment`
- Smoke-test tightening commit:
  `fd1af803 Add native audio smoke tests and disconnect handling`
- Latest local validation date: `2026-06-11`
- Default tests remain hermetic: no Python, WhisperX, Hugging Face token,
  network, CUDA, or local model files.

Native ASR, wav2vec2 alignment, ONNX speaker embedding, transcription ONNX
diarization, external WhisperX non-diarization parity, and token-gated
WhisperX diarization parity have all passed in the documented local setup.
Full native replacement is still not complete: production diarization quality,
a native pyannote replacement, default native container/video decode, broader
model coverage, and throughput parity remain open.

## Goal

Replace WhisperX runtime dependency over time with Rust-native transcription,
alignment, and diarization while preserving WhisperX import and command
execution as compatibility/parity tooling until native quality and throughput
are sufficient.

## Implemented

### Native ASR

- Candle Whisper provider behind `candle`.
- CUDA selection behind `cuda`.
- Local bundle validation behind `model-bundles`.
- WAV path input and direct samples.
- Prompt construction validates language, task, decoder, EOS, no-timestamps,
  and forced decoder IDs.
- Native Whisper attempts timestamp-token segment timing automatically when
  tokenizer metadata is present, and falls back to chunk/window timing when no
  bounded timestamp segments are produced.
- Approximate word timing projection from Whisper timestamp-token segments
  exists.
- Real CUDA smoke passes on the RTX 3060 Ti host with the local CUDA 12 shim.

### WhisperX Compatibility

- Existing WhisperX JSON import is supported through `text-transcripts`.
- Hermetic parity fixtures cover WhisperX-style segment words, flat
  `word_segments`, speakers, confidences, and unknown-field preservation.
- Mock external WhisperX command output is compared against imported WhisperX
  JSON so command compatibility and import normalization stay aligned without
  requiring Python in default tests.
- External WhisperX command provider remains available for parity checks and
  workflows needing Python WhisperX.

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
- The full native transcription entrypoint can run VAD, ASR, CTC alignment,
  and diarization assignment in order. Model-backed alignment still depends on
  a supported local wav2vec2 bundle.
- Unsupported wav2vec2 architectures, stable-layer-norm configs, inconsistent
  positional-convolution weight-norm tensors, or other unsupported safetensors
  layouts return typed errors instead of falling back silently.
- Internal wav2vec2 bundle layout inspection reports architecture,
  stable-layer-norm status, positional-convolution layout,
  feature-extractor norm, encoder layer count, missing tensor keys, and
  unsupported reasons.
- `audio.transcription.alignmentBundlePlan` exposes that readiness check as a
  Debug-only non-executing operation. It can return a static plan without local
  files, and invalid bundle paths are `setup_error`.

### Decode

- Default native tests remain hermetic and use direct samples or native WAV
  reads only.
- Native `.wav` paths always use the existing hound/native WAV reader.
- Non-WAV path decode is available only with the explicit `audio-io` feature,
  where the transcription loader calls FFmpeg-backed `audio-analysis-io`
  decode and normalizes/resamples to 16 kHz through the same native boundary.
- `audio.transcription.decodePlan` explains the selected decode path without
  opening files or executing FFmpeg.
- Executing decode paths have internal diagnostics for decode route, source
  extension, input sample rate when available, and normalized mono 16 kHz
  output shape.

### Diarization

- Deterministic native baseline exists.
- Transcript assignment supports majority overlap, nearest start, and strict
  contained policies.
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
- ADR 0003 is accepted. The first production model-backed diarization target
  is an opt-in ONNX speaker embedding provider.
- `audio-analysis-speakers` exposes `SpeakerEmbeddingProvider`,
  `SpeakerEmbeddingRequest`, and `SpeakerEmbeddingResponse` without changing
  existing `SpeakerEmbeddingExtractor` users.
- `OnnxSpeakerEmbedder` validates direct `.onnx` files, directories containing
  `model.onnx`, or `model-runtime` bundle manifests when `model-bundles` is
  enabled. It supports waveform inputs `[B,S]` and `[B,1,S]`, and feature
  inputs such as `[B,T,80]` through deterministic CPU fbank/log-mel
  preprocessing.
- Native transcription can construct
  `WindowedSpeakerDiarizer<OnnxSpeakerEmbedder, EnergyVoiceActivityDetector>`
  only when an explicit `speakerEmbeddingModelBundle` is supplied. Heuristic
  diarization remains the default.

### Batch Semantics

- Candle Whisper batch options are validated: `max_batch_size=0` is rejected as
  `invalid_request`.
- Native diagnostics report `chunkCount`, `batchChunks`, `maxBatchSize`,
  `batchCount`, and actual
  `batchExecution=candle-whisper-sequential`.
- Batch grouping preserves chunk order and global timing. Execution remains
  deterministic and non-concurrent; this is not throughput parity or
  tensor-batched model execution.

## Validated Local Parity

| Area | Status | Notes |
| --- | --- | --- |
| Candle Whisper native ASR | Passed | Local smoke bundle; CUDA shim noted in assets. |
| wav2vec2 CTC alignment | Passed | `facebook/wav2vec2-base-960h`; `vocab.json`; Candle execution. |
| Media/container decode | Passed | `audio-io`; checked-in WebM fixture; FFmpeg local. |
| ONNX speaker embedding | Passed with explicit ORT 1.26.0 | Feature-input WeSpeaker model; fbank/log-mel CPU preprocessing. |
| Transcription ONNX diarization | Passed with explicit ORT 1.26.0 | Direct samples and mock ASR timing; non-empty speaker assignments. |
| WhisperX non-diarization parity | Passed | `.audio-tools/whisperx-venv/bin/whisperx`, `tiny.en`, CPU, int8. |
| WhisperX token-gated diarization parity | Passed | `WHISPERX_DIARIZE=1`; pyannote/Hugging Face access valid on 2026-06-11. |

No token value was logged or documented.

## Remaining Gaps

- Production-grade native diarization quality and speaker-recognition
  behavior.
- Native pyannote replacement.
- Default native container/video decode. Non-WAV native decode is opt-in via
  `audio-io` and still depends on local FFmpeg availability.
- Full performance parity:
  - ASR is sequential Candle Whisper execution.
  - wav2vec2 alignment is per segment.
  - ONNX diarization is window-by-window.
- Broader wav2vec2 architecture/layout coverage.
- Local ONNX Runtime dylib setup is still host-sensitive unless
  `ORT_DYLIB_PATH` points at the compatible local ONNX Runtime 1.26.0 dylib.

Projected word timing is approximate and is not WhisperX wav2vec2 alignment
parity. The local wav2vec2 CTC path is closer to WhisperX alignment behavior,
but it is not full WhisperX parity yet.

## Local Smoke Assets

Current machine-local assets used for native WhisperX replacement smokes:

- Whisper bundle: `$SMOKE_ROOT/whisper-tiny`
- wav2vec2 bundle: `$SMOKE_ROOT/models/wav2vec2-base-960h/main`
- ONNX speaker bundle:
  `$SMOKE_ROOT/models/wespeaker-voxceleb-resnet34-LM/main`
- Speech WAV: `$SMOKE_ROOT/audio/native-transcription-smoke.wav`
- CUDA 12 shim: `$SMOKE_ROOT/cuda12-libs`
- WhisperX venv: `.audio-tools/whisperx-venv`
- ONNX Runtime 1.26.0 dylib:
  `.audio-tools/whisperx-venv/lib/python3.11/site-packages/onnxruntime/capi/libonnxruntime.so.1.26.0`

`/usr/local/cuda` points at CUDA 13.3 on this host. The passing CUDA smoke uses
CUDA 12.3 libraries through `RUSTFLAGS`, `LIBRARY_PATH`, and
`LD_LIBRARY_PATH`.

## Repro Commands

Baseline hermetic matrix:

```bash
cargo fmt --check
cargo test -p moritzbrantner-audio-analysis-speakers
cargo test -p moritzbrantner-audio-analysis-speakers --features onnx,model-bundles
cargo test -p moritzbrantner-audio-analysis-transcription
cargo test -p moritzbrantner-audio-analysis-transcription --features alignment,candle,model-bundles
cargo test -p moritzbrantner-audio-analysis-transcription --features audio-io
cargo test -p moritzbrantner-audio-analysis-transcription --features diarization,onnx,model-bundles
cargo test --test audio_surface_audit
cargo test --test audio_transcription_native_contracts
cargo test --test audio_voice_pipeline
bun run audio-app:typecheck
```

wav2vec2 alignment smoke:

```bash
ALIGNMENT_TRANSCRIPT_TEXT="hello world" \
cargo test -p moritzbrantner-audio-analysis-transcription \
  --features candle,alignment,model-bundles \
  ctc_alignment_wav2vec2_smoke_when_requested -- --ignored --nocapture
```

The ignored smoke runs with defaults when environment variables are omitted. It
uses `ALIGNMENT_AUDIO_PATH` when set, then `TRANSCRIPTION_AUDIO_PATH`, the
checked-in whisper.cpp sample WAV, or a generated 16 kHz WAV. It uses
`ALIGNMENT_MODEL_BUNDLE` when set, otherwise `ALIGNMENT_MODEL_DIR`,
`$XDG_DATA_HOME/video-analysis-smoke/models`,
`$HOME/.local/share/video-analysis-smoke/models`, or a generated tiny wav2vec2
bundle. Set `RUN_NATIVE_ALIGNMENT_TESTS=0` to skip it explicitly when running
ignored tests.

Media/container decode smoke:

```bash
RUN_NATIVE_MEDIA_DECODE_TESTS=1 \
TRANSCRIPTION_MEDIA_PATH="tests/fixtures/me-at-the-zoo-jNQXAC9IVRw.webm" \
cargo test -p moritzbrantner-audio-analysis-transcription \
  --features audio-io \
  native_media_decode_when_requested -- --ignored --nocapture
```

ONNX speaker embedding smoke:

```bash
RUN_NATIVE_SPEAKER_MODEL_TESTS=1 \
ORT_DYLIB_PATH="$PWD/.audio-tools/whisperx-venv/lib/python3.11/site-packages/onnxruntime/capi/libonnxruntime.so.1.26.0" \
SPEAKER_EMBEDDING_MODEL_BUNDLE="$SMOKE_ROOT/models/wespeaker-voxceleb-resnet34-LM/main" \
SPEAKER_EMBEDDING_MODEL_FILE="speaker-embedding.onnx" \
DIARIZATION_AUDIO_PATH="$SMOKE_ROOT/audio/native-transcription-smoke.wav" \
SPEAKER_EMBEDDING_DIMENSION=256 \
cargo test -p moritzbrantner-audio-analysis-speakers \
  --features onnx,model-bundles \
  onnx_speaker_embedding_smoke_when_requested -- --ignored --nocapture
```

Transcription ONNX diarization smoke:

```bash
RUN_NATIVE_SPEAKER_MODEL_TESTS=1 \
ORT_DYLIB_PATH="$PWD/.audio-tools/whisperx-venv/lib/python3.11/site-packages/onnxruntime/capi/libonnxruntime.so.1.26.0" \
SPEAKER_EMBEDDING_MODEL_BUNDLE="$SMOKE_ROOT/models/wespeaker-voxceleb-resnet34-LM/main" \
SPEAKER_EMBEDDING_MODEL_FILE="speaker-embedding.onnx" \
DIARIZATION_AUDIO_PATH="$SMOKE_ROOT/audio/native-transcription-smoke.wav" \
SPEAKER_EMBEDDING_DIMENSION=256 \
cargo test -p moritzbrantner-audio-analysis-transcription \
  --features diarization,onnx,model-bundles \
  native_onnx_diarization_smoke_when_requested -- --ignored --nocapture
```

WhisperX non-diarization parity smoke:

```bash
RUN_WHISPERX_PARITY_TESTS=1 \
WHISPERX_COMMAND="$PWD/.audio-tools/whisperx-venv/bin/whisperx" \
WHISPERX_MODEL="tiny.en" \
WHISPERX_LANGUAGE="en" \
WHISPERX_DEVICE="cpu" \
WHISPERX_COMPUTE_TYPE="int8" \
WHISPERX_AUDIO_PATH="$SMOKE_ROOT/audio/native-transcription-smoke.wav" \
cargo test --test audio_transcription_native_contracts \
  external_whisperx_parity_when_requested -- --ignored --nocapture
```

WhisperX token-gated diarization parity smoke:

```bash
test -n "$HF_TOKEN"

RUN_WHISPERX_PARITY_TESTS=1 \
WHISPERX_COMMAND="$PWD/.audio-tools/whisperx-venv/bin/whisperx" \
WHISPERX_MODEL="tiny.en" \
WHISPERX_LANGUAGE="en" \
WHISPERX_DEVICE="cpu" \
WHISPERX_COMPUTE_TYPE="int8" \
WHISPERX_DIARIZE=1 \
WHISPERX_AUDIO_PATH="$SMOKE_ROOT/audio/native-transcription-smoke.wav" \
HF_TOKEN="$HF_TOKEN" \
cargo test --test audio_transcription_native_contracts \
  external_whisperx_parity_when_requested -- --ignored --nocapture
```

## Validation History

- `2026-06-10`: baseline matrix passed. The only known warning was unrelated
  dead code in `text-model-runtime`.
- `2026-06-10`: wav2vec2 smoke passed after positional-convolution weight-norm
  handling supported the observed `facebook/wav2vec2-base-960h` layout.
- `2026-06-10`: media decode smoke passed after the ignored smoke harness
  resolved workspace-root-relative fixture paths.
- `2026-06-10`: ONNX speaker and transcription diarization smokes passed with
  explicit ONNX Runtime 1.26.0. Implicit ONNX Runtime selection timed out at
  `onnxSessionBuilder=begin`.
- `2026-06-10`: WhisperX non-diarization parity passed with the ignored local
  `.audio-tools/whisperx-venv`.
- `2026-06-10`: token-gated WhisperX diarization initially failed due Hugging
  Face gated repo/fine-grained-token permissions. Historical non-secret error
  categories: `GatedRepoError: 403` and
  `HfHubHTTPError: 403 Forbidden`.
- `2026-06-11`: token-gated WhisperX diarization parity passed after token
  permissions were updated.

## Recommended Next Order

1. Decide and implement the local ONNX Runtime dylib policy:
   - require explicit `ORT_DYLIB_PATH` for ignored ONNX smokes, or
   - add repo tooling that discovers the compatible ignored ONNX Runtime dylib.
2. Start native diarization quality/parity work:
   - compare native ONNX diarization output against the passed WhisperX
     diarization parity run,
   - identify speaker-count, boundary, assignment, and timing deltas,
   - add hermetic fixtures for any normalized output shapes discovered.
3. Continue performance parity separately:
   - Candle Whisper batching,
   - wav2vec2 segment batching,
   - ONNX diarization window batching.
4. Broaden model/layout support only after the above:
   - additional wav2vec2 layouts,
   - additional speaker embedding model input/output shapes,
   - default/native container decode decisions.
