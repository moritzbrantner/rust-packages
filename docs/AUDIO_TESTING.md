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

## Native Whisper Smoke Test

Audio recognition keeps native ASR out of default tests. The primary
deterministic workflow is `audio.recognition.transcribe`; the compatibility
operation is `audio.recognition.transcribeImported`. Real Whisper execution is
one provider path owned by `moritzbrantner-text-transcripts` and is only tested
when explicitly requested.

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
