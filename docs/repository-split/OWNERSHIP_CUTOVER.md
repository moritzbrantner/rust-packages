# Canonical destination ownership cutover

## Status

Accepted for the extracted **Rust library/package families** assigned to these destination repositories:

| Family | Canonical repository |
| --- | --- |
| foundation/data/math/runtime/vector/media contracts | `moritzbrantner/moenarch-foundation` |
| text/NLP | `moritzbrantner/nlp-stack` |
| audio | `moritzbrantner/audio-analysis` |
| image/vision/non-spatial video | `moritzbrantner/visual-analysis` |

This document changes ownership authority. It does **not** publish packages, create tags, yank versions, or delete historical source from this repository.

## Cutover rule

For a Rust package classified to one of the canonical repositories above, that destination is now the sole authority for:

- source changes and public API evolution;
- package tests and compatibility evidence;
- issue/work planning for new behavior;
- version selection and release manifests;
- future registry publication of that package.

`rust-packages` is no longer a competing source or release authority for those migrated Rust packages. Copies that remain here are compatibility/provenance material while consumers and release metadata finish migrating.

A package being physically present in this repository does not imply ownership.

## What remains valid here

`rust-packages` may continue to own packages whose reviewed target remains `rust-packages`, including compatibility/ComfyUI families, and any family that has not reached a real canonical destination repository.

Historical integration tests, migration inventories, provenance, and compatibility facades may remain until separately authorized cleanup. They must not be used as a reason to implement new domain behavior in a migrated package here.

## Release rule

A release task for a migrated package must run from its canonical destination repository and use that repository's release manifest/gates. A release issue in `rust-packages` may coordinate a landscape migration, but it must not publish a migrated package from this repository.

This ownership cutover is independent from publication readiness. A destination can be the release authority while still choosing not to publish until its package and consumer gates are satisfied.

## Development rule

Consumers should depend on released versions for normal work and may use exact source-development overrides against the canonical destination when co-developing an unreleased change. Do not create new consumer patches to a migrated package path inside `rust-packages`.

## Future source removal

Removing historical migrated source from `rust-packages` remains a separate destructive migration. It requires explicit scope and verification that compatibility consumers no longer rely on the old paths. This cutover intentionally establishes authority before deletion.
