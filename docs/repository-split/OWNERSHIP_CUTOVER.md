# Canonical destination ownership cutover

## Status

Accepted for the extracted **Rust library/package families** assigned to these proven destination repositories:

| Family | Canonical repository |
| --- | --- |
| foundation/data/math/runtime/vector/media contracts | `moritzbrantner/moenarch-foundation` |
| text/NLP | `moritzbrantner/nlp-stack` |
| audio | `moritzbrantner/audio-analysis` |
| image/vision/non-spatial video | `moritzbrantner/visual-analysis` |

This document changes ownership authority. It does **not** publish packages, create tags, yank versions, or delete historical source from this repository.

The former single `spatial-analysis` destination is intentionally not part of this cutover. Its registry-gated extraction plan was retired. Remaining spatial-family ownership is being reconciled from the current source topology in #179, including the proven authorities in `3d-lab` and `video-to-3d`.

## Cutover rule

For a Rust package classified to one of the canonical repositories above, that destination is now the sole authority for:

- source changes and public API evolution;
- package tests and compatibility evidence;
- issue/work planning for new behavior;
- version selection and release manifests;
- future registry publication when publication is required.

`rust-packages` is no longer a competing source or release authority for those migrated Rust packages. Copies that remain here are compatibility/provenance material while consumers and release metadata finish migrating.

A package being physically present in this repository does not imply ownership.

## What remains valid here

`rust-packages` may continue to own packages whose current proven authority remains `rust-packages`, including compatibility/ComfyUI families and spatial-family packages that have not yet been reconciled by #179.

Historical integration tests, migration inventories, provenance, and compatibility facades may remain until separately authorized cleanup. They must not be used as a reason to implement new domain behavior in a migrated package here.

## Release rule

A release task for a migrated package must run from its canonical destination repository and use that repository's release authority and gates. A release issue in `rust-packages` may coordinate a landscape migration, but it must not publish a migrated package from this repository.

Canonical ownership is independent from publication readiness. A destination can own a package while ordinary source-first development proceeds without forcing a registry release.

## Development rule

Ordinary cross-repository development may consume an exact source revision from the canonical destination. Registry coordinates remain the distribution contract where packages are published, but publication is not a prerequisite for feature development or migration proof.

Do not create new consumer patches to a migrated package path inside `rust-packages`.

## Future source removal

Removing historical migrated source from `rust-packages` remains a separate destructive migration. It requires explicit scope and deterministic evidence that the canonical destination and affected consumers no longer rely on the old source location. This cutover intentionally establishes authority before deletion.
