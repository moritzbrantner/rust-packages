# Native WhisperX Reimplementation Status

## Baseline

- Current commit: `4450d7af Tighten native transcription runtime validation`
- Native transcription baseline commit: `8876ed7f Add native audio transcription, diarization, and alignment`
- Smoke-test tightening commit: `fd1af803 Add native audio smoke tests and disconnect handling`
- Default tests remain hermetic: no Python, WhisperX, Hugging Face token, network, CUDA, or local model files.
- 2026-06-10 Phase 0 verification on this worktree passed:
  `cargo fmt --check`, speaker default and `onnx,model-bundles` tests,
  transcription default, `alignment,candle,model-bundles`, `audio-io`, and
  `diarization,onnx,model-bundles` tests,
  `cargo test --test audio_surface_audit`,
  `cargo test --test audio_transcription_native_contracts`,
  `cargo test --test audio_voice_pipeline`, and
  `bun run audio-app:typecheck`. Integration tests emitted an unrelated
  pre-existing `text-model-runtime` dead-code warning.

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
- Internal wav2vec2 bundle layout inspection reports architecture,
  stable-layer-norm status, positional-convolution layout, feature-extractor
  norm, encoder layer count, missing tensor keys, and unsupported reasons.
- `audio.transcription.alignmentBundlePlan` exposes that readiness check as a
  Debug-only non-executing operation. It can return a static plan without local
  files, and invalid bundle paths are `setup_error`.

### Decode

- Default native tests remain hermetic and use direct samples or native WAV
  reads only.
- Native `.wav` paths always use the existing hound/native WAV reader.
- Non-WAV path decode is available only with the explicit `audio-io` feature,
  where the transcription loader calls FFmpeg-backed `audio-analysis-io` decode
  and normalizes/resamples to 16 kHz through the same native boundary.
- `audio.transcription.decodePlan` explains the selected decode path without
  opening files or executing FFmpeg.
- Executing decode paths have internal diagnostics for decode route, source
  extension, input sample rate when available, and normalized mono 16 kHz
  output shape.

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
- ADR 0003 is accepted. The first production model-backed diarization target is
  an opt-in ONNX speaker embedding provider.
- `audio-analysis-speakers` exposes `SpeakerEmbeddingProvider`,
  `SpeakerEmbeddingRequest`, and `SpeakerEmbeddingResponse` without changing
  existing `SpeakerEmbeddingExtractor` users.
- An opt-in `OnnxSpeakerEmbedder` validates direct `.onnx` files, directories
  containing `model.onnx`, or `model-runtime` bundle manifests when
  `model-bundles` is enabled. It supports one f32 waveform input and one f32
  embedding output for the first tranche.
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
- wav2vec2 bundle: `/home/moenarch/.local/share/video-analysis-smoke/models/wav2vec2-base-960h/main`
- ONNX speaker bundle: `/home/moenarch/.local/share/video-analysis-smoke/models/wespeaker-voxceleb-resnet34-LM/main`
- Speech WAV: `/home/moenarch/.local/share/video-analysis-smoke/audio/native-transcription-smoke.wav`
- CUDA 12 shim: `/home/moenarch/.local/share/video-analysis-smoke/cuda12-libs`
- WhisperX venv: `.audio-tools/whisperx-venv`

`/usr/local/cuda` points at CUDA 13.3 on this host. The passing smoke uses CUDA 12.3 libraries through `RUSTFLAGS`, `LIBRARY_PATH`, and `LD_LIBRARY_PATH`.

## 2026-06-10 Native Validation Tranche

Phase 1 inventory used the default smoke-root layout by environment variable,
not checked-in model files. The speech WAV and checked-in WebM fixture were
present. `ffmpeg` was available. Python could import `whisperx`, but the
global `whisperx` console entry point failed during import. `HF_TOKEN` and
`MODEL_HF_TOKEN` were not present in the shell, so no token-gated pyannote
parity was run.

Baseline guard before edits:

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

Result: pass. The only noted warning was unrelated dead code in
`text-model-runtime`.

The same required matrix passed before the 2026-06-10 setup-unblocking edits
and again after the implementation. The only noted warning remained unrelated
dead code in `text-model-runtime`.

Model bundle tooling update:

- `scripts/model_bundles.lock.sh` now includes `wav2vec2-base-960h` and
  `wespeaker-voxceleb-resnet34-LM` specs with checksums for the local smoke
  bundles.
- The current `hbredin/wespeaker-voxceleb-resnet34-LM` main branch uses
  `speaker-embedding.onnx`; the previously documented
  `voxceleb_resnet34_LM.onnx` filename was not present on main and returned
  404.
- `vanalyze models download` accepts `--preset wav2vec2-base-960h`, custom
  `--task audio-embedding`, `--task speaker-diarization`, and a custom bundle
  `--name` so lock-file custom specs materialize under stable local names.

wav2vec2 alignment smoke:

