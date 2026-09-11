# ADR 0012: Capability Repository Split And Agent-Driven Releases

## Status

Accepted, with canonical destination ownership refined by
[`OWNERSHIP_CUTOVER.md`](../repository-split/OWNERSHIP_CUTOVER.md).

This ADR supersedes only the guidance in
[ADR 0011](0011-hybrid-geo-extraction-and-namespace.md) and the
[post-pilot playbook](../POST_PILOT_EXTRACTION_PLAYBOOK.md) that rejects
capability repositories for the primary media families. The geo clean-copy,
provenance, namespace, signpost, and source-removal lessons remain in force.

## Context

The original 347-crate Rust workspace and its Bun package surfaces imposed a
broad verification and context cost on focused changes. Release ownership was
unclear, and applications consumed a mixture of registry, path, Git, file, and
shim dependencies. A split is justified by maintainability, focused agent loops,
independent release cadence, explicit public-contract ownership, and smaller
verification surfaces—not checkout or build-cache size.

The repository topology has continued to evolve after the initial split plan.
The old registry-gated `spatial-analysis` publication train was retired; spatial
work is now source-first and is being reconciled against the narrower current
repositories rather than recreating a distributed monolith.

## Decision

The proven canonical capability repositories are:

- `moritzbrantner/moenarch-foundation` for domain-neutral runtime, jobs,
  progress, cancellation, diagnostics, artifacts, model lifecycle, media/time,
  data, math, tensor, graph, geometry, signal, vector, and neutral interchange
  contracts;
- `moritzbrantner/nlp-stack` for text, lexical/linguistic analysis,
  classification, embeddings, indexing, retrieval, QA, generation, transcript
  document semantics, parsing, formatting, and NLP enrichment;
- `moritzbrantner/audio-analysis` for audio IO/analysis, recognition,
  separation, transcription execution, synthesis, MIDI, TTS, and native audio
  adapters;
- `moritzbrantner/visual-analysis` for image/vision and non-spatial video
  contracts and implementations.

For those extracted Rust families, canonical ownership has cut over according to
[`ownership-cutover.json`](../repository-split/ownership-cutover.json).
Historical copies in `rust-packages` are compatibility/provenance material and
are not competing implementation or release authorities.

Spatial ownership is deliberately not assigned wholesale here. The original
single `moritzbrantner/spatial-analysis` target is no longer authoritative.
Current renderer-independent 3D authorities exist in `moritzbrantner/3d-lab`,
while reusable reconstruction semantics exist in `moritzbrantner/video-to-3d`.
Remaining legacy spatial packages in `rust-packages` are reconciled package by
package under issue #179 before their ownership records change.

`moritzbrantner/rust-packages` remains the compatibility facade, integration
suite, incubator, migration-signpost home, cross-domain prototype home, and
owner of packages that have not yet reached a proven canonical destination.

The preferred production graph is:

```text
                     foundation
                 /       |       \
                /        |        \
              nlp      audio     visual

             adapters / applications
             may compose capabilities

rust-packages compatibility/integration -> proven capability repositories
```

Foundation depends on no domain repository. Domain capability repositories
should depend downward on foundation rather than sideways on another domain's
implementation merely to exchange data. Genuine cross-domain behavior belongs
behind an explicit adapter or application composition boundary. Reverse edges
and cycles are forbidden. Any narrower spatial dependency direction must be
proved by the owning repositories rather than inferred from the retired
`spatial-analysis` plan.

The machine ownership source, exact reviewed baseline, and checker under
`docs/repository-split/` and `scripts/check_repository_boundaries.py` enforce
these boundaries while the monolith is neutralized. Historical temporary
exceptions are migration debt, not precedent for new edges.

### Neutral contracts and cycle breaking

