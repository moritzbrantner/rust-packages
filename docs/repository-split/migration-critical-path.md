# Repository-split migration critical path

## Evidence

This plan was derived on 2026-07-30 from:

- `cargo metadata --format-version 1 --no-deps` at
  `d032ad2890c1df3c6a5b9eff024562f00d017fce`;
- the reviewed package ownership map merged from pull request #137 at
  `ff2c3161392c9644b631aad20e4761b690982f88`;
- the current bodies, comments, labels, and states of PRD #106 and its open
  child issues.

The dependency query started from every Cargo package assigned to `nlp-stack`
or `audio-analysis`, traversed workspace dependencies, and retained packages
assigned to `moenarch-foundation`. It found 13 foundation libraries in the NLP
closure and nine in the audio closure. The audio set is a subset of the union
below.

## Foundation release waves

### Wave 1: critical contract closure

Issue #111 publishes only the current Cargo dependency closure required to
unblock the NLP and audio package families:

- `moenarch-data-inversion-core`
- `moenarch-jobs-core`
- `moenarch-math-geometry-2d`
- `moenarch-math-linear`
- `moenarch-math-signal-core`
- `moenarch-math-sparse-data`
- `moenarch-model-runtime`
- `moenarch-numbers-core`
- `moenarch-runtime-core`
- `moenarch-runtime-onnx`
- `moenarch-tensor-data`
- `moenarch-vector-analysis-core`
- `moenarch-vector-analysis-index`

The new neutral `moenarch-media-core` contract is bootstrapped separately by
#136 before wave 1. This keeps its one-time crates.io bootstrap authorization
explicit instead of hiding it inside a multi-package release.

`moenarch-runtime-onnx` remains in wave 1 even though it is a heavy optional
runtime. Current Cargo metadata places it in both the NLP and audio transitive
closures. Deferring it would require either a partial NLP package release or a
temporary non-registry dependency. Neither is authorized by the current issues.
It can move to a later wave only after a separate slice proves that the affected
NLP/audio packages remain complete and publishable without it.

### Wave 2: reusable core primitives

Issue #142 publishes the remaining foundation core libraries that are not in
the current NLP/audio dependency closure:

- `moenarch-dense-data`
- `moenarch-graph-analysis-core`
- `moenarch-math-statistics`

Wave 2 depends on the registry-visible wave-1 versions. It does not block the
NLP bootstrap or release.

### Wave 3: deferred adapters

Issue #143 publishes the approved foundation CLI, server, and WASM adapters
after their core libraries are registry-visible. No npm package is implied.
This removes adapter multiplication from the NLP/audio critical path without
creating incomplete public libraries or temporary path/Git dependencies.

## Deterministic dependency corrections

- #108 is coordinated from `rust-packages`; naming the nonexistent foundation
  repository as its accessible owner created a cycle because #110 cannot create
  that repository until #108 and #109 complete.
- #146 precedes #108 because the immutable Phase A ownership authority needs an
  append-only post-baseline provenance schema before a new Cargo package can be
  added without falsifying the Phase A audit commit.
- #112 follows #109. Both touch `text-transcripts`, audio adapters, the root
  manifest, and the root lockfile, so they are not safe parallel slices.
- #123 owns only `native-whisperx`; #144 owns the later `subtitle-merger`
  repository mutation.
- #131 also waits for #142 because its exact data-package requirements remain
  unaudited and may include a wave-2 primitive.
- #134 waits for waves 2 and 3 and the separate subtitle-merger migration.

## Current critical path

With current issue state, the deterministic longest path is:

```text
#146 → #108 → #109 → #110 → #136 → #111 → #113 → #114
     → #118 → #119 → #120 → #121 → #129 → #134
```

The path is based on unresolved dependencies, not the total issue count. Wave 2
and wave 3 remain required for final conversion but run off the NLP/audio
release path.

## Concurrency frontier after Phase A

After #107 completed, the three initially evaluated groups were:

- #108: neutral media contracts in `rust-packages`, now dependency-blocked by
  the active non-overlapping authority-schema prerequisite #146;
- #117: scene-ownership analysis in non-overlapping decision documents,
  currently human-blocked;
- #135: the former hosted release-environment policy in `geo-analysis`, now
  closed as superseded by authoritative local Cargo publication.

#146 and #117 have disjoint write scopes and exclusive resources, but #117
remains human-blocked. A slice becomes a parallel candidate only after it is
independently dependency-ready, any human blocker is cleared, and the repository
has a current passing exact-head local verification receipt. Missing or stale
local evidence permits at most one new worker; repeated reproducible local
failures, blocking review findings, or merge conflicts permit none. GitHub
Actions state is not a scheduling input.

Later safe candidates are consumer migrations in distinct repositories after
their exact release blockers complete. Transcript purification and removal of
neutral-contract reverse dependencies are explicitly serialized because their
current write scopes overlap.

## Publication safety

All publication waves remain bound to an exact release issue and checked-in
manifest. An agent may publish locally with Cargo when the exact package and
version are authorized, the release checkout is clean and matches the exact
release commit, all required local package and consumer checks pass, the exact
registry version is absent, package contents have been inspected, and an
existing Cargo credential is available without being printed or copied.
Packages are published in dependency order and verified on crates.io after each
publication. GitHub Actions and OIDC remain optional publication mechanisms.
Published versions are never overwritten or automatically yanked.
