# Audio Testing

The audio packages use a tiered test strategy.

## Fast PR Checks

Run these for normal development:

```bash
cargo fmt --all --check

cargo clippy \
  -p moenarch-audio-analysis-core \
  -p moenarch-audio-analysis-fourier \
  -p moenarch-audio-analysis-io \
  -p moenarch-audio-analysis-pitch \
  -p moenarch-audio-analysis-processing \
  -p moenarch-audio-analysis-recognition \
  -p moenarch-audio-analysis-rhythm \
  -p moenarch-audio-analysis-separation \
  -p moenarch-audio-analysis-speakers \
  -p moenarch-audio-analysis-synthesis \
  -p moenarch-audio-generation-midi \
  --all-targets -- -D warnings

PROPTEST_CASES=128 cargo test \
  -p moenarch-audio-analysis-core \
  -p moenarch-audio-analysis-fourier \
  -p moenarch-audio-analysis-io \
  -p moenarch-audio-analysis-pitch \
  -p moenarch-audio-analysis-processing \
  -p moenarch-audio-analysis-recognition \
  -p moenarch-audio-analysis-rhythm \
  -p moenarch-audio-analysis-separation \
  -p moenarch-audio-analysis-speakers \
  -p moenarch-audio-analysis-synthesis \
  -p moenarch-audio-generation-midi

cargo test \
  --test audio_surface_audit \
  --test audio_surface_public_api \
  --test audio_feature_contracts \
  --test audio_music_pipeline \
  --test audio_voice_pipeline \
  --test audio_transcription_contracts \
  --test audio_pipeline

bun run audio-wasm:test
bun run audio-app:typecheck
```

These tests are deterministic and do not require FFmpeg, Demucs, GPUs, network,
or committed audio fixtures. Synthetic audio lives in
`audio-analysis-test-support`.

No generated media, model files, virtual environments, or downloaded tool
artifacts are checked into git. Local external tools are installed under
`.audio-tools/`, which is ignored by git.

General e2e and radiance tools use the same installer helpers under
`.external-test-tools/`; see `docs/EXTERNAL_TEST_TOOLS.md`.

## Integration Checks

Root integration tests exercise the audio packages together:

```bash
cargo test --test audio_pipeline
```

FFmpeg decode coverage is gated because it requires external binaries:

```bash
bash scripts/setup_audio_external_tools.sh ffmpeg
FFMPEG_EXTERNAL_TESTS=1 cargo test -p moenarch-audio-analysis-io --test ffmpeg_decode
```

If `FFMPEG_EXTERNAL_TESTS` is not set, the test skips. If the variable is set,
missing FFmpeg is a failure.

## Demucs Smoke Test

The Demucs wrapper has fast argument/path validation tests, and real Demucs
execution is verified only in the external-tool tier. Install and verify Demucs
into the ignored local tools directory first:

```bash
bash scripts/setup_audio_external_tools.sh demucs
export PATH="$PWD/.audio-tools/bin:$PATH"
RUN_REAL_DEMUCS_TESTS=1 cargo test -p moenarch-audio-analysis-separation \
  real_demucs_smoke_test_when_requested -- --ignored --nocapture
```

By default, the setup script tries the shared Python virtual environment in
`.audio-tools/python-venv`. If that fails, it falls back to a Conda-compatible
environment in `.audio-tools/demucs-conda`. Conda, Mamba, and Micromamba are
supported; if none exists locally, the setup script can install Micromamba under
`.audio-tools/`:

```bash
bash scripts/setup_audio_external_tools.sh conda demucs
```

You can force one installer:

```bash
AUDIO_DEMUCS_INSTALLER=venv bash scripts/setup_audio_external_tools.sh demucs
AUDIO_DEMUCS_INSTALLER=conda bash scripts/setup_audio_external_tools.sh conda demucs
```

Both paths symlink `demucs` into `.audio-tools/bin` and verify `demucs --help`
before tests run. You can override the command with
`DEMUCS_COMMAND=/path/to/demucs`.

## Native Transcription Smoke Tests