Issue [#108](https://github.com/moritzbrantner/rust-packages/issues/108)
established the domain-neutral media/time crate. `moenarch-media-core` owns
neutral timebases, timestamps, time ranges, generic media/source metadata,
neutral events, stream-format identifiers, and neutral timed-text interchange
DTOs. It must not own scenes, frames, audio/image buffers, NLP transcript
parsing/formatting, detections, keypoints, domain model execution, or
linguistic enrichment.

The original issue [#112](https://github.com/moritzbrantner/rust-packages/issues/112)
purified `text-transcripts` around transcript/segment/timing/speaker plus
SRT/WebVTT/Whisper JSON semantics. Later decoupling separated those concerns
further: neutral text-plus-media-timing interchange belongs in foundation,
while `nlp-stack` keeps transcript document semantics, parsing, formatting,
text-document conversion, heuristics, and NLP enrichment. Audio transcription
produces neutral media contracts; consumers select NLP only when they need NLP
behavior.

Generic probing, finite-source selection, container metadata, and audio-track
decoding must not force audio applications through visual-analysis. Generic
source metadata belongs in foundation, visual frames in visual-analysis, and
audio sample preparation in audio-analysis. A narrow neutral IO adapter is
permitted only where implementation behavior is genuinely shared.

### Names, semver, adapters, and provenance

Package names remain stable where practical. Repository movement alone is not a
breaking change. Patch releases cover compatible metadata or re-exports;
additive stable APIs use minor bumps; stable breaking APIs use major bumps at
1.x and minor bumps at 0.x. Renames require a new package plus a deliberately
versioned old-name deprecation release. One repository is the sole release
owner at every step.

Each extraction records an exact source commit, copied paths, licenses, notices,
attribution, and relevant history notes. History rewriting, force pushing,
repository deletion, and destructive source removal before destination and
consumer proof are excluded.

Focused CLI, server, WASM, npm, and app adapters remain during initial
extraction. Removing a focused adapter requires usage evidence, migration notes,
deployment analysis, and a separate compatibility decision.

### Consumer and release gates

Source development and publication are separate. Ordinary cross-repository
feature work may use exact source revisions from canonical repositories before a
registry version exists. Source-mode evidence does not authorize publication and
does not transfer authority back to `rust-packages`.

Where publication is required, destination code must pass its repository checks,
package-surface and operation-ID parity, provenance review, and appropriate
candidate-consumer checks. After publication, agents verify the exact registry
version and run the required clean-consumer evidence. Manifest inspection alone
is never reported as a passing consumer check.

A publication wave requires an exact live release authority and reviewed
machine-readable manifest for the canonical source repository. Agents may not
use administrator bypasses, publish an unspecified package, publish from a
non-canonical source repository, or publish a version absent from the release
authorization.

If a wave partially publishes, already published versions remain immutable and
are neither republished nor automatically yanked. Record the partial state, fix
the remaining package/workflow in a follow-up commit, and resume idempotently at
the first unpublished version.

### Source-removal and repository-creation gates

Canonical ownership may precede physical source removal. Source leaves
`rust-packages` only after the destination is independently proven, affected
consumers can migrate without relying on the historical path, compatibility
signposts exist where required, and rollback is documented. Registry proof is
required only when registry publication is part of the distribution contract.

Source removal, compatibility releases, and consumer migration remain separate
changes unless an exact issue proves the family tiny and the combined change
retains equivalent evidence.

A new repository should not be created merely because an old destination matrix
named one. Repository creation requires a current ownership need that cannot be
served coherently by an existing authority. Issue #179 applies this rule to the
remaining spatial family.

## Rollback

Before publication, a failed destination change is repaired or reverted at the
canonical destination; it does not silently restore competing ownership in
`rust-packages`. After publication, published artifacts are not deleted or
yanked automatically. Keep the last known-good source and compatibility
surface, stop further destructive migration, record the exact released state,
and ship a forward-compatible repair through explicit release authority.

A reverse migration of canonical ownership requires its own explicit decision.

## Consequences

Release cadence and verification become capability-scoped while ordinary
source-first development remains possible. Existing forbidden or transitional
edges remain visible migration debt with individual owners; no wildcard
exemption permits new coupling. Cross-domain applications remain free to compose
capabilities without forcing the capability repositories to behave as one
implicit distributed monorepo.
