# Audio Testing

The audio packages use a tiered test strategy.

## Fast PR Checks

Run these for normal development:

```bash
cargo fmt --all --check

cargo clippy \
  -p audio-analysis-core \
  -p audio-analysis-fourier \
  -p audio-analysis-io \
  -p audio-analysis-pitch \
  -p audio-analysis-processing \
  -p audio-analysis-recognition \
  -p audio-analysis-rhythm \
  -p audio-analysis-separation \
  --all-targets -- -D warnings

PROPTEST_CASES=128 cargo test \
  -p audio-analysis-core \
  -p audio-analysis-fourier \
  -p audio-analysis-io \
  -p audio-analysis-pitch \
  -p audio-analysis-processing \
  -p audio-analysis-recognition \
  -p audio-analysis-rhythm \
  -p audio-analysis-separation
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
FFMPEG_EXTERNAL_TESTS=1 cargo test -p audio-analysis-io --test ffmpeg_decode
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
RUN_REAL_DEMUCS_TESTS=1 cargo test -p audio-analysis-separation \
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

## Benchmarks

Compute-heavy crates have Criterion benchmarks:

```bash
cargo bench \
  -p audio-analysis-core \
  -p audio-analysis-fourier \
  -p audio-analysis-pitch \
  -p audio-analysis-processing \
  -p audio-analysis-recognition \
  -p audio-analysis-rhythm

python3 scripts/check_audio_bench.py
```

`scripts/check_audio_bench.py` compares Criterion median estimates against
`benches/baselines/audio-linux-x86_64.json` and fails when a benchmark regresses
by more than 15 percent. The committed baseline is intentionally empty until a
clean `main` run is used to populate it.
