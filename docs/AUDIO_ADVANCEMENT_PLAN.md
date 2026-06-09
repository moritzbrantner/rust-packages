# Audio Advancement Plan

This tracker keeps audio package capability work split into small, independent
tasks. Default builds must remain deterministic, pure Rust, browser-safe, and
free of model downloads or external command execution.

## Ownership

| Crate | Owns |
|---|---|
| `audio-analysis-core` | Shared audio contracts, sample conversion, windowing, level series |
| `audio-analysis-io` | Audio input, waveform batch decode, WAV export, FFmpeg-backed helpers |
| `audio-analysis-fourier` | FFT, STFT, spectral features, mel-style summaries |
| `audio-analysis-pitch` | Pitch, note tracking, chroma and pitch-class summaries |
| `audio-analysis-rhythm` | Onsets, tempo, beat grids, music timing |
| `audio-analysis-processing` | Realtime/offline transforms, effects, mixdown, loudness metrics |
| `audio-analysis-recognition` | Generic audio embeddings, similarity, recognition, transcription contracts |
| `audio-analysis-speakers` | Speaker enrollment, VAD, diarization, transcript speaker assignment |
| `audio-analysis-separation` | Demucs/HTDemucs planning and opt-in execution |
| `audio-analysis-synthesis` | Deterministic audio generation from events and timelines |
| `audio-generation-midi` | MIDI-like note sequencing, MIDI export, audio rendering |

## New Operation IDs

| Operation | Crate | Runtime policy |
|---|---|---|
| `audio.processing.loudness` | `audio-analysis-processing` | Pure Rust, WASM-safe |
| `audio.fourier.features` enhancements | `audio-analysis-fourier` | Pure Rust, WASM-safe |
| `audio.pitch.chroma` | `audio-analysis-pitch` | Pure Rust, WASM-safe |
| `audio.speakers.vad` | `audio-analysis-speakers` | Pure Rust, WASM-safe |
| `audio.speakers.diarize` | `audio-analysis-speakers` | Pure Rust baseline, imported segments supported |
| `audio.transcription.transcribe` | `audio-analysis-transcription` | Native ASR orchestration; model execution requires explicit bundle/features |
| `audio.transcription.importWhisperX` | `audio-analysis-transcription` | Delegates WhisperX JSON import to `text-transcripts` without running models |
| `transcripts.normalize`, `transcripts.importWhisperX` | `text-transcripts` | Imported transcript contract conversion and normalization |
| `audio.transcription.plan` | `audio-analysis-transcription` | Debug-only provider plan, no execution |
| `audio.io.wavSummary` | `audio-analysis-io` | Inline pure summary; path reads are native/server only |
| `audio.io.probePlan` | `audio-analysis-io` | Debug-only plan, no FFprobe execution |
| `audio.separation.runDemucs` | `audio-analysis-separation` | Server/native only, external-test gated |
| `audio.midi.fromPitchTrack` | `audio-generation-midi` | Pure Rust, WASM-safe |
| `audio.synthesis.clickTrack` | `audio-analysis-synthesis` | Pure Rust, WASM-safe |

## Feature-Gate Policy

- Default features must not download models, invoke FFmpeg, run Demucs, run
  Whisper, require GPU/native inference, or write generated media.
- Real external tools use ignored or environment-gated tests.
- Missing external tools should be a skip unless the corresponding opt-in
  environment variable is set; when set, missing setup is a test failure.
- Runtime catalogs and plan operations must clearly state setup commands and
  required features without performing setup.

## Test Matrix

Baseline:

```bash
cargo fmt --all --check
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
cargo test --test audio_pipeline --test audio_surface_public_api --test audio_transcription_contracts
bun run audio-wasm:test
```

Per changed package:

```bash
cargo test -p <changed-library-crate>
cargo test -p <changed-cli-crate> --tests
cargo test -p <changed-server-crate> --tests
bun run --cwd packages/<crate-name>-wasm test
bun run --cwd packages/<crate-name>-app typecheck
```

## Progress

| Task | Status |
|---|---|
| 0.1 capture audio baseline | done |
| 0.2 add tracker doc | done |
| 1.1 shared feature-series contracts | done |
| 1.2 windowed level helpers | done |
| 2.1 loudness metrics and surface operation | done |
| 3.1 Fourier feature enhancements | done |
| 3.2 chroma and pitch-class helpers | done |
| 4.1 VAD surface operation | done |
| 4.2 deterministic diarization surface operation | done |
| 4.3 transcript speaker assignment policies | done |
| 5.1 ASR planning operation | done |
| 5.2 generic imported-transcription workflow operation | done |
| 5.3 optional native Whisper smoke path | done |
| 6.1 pure WAV decode and summary | done |
| 6.2 audio probe plan | done |
| 7.1 separation plan quality | done |
| 7.2 opt-in Demucs execution surface | done |
| 8.1 pitch-track to MIDI workflow | done |
| 8.2 click-track synthesis workflow | done |
| 9 package app workflow/debug grouping | done |
| 10 integration tests and docs | done |
| 11 release surface audit | done |
| 12 all audio WASM package tests | done |
| 13 all audio app typechecks | done |
| 14 audio package dry-runs | blocked until prerequisite crates are published |

## Release Hardening

Capability work is complete for the current audio slice. Remaining release work
is hardening and verification:

- Keep `tests/audio_surface_audit.rs` green for every audio package surface.
- Run `bun run audio-wasm:test` across every `packages/audio-*-wasm` package.
- Run `bun run audio-app:typecheck` across every `packages/audio-*-app` package.
- Run `cargo package --allow-dirty -p <crate-name>` for every publishable audio
  library, CLI wrapper, server wrapper, and Rust WASM binding crate after the
  prerequisite crates listed in `docs/RELEASE_CHECKLIST.md` are available from
  crates.io at the required versions.
- Keep Demucs, FFmpeg, and Whisper execution in explicit external-tool tiers.