```bash
RUN_NATIVE_ALIGNMENT_TESTS=1 \
ALIGNMENT_MODEL_BUNDLE="$SMOKE_ROOT/models/wav2vec2-base-960h/main" \
TRANSCRIPTION_AUDIO_PATH="$SMOKE_ROOT/audio/native-transcription-smoke.wav" \
ALIGNMENT_TRANSCRIPT_TEXT="hello world" \
cargo test -p moritzbrantner-audio-analysis-transcription \
  --features candle,alignment,model-bundles \
  ctc_alignment_wav2vec2_smoke_when_requested -- --ignored --nocapture
```

Result after setup-unblocking edits: pass. The bundle resolved through the
model-runtime manifest and used `vocab.json` as the tokenizer file. The smoke
reported `architecture="Wav2Vec2ForCTC"`,
`do_stable_layer_norm=false`, `positional_conv_layout="weight-norm"`,
`feature_extractor_norm="group"`, `encoder_layer_count=12`, no missing
required keys, and no unsupported layout reasons. Inference started and
completed through `alignmentModelExecution=candle-wav2vec2`.

Observed implementation fix: `facebook/wav2vec2-base-960h` stores positional
convolution `weight_g` per kernel position (`128` values), not per output
channel (`768` values). Native reconstruction now supports both per-output and
per-kernel weight-norm layouts.

Media decode smoke:

```bash
RUN_NATIVE_MEDIA_DECODE_TESTS=1 \
TRANSCRIPTION_MEDIA_PATH="tests/fixtures/me-at-the-zoo-jNQXAC9IVRw.webm" \
cargo test -p moritzbrantner-audio-analysis-transcription \
  --features audio-io \
  native_media_decode_when_requested -- --ignored --nocapture
```

Initial result: fail, `setup_error`-class harness issue. The documented
workspace-root-relative fixture path was interpreted from the crate test
process and FFprobe reported `No such file or directory`.

Hardening: the ignored smoke harness now resolves missing relative media paths
against the workspace root. It does not change runtime decode behavior,
feature defaults, `audio.transcription.decodePlan`, or FFmpeg requirements.

Rerun result: pass. The smoke asserted
`decode_route=audio-io-media-decode`, mono output, 16 kHz output, non-empty
finite samples, preserved resolved source path, and available input sample-rate
diagnostics.

ONNX speaker embedding smoke:

```bash
RUN_NATIVE_SPEAKER_MODEL_TESTS=1 \
SPEAKER_EMBEDDING_MODEL_BUNDLE="$SMOKE_ROOT/models/wespeaker-voxceleb-resnet34-LM/main" \
SPEAKER_EMBEDDING_MODEL_FILE="speaker-embedding.onnx" \
DIARIZATION_AUDIO_PATH="$SMOKE_ROOT/audio/native-transcription-smoke.wav" \
SPEAKER_EMBEDDING_DIMENSION=256 \
cargo test -p moritzbrantner-audio-analysis-speakers \
  --features onnx,model-bundles \
  onnx_speaker_embedding_smoke_when_requested -- --ignored --nocapture
```

Result after bundle setup: fail, `unsupported_runtime`/runtime-load blocker for
the selected current model category. The smoke now prints the resolved model
path, configured input/output names, and expected embedding dimension before
session construction. A 20-second timeout run printed:
`speakerEmbeddingResolvedModelPath=.../speaker-embedding.onnx`,
`speakerEmbeddingExpectedDimension=256`, and auto input/output selection. ONNX
Runtime session construction did not return within the timeout.

Offline ONNX metadata inspection with the local ignored helper venv classified
the current model as feature-input, not waveform-input:
input `feats` is f32 `[B,T,80]`, output `embs` is f32 `[B,256]`. The native
adapter now detects this metadata shape as `unsupported_runtime` when metadata
is available because this tranche intentionally does not add fbank/mel
preprocessing. The observed model category is therefore not compatible with the
current waveform adapter, which expects `[batch, 1, samples]`.

Transcription ONNX diarization smoke:

```bash
RUN_NATIVE_SPEAKER_MODEL_TESTS=1 \
SPEAKER_EMBEDDING_MODEL_BUNDLE="$SMOKE_ROOT/models/wespeaker-voxceleb-resnet34-LM/main" \
SPEAKER_EMBEDDING_MODEL_FILE="speaker-embedding.onnx" \
DIARIZATION_AUDIO_PATH="$SMOKE_ROOT/audio/native-transcription-smoke.wav" \
SPEAKER_EMBEDDING_DIMENSION=256 \
cargo test -p moritzbrantner-audio-analysis-transcription \
  --features diarization,onnx,model-bundles \
  native_onnx_diarization_smoke_when_requested -- --ignored --nocapture
```

Result after bundle setup: fail, timeout during the same ONNX speaker session
construction path before diarization diagnostics or strict transcript speaker
validation were reached. Classification: runtime/model compatibility blocker
for the selected `speaker-embedding.onnx` feature-input model, not a missing
bundle setup failure. The smoke still uses direct samples and mock ASR timing.

External WhisperX parity smoke:

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