`audio-analysis-transcription` owns real audio/video-to-text execution. Candle
Whisper CUDA is the primary native target, and Candle CPU is the local
development fallback when the crate is built with `candle`. CUDA is used only
when the crate is built with `cuda` and the request explicitly selects that
device. Default tests use deterministic samples, WAV fixtures, CTC primitives,
synthetic wav2vec2 safetensors bundles, and mock providers; they do not require
real model files, CUDA, Python, Hugging Face tokens, downloads, or network
access.

Native path decoding accepts WAV files by default through the native hound-based
reader. Broader container/video decode is feature-gated behind `audio-io`,
which reuses FFmpeg-backed `audio-analysis-io`; it is not part of default tests.
Browser and WASM package surfaces can plan or import transcript data, but they
do not run native ASR.

Imported transcript normalization belongs to `text-transcripts` through
`transcripts.normalize` and `transcripts.importWhisperX`. Recognition package
surfaces no longer advertise transcription operations. External Python
WhisperX is compatibility/parity tooling only.

Candle Whisper CUDA smoke test:

```bash
RUN_NATIVE_TRANSCRIPTION_TESTS=1 \
TRANSCRIPTION_MODEL_BUNDLE=/path/to/whisper-tiny-or-base \
TRANSCRIPTION_AUDIO_PATH=/path/to/audio.wav \
cargo test -p moenarch-audio-analysis-transcription \
  --features candle,cuda,model-bundles \
  candle_whisper_cuda_smoke_when_requested -- --ignored --nocapture
```

Candle Whisper CUDA translate-to-English smoke test:

```bash
RUN_NATIVE_TRANSLATION_TESTS=1 \
TRANSCRIPTION_MODEL_BUNDLE=/path/to/whisper-tiny-or-small \
TRANSCRIPTION_AUDIO_PATH=/path/to/non-english-audio.wav \
cargo test -p moenarch-audio-analysis-transcription \
  --features candle,cuda,model-bundles \
  candle_whisper_cuda_translate_smoke_when_requested -- --ignored --nocapture
```

Native translation uses Whisper's built-in `translate` task and reports
`translationRuntime=whisper-task` and `translationTargetLanguage=en`.
wav2vec2/CTC alignment is intentionally rejected for translated output because
the transcript text is no longer source-language audio text. OPUS-MT/Marian
post-ASR translation is not part of this path.

The native Whisper path attempts timestamp-token segment timing automatically
when the tokenizer exposes Whisper timestamp metadata. If timestamp decoding
does not produce bounded text segments, it falls back to chunk/window segment
timing. Tokenizer, prompt, parser, fallback, timestamp-token segment timing,
global mapping, approximate word projection, and alignment overwrite behavior
are covered by hermetic unit tests that do not require model files.

Native transcription pipeline wiring is also covered hermetically. Unit tests
prove native providers supply `CtcForcedAligner` when alignment is enabled,
leave alignment absent when disabled, run alignment before diarization
assignment, and exercise a synthetic tiny wav2vec2 bundle through the pipeline
without real model files, CUDA, Python, WhisperX, Hugging Face tokens, network,
or downloads. Batch option tests are hermetic as well: `max_batch_size=0`
rejection, output order, batch counts, unbounded batch diagnostics, and
`batchExecution=candle-whisper-sequential` diagnostics are covered without real
models. This is semantic batch grouping and does not claim throughput parity or
true tensor-batched Whisper execution.

Native diarization seam coverage is hermetic as well. Unit tests cover
transcript-timing-derived speech spans, fallback to energy VAD when transcript
timing is absent, `min_speakers`/`max_speakers` validation and diagnostics, and
pipeline diarization diagnostics without real speaker models, Python, network,
CUDA, Hugging Face tokens, or downloads. `audio.transcription.diarizationPlan`
is a Debug-only planning surface: it reports the current heuristic runtime and
the accepted opt-in ONNX speaker embedding direction, not production pyannote
parity. `audio-analysis-speakers` exposes `SpeakerEmbeddingProvider` for
model-backed embedders while keeping existing `SpeakerEmbeddingExtractor`
callers working.
`audio.transcription.alignmentBundlePlan` is also Debug-only: it can report a
static plan without local files, or inspect local wav2vec2 config/tokenizer/
safetensors metadata without model inference when a bundle path is provided.

On the RTX 3060 Ti development host used for the current smoke, the working
local assets are:

- `$SMOKE_ROOT/whisper-tiny`
- `$SMOKE_ROOT/audio/native-transcription-smoke.wav`
- `$SMOKE_ROOT/cuda12-libs`

