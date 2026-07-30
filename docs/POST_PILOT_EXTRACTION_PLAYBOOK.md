# Post-Pilot Extraction Playbook

> **Superseded guidance:** ADR 0012 supersedes this playbook's rejection of
> primary media-family capability repositories. Its clean-copy provenance,
> signpost, namespace, independent-build, and source-removal gates remain
> applicable.

Use this playbook after the geo/map pilot when deciding whether another package
family should leave `rust-packages`. It records what the pilot proved, the
minimum extraction sequence, and the checks that should happen before a future
slice removes active ownership from this workspace.

## What Geo Proved

The geo pilot proved the hybrid strategy from
[`ADR 0011`](adr/0011-hybrid-geo-extraction-and-namespace.md):

- A coherent adjacent domain can move to a sibling repository with clean-copy
  history instead of a repo-wide history rewrite.
- Extracted crates can publish under the `moenarch-*` namespace while this
  repository keeps only migration signposts for old package names.
- The extracted family should depend on shared foundations, such as
  `moritzbrantner-runtime-core`, instead of depending back on
  `moritzbrantner-video-analysis-core`.
- Removing active ownership from `rust-packages` is separate from local build
  cache cleanup. Disk pressure from `.cargo-target`, `target`, WASM `pkg/`, or
  frontend `dist/` output is a cache policy issue, not an extraction reason.

The pilot did not prove that all media-type families should leave. It proved a
repeatable path for adjacent package families whose primary ownership is outside
the core multimodal workspace.

## Candidate Criteria

Classify a future candidate before planning extraction.

### Adjacent Package Family

Extract only when most of these are true:

- The family has a domain identity that can stand alone from video, audio,
  image, text, vector, 3D, runtime, and interoperability foundations.
- It has coherent library, CLI, server, WASM, npm, and app surfaces listed in
  [`PACKAGE_SURFACE_MATRIX.md`](PACKAGE_SURFACE_MATRIX.md), or an explicit
  reason a surface does not move.
- Its active crates do not require private path dependencies back into this
  workspace after extraction.
- It can depend on published foundation crates instead of exposing local
  workspace-only contracts.
- It can own a release cadence, repository README, issue queue, and migration
  notes outside this workspace.
- Removing it does not make core multimodal workflows harder to test or
  understand here.

Finance and geo are the reference cases: useful to multimodal workflows, but
owned best as adjacent package families.

### Core Multimodal Foundation

Do not extract when any of these are true:

- The family defines common contracts used by several media domains.
- The family is the default package-surface, runtime, model, job, vector,
  tensor, math, geometry, vision, or UI foundation for other crates.
- Its public API is primarily an interoperability layer between retained media
  families.
- Extraction would create cross-repository coordination for normal workspace
  development or package-surface audits.

For these cases, improve crate boundaries, feature gates, package-surface
quality, or cache policy inside this repository instead.

## Extraction Sequence

Plan and execute extraction as small PRs. Do not combine dependency cleanup,
namespace migration, source removal, and consumer migration unless the family is
tiny.

1. **Readiness audit.** Use `cargo metadata --no-deps`,
   [`CRATE_INVENTORY.md`](CRATE_INVENTORY.md), and
   [`PACKAGE_SURFACE_MATRIX.md`](PACKAGE_SURFACE_MATRIX.md) to list the library
   crates, adapters, npm packages, app packages, facade exports, docs, and tests
   that belong to the candidate family.
2. **Boundary proof.** Prove the family can build from published or publishable
   foundations without depending on local `rust-packages` path crates that will
   not move with it. Shared foundations should stay here unless they are part of
   the adjacent domain.
3. **External repository bootstrap.** Create the sibling repository as a clean
   copy of the selected family, with its own workspace metadata, README, checks,
   issue labels, release notes, and package publishing plan.
4. **Namespace decision.** Decide whether extracted Rust and npm packages keep
   old names, as finance did, or move to new `moenarch-*` names, as geo did.
   Document the rule before publishing or removing local ownership.
5. **Packaging proof.** Run local packaging checks in the extracted repository.
   Do not publish from `rust-packages` as part of the proof.
6. **Migration signposts.** Add migration documentation in `rust-packages` for
   old crate names, facade modules, package names, and npm packages that users
   may still search for.
7. **Ownership removal.** Remove active source, package-surface adapters,
   generated package entries, and workspace membership from `rust-packages` only
   after the external repository owns the active implementation.