Result after local venv setup: pass for non-diarization parity. The ignored
local venv `.audio-tools/whisperx-venv` installed `whisperx==3.8.6`; its
`whisperx --help` command worked, and the parity smoke passed with `tiny.en`,
CPU, and int8 compute. Token-gated diarization parity was not run because
`HF_TOKEN` was absent.

Backlog from this tranche:

- Pick a waveform-input ONNX speaker embedding model, or add explicit
  feature-extraction preprocessing for feature-input speaker models such as the
  current `hbredin/wespeaker-voxceleb-resnet34-LM` main artifact.
- Investigate why local ONNX Runtime session construction hangs on
  `speaker-embedding.onnx` before metadata can be queried through Rust.
- Run token-gated WhisperX diarization parity once `HF_TOKEN` is present.
- Keep throughput work separate: ASR reports
  `batchExecution=candle-whisper-sequential`, wav2vec2 alignment executes per
  segment, and ONNX diarization embeds windows one by one.

Optional external WhisperX parity can be run manually when Python WhisperX and
input media are configured:

```bash
RUN_WHISPERX_PARITY_TESTS=1 \
WHISPERX_COMMAND=whisperx \
WHISPERX_AUDIO_PATH=/path/to/audio.wav \
cargo test -p moritzbrantner-audio-analysis-transcription external_whisperx_parity_when_requested -- --ignored --nocapture
```

Set `WHISPERX_EXPECTED_JSON=/path/to/expected.json` to compare command output
against an imported WhisperX fixture. Optional parity-only overrides are
`WHISPERX_MODEL`, `WHISPERX_LANGUAGE`, `WHISPERX_DEVICE`, and
`WHISPERX_COMPUTE_TYPE`. Set `WHISPERX_DIARIZE=1` only when `HF_TOKEN` is
available.

Optional local native media decode can be run when the crate is built with
`audio-io` and FFmpeg can decode the local input:

```bash
RUN_NATIVE_MEDIA_DECODE_TESTS=1 \
TRANSCRIPTION_MEDIA_PATH=/path/to/video-or-audio-container \
cargo test -p moritzbrantner-audio-analysis-transcription \
  --features audio-io \
  native_media_decode_when_requested -- --ignored --nocapture
```

Optional local ONNX speaker embedding smoke can be run with a caller-owned ONNX
speaker model bundle and 16 kHz WAV:

```bash
RUN_NATIVE_SPEAKER_MODEL_TESTS=1 \
SPEAKER_EMBEDDING_MODEL_BUNDLE=/path/to/onnx-speaker-model \
SPEAKER_EMBEDDING_MODEL_FILE=model.onnx \
DIARIZATION_AUDIO_PATH=/path/to/meeting.wav \
cargo test -p moritzbrantner-audio-analysis-speakers \
  --features onnx,model-bundles \
  onnx_speaker_embedding_smoke_when_requested -- --ignored --nocapture
```

Set `SPEAKER_EMBEDDING_DIMENSION` if the model does not emit the default 192
dimensions expected by the smoke. Set `SPEAKER_EMBEDDING_INPUT_NAME` and
`SPEAKER_EMBEDDING_OUTPUT_NAME` when the model has multiple inputs or outputs.

The transcription pipeline's explicit ONNX diarization path has its own ignored
smoke. It reads the same caller-owned 16 kHz WAV, builds a direct-sample
pipeline request with mock ASR timing, and verifies that
`diarization.speakerEmbeddingModelBundle` produces ONNX diarization diagnostics:

```bash
RUN_NATIVE_SPEAKER_MODEL_TESTS=1 \
SPEAKER_EMBEDDING_MODEL_BUNDLE=/path/to/onnx-speaker-model \
SPEAKER_EMBEDDING_MODEL_FILE=model.onnx \
DIARIZATION_AUDIO_PATH=/path/to/meeting-16khz.wav \
SPEAKER_EMBEDDING_DIMENSION=192 \
cargo test -p moritzbrantner-audio-analysis-transcription \
  --features diarization,onnx,model-bundles \
  native_onnx_diarization_smoke_when_requested -- --ignored --nocapture
```

## Performance Reality Backlog

- ASR batch execution is currently `candle-whisper-sequential`.
- wav2vec2 alignment executes the model per segment.
- Diarization embeds and clusters speech windows one by one.
- None of these claim WhisperX throughput parity.

Future throughput work is separate from correctness work: provider-level
Whisper mel/model batching if Candle supports it, wav2vec2 segment batching if
the model API supports it, and batched ONNX embedding windows if the runtime and
model input shape support batched waveforms.

## Recommended Next Order

1. Pick a waveform-input local ONNX speaker embedding model, or add fbank/mel
   preprocessing for feature-input models and rerun the speaker plus
   transcription ONNX diarization smokes.
2. Investigate the ONNX Runtime session-load timeout for
   `speaker-embedding.onnx`.
3. Run token-gated WhisperX diarization parity after `HF_TOKEN` is available.
