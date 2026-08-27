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

The 347-crate Rust workspace and its 173 Bun package surfaces impose a broad
verification and context cost on focused changes. Release ownership is unclear,
and applications consume a mixture of registry, path, Git, file, and shim
dependencies. A split is justified by maintainability, focused agent loops,
independent release cadence, explicit public-contract ownership, and smaller
verification surfaces—not checkout or build-cache size.

## Decision

Split by capability:

- `moritzbrantner/moenarch-foundation` owns domain-neutral runtime, job,
  progress, cancellation, diagnostic, artifact, model-lifecycle, media/time,
  data, math, tensor, graph, geometry, signal, vector, and neutral timed-text
  interchange contracts.
- `moritzbrantner/nlp-stack` owns text, lexical/linguistic analysis,
  classification, embeddings, indexing, retrieval, QA, generation, transcript
  document semantics, parsing, formatting, and NLP enrichment.
- `moritzbrantner/audio-analysis` owns audio IO/analysis, recognition,
  separation, transcription execution, synthesis, MIDI, TTS, and native audio
  adapters.
- `moritzbrantner/visual-analysis` owns image/vision and non-spatial video
  contracts and implementations.
- `moritzbrantner/spatial-analysis` owns 3D, animation, posture, SFM, MVS,
  reconstruction, radiance fields, and Gaussian splatting.
- `moritzbrantner/rust-packages` becomes the compatibility facade, integration
  suite, incubator, migration-signpost home, cross-domain prototype home, and
  temporary ComfyUI owner.

The preferred production graph is:

```text
                  foundation
             /        |        \
            /         |         \
          nlp       audio      visual
                                   ↑
                                   └── spatial

          adapters / applications
          may compose multiple domains

rust-packages compatibility/integration ─→ released capability repositories
```

Foundation depends on no target repository. Domain capability repositories
should depend downward on foundation rather than sideways on another domain's
implementation merely to exchange data. Genuine cross-domain behavior belongs
behind an explicit adapter or application composition boundary. Spatial may
depend on foundation and visual where the spatial capability genuinely builds
on visual data/algorithms. Reverse edges and cycles are forbidden.

The machine ownership source, exact reviewed baseline, and checker under
`docs/repository-split/` and `scripts/check_repository_boundaries.py` enforce
this direction while the current monolith is neutralized. The later dependency
architecture policy may be stricter than historical temporary exceptions in the
original extraction inventory; those exceptions are migration debt, not
precedent for new edges.

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
SRT/WebVTT/Whisper JSON semantics. The later decoupling refinement separates
those concerns further: neutral text-plus-media-timing interchange belongs in
foundation, while `nlp-stack` keeps transcript document semantics, parsing,
formatting, text-document conversion, heuristics, and NLP enrichment. Audio
transcription produces the neutral media contract; consumers select NLP only
when they need NLP behavior.

Generic probing, finite-source selection, container metadata, and audio-track
decoding must not force audio applications through visual-analysis. Generic
source metadata belongs in foundation, visual frames in visual-analysis, and
audio sample preparation in audio-analysis. A narrow neutral IO/FFmpeg adapter
is permitted only where implementation behavior is genuinely shared.

### Names, semver, adapters, and provenance

Package names remain stable where practical. Repository movement alone is not a
breaking change. Patch releases cover compatible metadata or re-exports;
additive stable APIs use minor bumps; stable breaking APIs use major bumps at
1.x and minor bumps at 0.x. Renames require a new package plus a deliberately
versioned old-name deprecation release. One repository is the sole release
owner at every step.

For the Cargo families covered by
[`ownership-cutover.json`](../repository-split/ownership-cutover.json), the
canonical destination repository now owns source changes, tests, issues,
version selection, release manifests, and future publication. Historical copies
remaining in `rust-packages` are compatibility/provenance material and do not
retain competing release authority merely because source removal is not yet
complete.

Each extraction is a clean copy from an exact source commit. The destination
records every copied path, licenses, notices, attribution, and relevant history
notes and starts a focused history. History rewriting, force pushing, repository
deletion, and source removal before release proof are excluded.

Focused CLI, server, WASM, npm, and app adapters remain during initial
extraction. A repository-level registry may be piloted additively. Removing a
focused adapter requires usage evidence, migration notes, deployment analysis,
and a separate semver/release decision.

### Consumer and release gates

Before publication, destination code must pass independent clean-checkout
builds, its repository checks, `cargo package`, package-surface/operation-ID
parity, provenance review, and candidate consumer checks using temporary,
uncommitted patches. After publication, agents verify the exact registry
version, resolve it in a clean consumer without patches, run the narrow consumer
check, and create a repository-scoped update PR. Manifest inspection is never
reported as a passing consumer check.

Source development and publication are separate. A consumer may validate an
exact destination source revision before the corresponding registry version
exists. That source-mode evidence does not authorize publication and does not
transfer release authority back to `rust-packages`.

Each publication wave is authorized by this ADR, its exact destination-local or
migration release issue, and a reviewed machine release manifest. Agents may
choose documented semver bumps, open/merge ordinary release PRs when gates
permit, publish, verify, tag, create GitHub Releases, and open consumer PRs
without another confirmation when that exact release authority exists. They may
not use administrator bypasses, publish an unspecified package, publish from a
non-canonical source repository, or publish a version absent from the
authorization.

Agents may publish locally through Cargo's already-configured credential.
GitHub Actions/OIDC trusted publishing is an optional alternative, not a
prerequisite. Before either path, the validator fetches the exact live release
issue and binds its structured authorization to the repository, issue URL,
immutable source/base SHAs, required checks, and exact package versions. The
publisher packages and publishes topologically, verifies each registry version,
resumes idempotently from the first unpublished crate, and tags only
registry-verified versions. Credential values are never inspected, printed,
copied, placed in arguments, or logged.

If a wave partially publishes, already published versions remain immutable and
are neither republished nor automatically yanked. Record the partial state, fix
the remaining package/workflow in a follow-up commit, and resume at the first
unpublished version. Downstream constraints wait for the required closure.

### Source-removal and repository-creation gates

Source leaves `rust-packages` only after the destination is independently green,
required crates are verified on the registry where publication is required,
consumer migration is possible, compatibility signposts exist, the facade can
consume released crates, and rollback is documented. Canonical ownership may
therefore precede physical source removal. Source removal, deprecation releases,
and consumer migration are separate PRs unless an issue proves the family tiny.

An agent may create a named target repository only when its issue specifies the
exact `moritzbrantner` repository and visibility and authenticated permissions
are sufficient. Unspecified visibility defaults private. Existing repository
visibility does not change under this ADR.

## Rollback

Before registry publication, a failed destination release attempt stops at the
canonical destination: close or repair the release PR and retain the last
known-good published consumer graph. Do not restore competing release ownership
to `rust-packages` merely because publication has not happened yet.

After publication, published artifacts are not deleted or yanked automatically:
keep the last known-good source and compatibility facade, stop further source
removal and consumer updates, record the exact released state, and ship a
forward-compatible repair through a new authorized release. A reverse migration
of canonical ownership requires its own ADR and explicit migration authority.

## Consequences

Release cadence and verification become capability-scoped, while neutralization
and registry-first consumer proof add deliberate sequencing. Existing forbidden
or transitional edges remain visible migration debt with individual owners and
phases; no wildcard exemption permits new coupling. Cross-domain applications
remain free to compose capabilities, but capability repositories no longer need
to behave like one implicit distributed monorepo.