That host's `/usr/local/cuda` points at CUDA 13.3, so the passing smoke points
`RUSTFLAGS`, `LIBRARY_PATH`, and `LD_LIBRARY_PATH` at the CUDA 12.3 shim to
avoid `CUBLAS_STATUS_NOT_INITIALIZED`.

CTC alignment wav2vec2 smoke test:

```bash
cargo test -p moenarch-audio-analysis-transcription \
  --features candle,alignment,model-bundles \
  ctc_alignment_wav2vec2_smoke_when_requested -- --ignored --nocapture
```

The ignored smoke has defaults when the environment is unset: `ALIGNMENT_AUDIO_PATH`
falls back to `TRANSCRIPTION_AUDIO_PATH`, the checked-in whisper.cpp sample WAV,
or a generated 16 kHz WAV. `ALIGNMENT_MODEL_BUNDLE` is used first, then
`ALIGNMENT_MODEL_DIR`, `$XDG_DATA_HOME/video-analysis-smoke/models`,
`$HOME/.local/share/video-analysis-smoke/models`, or a generated tiny wav2vec2
bundle. Set `RUN_NATIVE_ALIGNMENT_TESTS=0` to skip it explicitly when running
ignored tests.

The CTC path validates wav2vec2 bundle files, config, tokenizer vocabulary, and
preprocessor metadata. Supported local `Wav2Vec2ForCTC` safetensors bundles
execute through Candle and native CTC trellis/backtracking. Unsupported
architectures or safetensors layouts return typed errors. Positional
convolution supports plain weights, legacy `weight_g`/`weight_v` weight norm,
and PyTorch parametrization `original0/original1` weight norm layouts. The
ignored real-bundle smoke prints the wav2vec2 layout report before execution,
including stable-layer-norm status, feature-extractor norm, encoder layer
count, missing tensor keys, and unsupported reasons.

2026-06-10 validation: the default speech WAV was present, but the default
caller-owned wav2vec2 bundle directory was missing. The real-bundle smoke was
therefore classified as `setup_error` before layout inspection or inference.

Native media/container decode smoke test:

```bash
RUN_NATIVE_MEDIA_DECODE_TESTS=1 \
TRANSCRIPTION_MEDIA_PATH=/path/to/video-or-audio-container \
cargo test -p moenarch-audio-analysis-transcription \
  --features audio-io \
  native_media_decode_when_requested -- --ignored --nocapture
```

The media smoke asserts that explicit `audio-io` decode returns non-empty
finite mono 16 kHz audio, preserves the source path, and reports
`decodeRoute=audio-io-media-decode`-equivalent internal diagnostics. Default
tests still cover native WAV and direct-sample diagnostics without FFmpeg.

2026-06-10 validation: the checked-in WebM fixture decoded successfully with
FFmpeg after the ignored smoke harness resolved workspace-root-relative fixture
paths. `audio-io` remains opt-in and default tests still do not require FFmpeg.

Capability tiers for transcription:

- Default hermetic: direct samples, native WAV reads, VAD, batch semantics,
  WhisperX JSON import, mock command parity, deterministic alignment, synthetic
  wav2vec2 bundles, and heuristic diarization diagnostics.
- Feature-gated: Candle Whisper (`candle`), CUDA (`cuda`), local bundle
  validation (`model-bundles`), native Whisper translate-to-English
  (`provider.task="translate"`), CTC alignment (`alignment`), heuristic speaker
  diarization (`diarization`), opt-in ONNX speaker embeddings (`onnx`), and
  non-WAV media decode (`audio-io`).
- Ignored local smoke: Candle Whisper CUDA transcription and translation, real wav2vec2 alignment,
  `audio-io` media/container decode, ONNX speaker embeddings, and the
  transcription ONNX diarization path.
- External compatibility only: Python WhisperX execution and pyannote-backed
  diarization.

Diarization baseline smoke test:

```bash
RUN_NATIVE_DIARIZATION_TESTS=1 \
DIARIZATION_AUDIO_PATH=/path/to/meeting.wav \
cargo test -p moenarch-audio-analysis-speakers \
  --features external-tests \
  native_diarization_baseline_smoke_when_requested -- --ignored --nocapture
```

