# Next Work Index

This index points to the active follow-up lanes after the package-surface audit
reached `L4 Usable` coverage across the audited library crates.

## Release Gates

Use [RELEASE_CHECKLIST.md](RELEASE_CHECKLIST.md) for release-oriented
verification and manual publish sequencing. The fast local baseline starts with:

```bash
bun run progress:check
bun run snapshot:check
bun run hygiene:generated
bun run format:check
git diff --check
python3 scripts/audit_curated_landscape.py --check
cargo test --test curated_landscape
cargo test --test contract_ownership --test dependency_layers --test foundation_surface_audit --test package_structure --test package_interop_pipeline
scripts/check-fast.sh
```

`scripts/check-fast.sh` intentionally skips browser e2e, production web builds,
and benchmarks. Use `scripts/check-preflight.sh` for the broad local CI/preflight
mirror before PR/release-oriented handoff, and `scripts/check.sh` for the full
baseline with external-tool checks. Benchmark checks belong to `bun run bench`,
`performance-ci`, or explicit benchmark commands.

Publishing remains manual. Do not add release automation unless a task
explicitly asks for it.

## PySceneDetect 0.7 Parity

Use [VIDEO_SCENE_DETECTION_PARITY.md](VIDEO_SCENE_DETECTION_PARITY.md) as the
source of truth. The current stable parity target remains PySceneDetect
`0.6.7.1`; `0.7` work is additive. Prioritize:

- timestamp and VFR metadata,
- CLI spelling compatibility,
- scene-list output exporters,
- image extraction as a separate later surface.

## Native WhisperX Replacement

Use
[NATIVE_WHISPERX_REIMPLEMENTATION_STATUS.md](NATIVE_WHISPERX_REIMPLEMENTATION_STATUS.md)
for current status, local validation history, and remaining gaps. Default tests
must remain hermetic: no Python, WhisperX, Hugging Face token, network, CUDA, or
local model files.

## External Smoke Policy

Use [EXTERNAL_TEST_TOOLS.md](EXTERNAL_TEST_TOOLS.md) for opt-in setup and check
commands. External model, audio, video-scene, and radiance smokes should remain
explicitly environment-gated and outside the default contributor gate.

## Package Surface Maintenance

Use [CRATE_SURFACE_AUDIT_PROTOCOL.md](CRATE_SURFACE_AUDIT_PROTOCOL.md) only for
the crate being touched unless a task explicitly asks for a batch audit. Keep
primary workflow operations as app defaults, keep `describe` and plan/inspect
helpers in Debug groups, and regenerate progress docs only with:

```bash
bun run progress:write
```