8. **Inventory refresh.** Regenerate or verify generated workspace docs only
   when workspace membership or package-surface matrices changed. Keep
   generated output out of manual edits.
9. **Follow-up cleanup.** File separate issues for stale docs, release
   publishing, npm scope migration, downstream consumer updates, and old-name
   deprecation releases.

## Required Checks

Choose the narrowest checks that match the extraction phase.

- Before extraction planning: `git status --short --branch`,
  `cargo metadata --no-deps`, and exact `rg` searches for candidate package
  names.
- Before moving code: candidate crate checks, package-surface audits, and any
  dependency graph checks needed to prove no retained crate depends on the
  moving implementation.
- After workspace membership changes:
  `python3 scripts/audit_workspace_crates.py --check`, dependency chart
  regeneration if required, and package-surface inventory checks.
- After package-surface removal or app package removal: the relevant Rust,
  WASM, package, and UI checks named by
  [`CRATE_SURFACE_AUDIT_PROTOCOL.md`](CRATE_SURFACE_AUDIT_PROTOCOL.md).
- For documentation-only playbook or migration-note updates:
  `git diff --check`. Run `bun run snapshot:check` only when reviewed generated
  docs or snapshot allowlisted files change.

## Blockers

Stop extraction planning and fix the blocker first when any of these are true:

- The candidate still exports third-party or local dependency types that would
  force consumers to depend on this workspace.
- Retained crates depend on the candidate for core contracts instead of optional
  adapters.
- Runtime, model-cache, external-tool, fixture, or frontend build output is the
  real source of pain.
- The family has not reached a usable package-surface maturity level for the
  workflows it claims to own.
- The migration path for old package names, facade modules, npm packages, or
  docs is unclear.
- The extracted repository cannot run a focused local check without private
  state from this workspace.

## Namespace Migration Steps

When a family changes package namespace during extraction, use the geo pattern:

1. Pick the replacement names before publishing, such as `moenarch-<family>`.
2. Document the legacy-to-replacement map in a focused migration note.
3. Keep active implementation only in the extracted repository after ownership
   moves.
4. Keep `rust-packages` signposts lightweight: docs, deprecated empty modules,
   or package notes only when they help users find the replacement.
5. Avoid active wrapper crates or re-export shims unless a release plan
   explicitly requires them.
6. Treat final old-name deprecation releases as manual release work, not as part
   of the source-removal PR.
7. Defer npm scope changes unless the target scope, package names, and publish
   credentials are already prepared.

When a family keeps package names, use the finance pattern: document the new
repository as the active owner and remove local implementation ownership after
the extracted repository can publish and test the same package names.

## Current Family Assessment

| Family | Extraction readiness | Notes |
| --- | --- | --- |
| Text | Not an extraction candidate now. | Text has many mature package surfaces, but it owns core document, indexing, retrieval, transcript, QA, model-runtime, and package UI workflows used across multimodal analysis. Improve boundaries and release scope inside this workspace before considering a split. |
| Audio | Not an extraction candidate now. | Audio is a primary media family with transcription, recognition, signal, TTS, MIDI, and external-tool/model integration. Cache, model, and external-tool pressure should be handled through feature gates, fixtures, and cache policy rather than repository extraction. |
| Image | Not an extraction candidate now. | Image shares visual contracts, geometry, model-runtime, OCR, detection, segmentation, captioning, processing, and ComfyUI interoperability with video and vision workflows. Extract only a clearly adjacent subfamily, not the image foundation itself. |
| Video | Should remain here. | Video is the original and still central multimodal family. It anchors facade compatibility, retained datasets, feature extraction, SFM, reconstruction, radiance, tracking, editing, and app-package workflows. Splitting video would be a broad architecture change, not a post-geo adjacent-family extraction. |

The next plausible extraction should be another adjacent domain family with a
clear standalone owner, not text, audio, image, or video as whole media groups.

## When Not To Extract

Do not extract to solve these problems:

- The checkout is large because ignored build outputs accumulated.
- A crate needs a surface audit, README cleanup, package operation grouping, or
  better tests.
- A dependency is heavy but can be feature-gated or moved behind an adapter.
- A model cache, external test fixture, or generated package output needs a
  clearer local cleanup policy.
- A family is broad and inconvenient but still acts as a foundation for retained
  multimodal workflows.

For build-cache pressure, use the local cache guidance in
[`development.md`](development.md): measure `.cargo-target`, `target`, WASM
`pkg/`, and frontend `dist/` directories, delete local caches when needed, and
keep generated outputs untracked.