ONNX speaker embedding smoke test:

```bash
RUN_NATIVE_SPEAKER_MODEL_TESTS=1 \
ORT_DYLIB_PATH="$PWD/.audio-tools/whisperx-venv/lib/python3.11/site-packages/onnxruntime/capi/libonnxruntime.so.1.26.0" \
SPEAKER_EMBEDDING_MODEL_BUNDLE=/path/to/onnx-speaker-model \
SPEAKER_EMBEDDING_MODEL_FILE=model.onnx \
DIARIZATION_AUDIO_PATH=/path/to/meeting.wav \
cargo test -p moenarch-audio-analysis-speakers \
  --features onnx,model-bundles \
  onnx_speaker_embedding_smoke_when_requested -- --ignored --nocapture
```

The ONNX smoke uses a caller-owned local bundle only. It does not download
models or use Python, Hugging Face auth, pyannote auth, CUDA, or network access.
The WAV must already be 16 kHz. Set `SPEAKER_EMBEDDING_DIMENSION` when the
model output dimension differs from the smoke default of 192. Set
`SPEAKER_EMBEDDING_MODEL_FILE`, `SPEAKER_EMBEDDING_INPUT_NAME`, and
`SPEAKER_EMBEDDING_OUTPUT_NAME` when the local bundle does not use `model.onnx`
or needs explicit ONNX IO selection.

2026-06-10 validation update: `scripts/sync_model_bundles.sh` provisioned the
current `hbredin/wespeaker-voxceleb-resnet34-LM` main artifact as
`speaker-embedding.onnx` under the ignored smoke model root. Static ONNX
metadata inspection reported f32 input `feats` with `[B,T,80]` and f32 output
`embs` with `[B,256]`. The Rust adapter now auto-detects that feature-input
shape and builds deterministic CPU fbank/log-mel features from 16 kHz mono
audio. The ignored smoke printed `onnxStaticMetadata=ok`,
`onnxGraphDomains=<default>:110`, `onnxGraphOpsets=<default>:14`,
`onnxGraphInitializerCount=75`, `onnxGraphNodeCount=110`, and
`onnxSessionOptions=cpu-single-threaded,no-memory-pattern,graph-optimization-disabled`.
With `ORT_DYLIB_PATH` unset, file and memory diagnostic load modes timed out
via external `timeout 120s` exit `124` after `onnxSessionBuilder=begin` and
before `onnxSessionBuilder=ok`. Python ONNX Runtime 1.26.0 loaded the same
artifact. With `ORT_DYLIB_PATH` pointed at the local `.audio-tools/whisperx-venv`
`libonnxruntime.so.1.26.0`, both file and memory speaker smokes passed with
`onnxSessionCommit=ok`, `onnxSessionLoad=ok`,
`speakerEmbeddingInputKind=feature`, and `speakerFeatureBins=80`.
Classification: implicit ORT dynamic-library selection/setup blocker on this
host, not missing-bundle `setup_error` or waveform/feature-shape rejection.

Transcription ONNX diarization smoke test:

```bash
RUN_NATIVE_SPEAKER_MODEL_TESTS=1 \
ORT_DYLIB_PATH="$PWD/.audio-tools/whisperx-venv/lib/python3.11/site-packages/onnxruntime/capi/libonnxruntime.so.1.26.0" \
SPEAKER_EMBEDDING_MODEL_BUNDLE=/path/to/onnx-speaker-model \
SPEAKER_EMBEDDING_MODEL_FILE=model.onnx \
DIARIZATION_AUDIO_PATH=/path/to/meeting-16khz.wav \
SPEAKER_EMBEDDING_DIMENSION=192 \
cargo test -p moenarch-audio-analysis-transcription \
  --features diarization,onnx,model-bundles \
  native_onnx_diarization_smoke_when_requested -- --ignored --nocapture
```

This smoke reads the caller-owned 16 kHz WAV, passes direct samples into the
native transcription pipeline, uses mock ASR with timed segments, and verifies
the explicit `diarization.speakerEmbeddingModelBundle` path. It is ignored by
default and does not use FFmpeg, Python, CUDA, pyannote, Hugging Face auth,
network, or downloaded model files.

