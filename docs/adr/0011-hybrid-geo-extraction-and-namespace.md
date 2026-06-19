# ADR 0011: Hybrid Geo Extraction And Namespace

## Status

Accepted.

## Context

The workspace has grown into many reusable package families, but its immediate
local disk-pressure problem is build-cache growth rather than source checkout
size. A full split by media type would move source files without proving that
the dominant `.cargo-target`, `target`, WASM, and frontend build artifacts are
better managed. It would also force many crate-boundary and publishing decisions
at once.

The geo/map data crates are a better pilot for extraction. They are useful to
multimodal workflows, but their primary ownership is an adjacent map-data
domain. They have clear package-surface boundaries, explicit foundation
dependencies, and visible old-name migration concerns.

The finance extraction already established a clean-copy pattern for adjacent
package families: copy the coherent package family into a sibling repository
with one new repository history, publish from there, and keep only migration
signposts in `rust-packages` once active implementation ownership moves out.

## Decision

Use a hybrid extraction strategy instead of a full media-type repository split.
`rust-packages` remains the home for shared multimodal foundations, audited
package-surface contracts, and migration signposts. Adjacent package families
may move out when they have coherent ownership, clear package boundaries, and a
publishable external path.

Pilot that strategy with the geo/map package family. The target extracted
repository is `moenarch/geo-analysis`, and the target publisher namespace for
new extracted packages is `moenarch-*`, starting with names such as
`moenarch-geo-core`.

Use clean-copy history for the geo pilot. The extracted repository gets a new
repository history containing the selected geo package family, rather than a
history-rewrite split of this workspace. Old `moritzbrantner-geo-*` package
names should become legacy package signposts after active implementation
ownership moves to the extracted family.

## Consequences

- Extraction decisions stay package-family-specific instead of forcing a
  repo-wide media-type split.
- Geo/map extraction proves the external repository, package namespace,
  foundation dependencies, and migration-signpost model before other families
  are considered.
- Local build-cache cleanup remains a contributor-environment concern, not the
  architectural justification for splitting source repositories.
- `rust-packages` keeps shared foundations and compatibility signposts while
  extracted adjacent package families can publish under the `moenarch-*`
  namespace on their own release cadence.
