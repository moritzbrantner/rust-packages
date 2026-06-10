# Audio Testing

The audio packages use a tiered test strategy.

## Fast PR Checks

Run these for normal development:

```bash
cargo fmt --all --check

cargo clippy \
  -p moritzbrantner-audio-analysis-core \
  -p moritzbrantner-audio-analysis-fourier \
  -p moritzbrantner-audio-analysis-io \
  -p moritzbrantner-audio-analysis-pitch \
  -p moritzbrantner-audio-analysis-processing \
  -p moritzbrantner-audio-analysis-recognition \
  -p moritzbrantner-audio-analysis-rhythm \
  -p moritzbrantner-audio-analysis-separation \
  -p moritzbrantner-audio-analysis-speakers \
  -p moritzbrantner-audio-analysis-synthesis \
  -p moritzbrantner-audio-generation-midi \
  --all-targets -- -D warnings

PROPTEST_CASES=128 cargo test \
  -p moritzbrantner-audio-analysis-core \
  -p moritzbrantner-audio-analysis-fourier \
  -p moritzbrantner-audio-analysis-io \
  -p moritzbrantner-audio-analysis-pitch \
  -p moritzbrantner-audio-analysis-processing \
  -p moritzbrantner-audio-analysis-recognition \
  -p moritzbrantner-audio-analysis-rhythm \
  -p moritzbrantner-audio-analysis-separation \
  -p moritzbrantner-audio-analysis-speakers \
  -p moritzbrantner-audio-analysis-synthesis \
  -p moritzbrantner-audio-generation-midi

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
FFMPEG_EXTERNAL_TESTS=1 cargo test -p moritzbrantner-audio-analysis-io --test ffmpeg_decode
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
RUN_REAL_DEMUCS_TESTS=1 cargo test -p moritzbrantner-audio-analysis-separation \
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
cargo test -p moritzbrantner-audio-analysis-transcription \
  --features candle,cuda,model-bundles \
  candle_whisper_cuda_smoke_when_requested -- --ignored --nocapture
```

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
sequential-provider diagnostics are covered without real models.

Native diarization seam coverage is hermetic as well. Unit tests cover
transcript-timing-derived speech spans, fallback to energy VAD when transcript
timing is absent, `min_speakers`/`max_speakers` validation and diagnostics, and
pipeline diarization diagnostics without real speaker models, Python, network,
CUDA, Hugging Face tokens, or downloads. `audio.transcription.diarizationPlan`
is a Debug-only planning surface: it reports the current heuristic runtime and
future model-backed provider directions, not production pyannote parity.

On the RTX 3060 Ti development host used for the current smoke, the working
local assets are:

- `/home/moenarch/.local/share/video-analysis-smoke/whisper-tiny`
- `/home/moenarch/.local/share/video-analysis-smoke/audio/native-transcription-smoke.wav`
- `/home/moenarch/.local/share/video-analysis-smoke/cuda12-libs`

That host's `/usr/local/cuda` points at CUDA 13.3, so the passing smoke points
`RUSTFLAGS`, `LIBRARY_PATH`, and `LD_LIBRARY_PATH` at the CUDA 12.3 shim to
avoid `CUBLAS_STATUS_NOT_INITIALIZED`.

CTC alignment wav2vec2 smoke test:

```bash
RUN_NATIVE_ALIGNMENT_TESTS=1 \
ALIGNMENT_MODEL_BUNDLE=/path/to/wav2vec2 \
TRANSCRIPTION_AUDIO_PATH=/path/to/audio.wav \
cargo test -p moritzbrantner-audio-analysis-transcription \
  --features candle,alignment,model-bundles \
  ctc_alignment_wav2vec2_smoke_when_requested -- --ignored --nocapture
```

The CTC path validates wav2vec2 bundle files, config, tokenizer vocabulary, and
preprocessor metadata. Supported local `Wav2Vec2ForCTC` safetensors bundles
execute through Candle and native CTC trellis/backtracking. Unsupported
architectures or safetensors layouts return typed errors. Positional
convolution supports plain weights, legacy `weight_g`/`weight_v` weight norm,
and PyTorch parametrization `original0/original1` weight norm layouts.

Native media/container decode smoke test:

```bash
RUN_NATIVE_MEDIA_DECODE_TESTS=1 \
TRANSCRIPTION_MEDIA_PATH=/path/to/video-or-audio-container \
cargo test -p moritzbrantner-audio-analysis-transcription \
  --features audio-io \
  native_media_decode_when_requested -- --ignored --nocapture
```

Capability tiers for transcription:

- Default hermetic: direct samples, native WAV reads, VAD, batch semantics,
  WhisperX JSON import, mock command parity, deterministic alignment, synthetic
  wav2vec2 bundles, and heuristic diarization diagnostics.
- Feature-gated: Candle Whisper (`candle`), CUDA (`cuda`), local bundle
  validation (`model-bundles`), CTC alignment (`alignment`), heuristic speaker
  diarization (`diarization`), and non-WAV media decode (`audio-io`).
- Ignored local smoke: Candle Whisper CUDA, real wav2vec2 alignment, and
  `audio-io` media/container decode.
- External compatibility only: Python WhisperX execution and pyannote-backed
  diarization.

Diarization baseline smoke test:

```bash
RUN_NATIVE_DIARIZATION_TESTS=1 \
DIARIZATION_AUDIO_PATH=/path/to/meeting.wav \
cargo test -p moritzbrantner-audio-analysis-speakers \
  --features external-tests \
  native_diarization_baseline_smoke_when_requested -- --ignored --nocapture
```

## whisper.cpp Compatibility Smoke Test

Native whisper.cpp remains a compatibility path outside the primary
transcription provider. It is tested only when explicitly requested.

Prepare a local 16 kHz mono WAV fixture and a cached whisper.cpp model first.
The smoke test checks that the model already exists before calling the native
transcriber, so it does not download models itself:

```bash
RUN_NATIVE_WHISPER_TESTS=1 \
NATIVE_WHISPER_AUDIO_PATH=/path/to/fixture-16khz-mono.wav \
cargo test -p moritzbrantner-text-transcripts \
  --features native,external-tests \
  native_whisper_cpp_smoke_when_requested -- --ignored --nocapture
```

Optional override:

```bash
WHISPER_CPP_MODEL_STORE="$PWD/.model-runtime/whisper-cpp" \
RUN_NATIVE_WHISPER_TESTS=1 \
NATIVE_WHISPER_AUDIO_PATH=/path/to/fixture-16khz-mono.wav \
cargo test -p moritzbrantner-text-transcripts \
  --features native,external-tests \
  native_whisper_cpp_smoke_when_requested -- --ignored --nocapture
```

If `RUN_NATIVE_WHISPER_TESTS` is not set, the ignored smoke test exits as a
skip. If it is set and the fixture or cached model is missing, the test fails
with setup text.

## Benchmarks

Compute-heavy crates have Criterion benchmarks:

```bash
cargo bench \
  -p moritzbrantner-audio-analysis-core \
  -p moritzbrantner-audio-analysis-fourier \
  -p moritzbrantner-audio-analysis-pitch \
  -p moritzbrantner-audio-analysis-processing \
  -p moritzbrantner-audio-analysis-recognition \
  -p moritzbrantner-audio-analysis-rhythm

python3 scripts/check_audio_bench.py
```

`scripts/check_audio_bench.py` compares Criterion median estimates against
`benches/baselines/audio-linux-x86_64.json` and fails when a benchmark regresses
by more than 15 percent. The committed baseline is intentionally empty until a
clean `main` run is used to populate it.