2026-06-10 validation update: after local bundle provisioning, the transcription
ONNX diarization smoke still used direct samples and mock ASR timing. Its
static preflight printed `onnxStaticMetadata=ok`, `feats` `[B,T,80]`, `embs`
`[B,256]`, default-domain ONNX ops only, opset 14, 110 nodes, 75 initializers,
and the same single-threaded/no-memory-pattern session options. With
`ORT_DYLIB_PATH` unset, diagnostic file and memory modes timed out via external
`timeout 120s` exit `124` after `onnxSessionBuilder=begin` and before
`onnxSessionBuilder=ok`. With explicit ONNX Runtime 1.26.0 from
`.audio-tools/whisperx-venv`, the smoke passed and reached
`diarizationRuntime=onnx`, `speakerEmbeddingProvider=onnx`,
`speakerEmbeddingDimension=256`, `diarizationBaseline=false`, and strict
transcript speaker validation. Classification: implicit ORT dynamic-library
selection/setup blocker on this host, fixed for local smokes by explicit
`ORT_DYLIB_PATH`.

Current performance reality:

- ASR batch execution reports `batchExecution=candle-whisper-sequential`.
- wav2vec2 alignment runs model execution per segment.
- Diarization embeds and clusters windows one by one.

These are correctness paths, not WhisperX throughput parity claims.

Token-gated WhisperX diarization parity uses the external Python WhisperX
provider and is intentionally local/ignored:

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

2026-06-10 validation: non-diarization WhisperX parity passed with
`.audio-tools/whisperx-venv/bin/whisperx`, `tiny.en`, CPU, and int8. Initial
token-gated diarization attempts reached pyannote but failed as setup/auth:
first `huggingface_hub.errors.GatedRepoError: 403 Client Error`, then
`huggingface_hub.errors.HfHubHTTPError: 403 Forbidden` until the fine-grained
token allowed public gated repositories and access to
`pyannote/speaker-diarization-community-1`.

2026-06-11 validation: token-gated WhisperX diarization parity passed with
`WHISPERX_DIARIZE=1`, `.audio-tools/whisperx-venv/bin/whisperx`, `tiny.en`,
CPU, int8, and `HF_TOKEN="$HF_TOKEN"`. Pyannote/Hugging Face access was valid
for this run. No token value was documented.

## whisper.cpp Compatibility Smoke Test

Native whisper.cpp remains a compatibility path outside the primary
transcription provider. It is tested only when explicitly requested.

Prepare a local 16 kHz mono WAV fixture and a cached whisper.cpp model first.
The smoke test checks that the model already exists before calling the native
transcriber, so it does not download models itself:

```bash
RUN_NATIVE_WHISPER_TESTS=1 \
NATIVE_WHISPER_AUDIO_PATH=/path/to/fixture-16khz-mono.wav \
cargo test -p moenarch-audio-analysis-transcription \
  --features native,external-tests \
  --test whisper_native_external native_whisper_cpp_smoke_when_requested -- --ignored --nocapture
```

Optional override:

```bash
WHISPER_CPP_MODEL_STORE="$PWD/.model-runtime/whisper-cpp" \
RUN_NATIVE_WHISPER_TESTS=1 \
NATIVE_WHISPER_AUDIO_PATH=/path/to/fixture-16khz-mono.wav \
cargo test -p moenarch-audio-analysis-transcription \
  --features native,external-tests \
  --test whisper_native_external native_whisper_cpp_smoke_when_requested -- --ignored --nocapture
```

If `RUN_NATIVE_WHISPER_TESTS` is not set, the ignored smoke test exits as a
skip. If it is set and the fixture or cached model is missing, the test fails
with setup text.

## Benchmarks

Compute-heavy crates have Criterion benchmarks:

```bash
cargo bench \
  -p moenarch-audio-analysis-core \
  -p moenarch-audio-analysis-fourier \
  -p moenarch-audio-analysis-pitch \
  -p moenarch-audio-analysis-processing \
  -p moenarch-audio-analysis-recognition \
  -p moenarch-audio-analysis-rhythm

python3 scripts/check_audio_bench.py
```

`scripts/check_audio_bench.py` compares Criterion median estimates against
`benches/baselines/audio-linux-x86_64.json` and fails when a benchmark regresses
by more than 15 percent. The committed baseline is intentionally empty until a
clean `main` run is used to populate it.
