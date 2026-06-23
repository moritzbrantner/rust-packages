# Geo Migration

The old Rust crates published under `moritzbrantner-geo-*` are being
superseded by `moenarch-geo-*` crates. Active implementation ownership for the
geo package family has moved to
[`moritzbrantner/geo-analysis`](https://github.com/moritzbrantner/geo-analysis).

This repository keeps migration signposts for Rust package consumers. It is not
adding active compatibility wrappers, re-export shims, deprecated modules, or
runtime compatibility layers for the old crate names.

## Rust Crate Map

| Legacy crate | Replacement crate |
| --- | --- |
| `moritzbrantner-geo-core` | `moenarch-geo-core` |
| `moritzbrantner-geo-io-geojson` | `moenarch-geo-io-geojson` |
| `moritzbrantner-geo-io-osm` | `moenarch-geo-io-osm` |
| `moritzbrantner-geo-clustering` | `moenarch-geo-clustering` |
| `moritzbrantner-geo-viz` | `moenarch-geo-viz` |
| `moritzbrantner-maps-kernels-core` | `moenarch-maps-kernels-core` |

## Deprecation Release Preparation

Where crates.io ownership and release state allow it, the final old-name Rust
crate releases should be documentation-only deprecation releases that point to
the corresponding `moenarch-*` crate. Those releases should not carry active
wrapper implementation or new compatibility behavior.

No release publishing is performed by this migration note. Publishing a final
old-name deprecation release remains a manual release action.

## npm Package Migration

npm package migration is deferred. Setting up npm publishing, npm `@moenarch`
scope ownership, and frontend package migration is not part of this Rust-focused
migration path.
