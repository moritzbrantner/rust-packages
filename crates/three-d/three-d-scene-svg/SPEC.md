# Scene Vector 3D Profile 0.1

Scene Vector 3D is an SVG-inspired profile for compact, declarative 3D scene
documents. It is a workspace-level profile, not a replacement for X3D, glTF, or
OpenUSD.

## Goals

- Keep generated 3D diagrams as editable structured data.
- Use a simple retained scene graph with groups, transforms, styles, and typed
  primitives.
- Preserve deterministic rendering for tests and reports.
- Keep documents serde-friendly and suitable for JSON storage.
- Leave high-fidelity model delivery to glTF and full declarative web 3D to
  X3D.

## Non-Goals

- No physics, shader language, scripting, browser DOM, events, skeletal
  animation, texture streaming, or binary geometry buffers.
- No guarantee of visual parity with GPU engines.
- No claim that the profile is an industry standard.

## Document

A document is JSON-compatible structured data with these fields:

- `schema`: must be `scene-vector-3d`.
- `version`: the profile version, currently `0.1`.
- `title`: optional human-readable title.
- `viewport`: SVG preview output dimensions in pixels.
- `camera`: orthographic or perspective camera.
- `root`: the root scene node.

## Coordinates

The profile uses a right-handed coordinate system. Units are arbitrary but must
be consistent within a document. Numeric values must be finite.

## Cameras

Orthographic cameras define `eye`, `target`, `up`, and vertical `scale` in
world units. Perspective cameras define `eye`, `target`, `up`, vertical field
of view in degrees, `near`, and `far` clip distances.

## Nodes

All nodes may carry an optional `id`, local `transform`, and local `style`.
Groups contain child nodes. Primitive nodes are:

- `point`: a 3D point rendered as a circle in preview SVG.
- `line`: a segment between two 3D points.
- `polyline`: an ordered list of points, optionally closed.
- `mesh`: indexed triangles with inline vertices.
- `box`: axis-aligned box defined by center and size.
- `sphere`: center and radius, rendered as a projected circle preview.

## Styling

Style is intentionally close to static SVG paint:

- `fill`: optional RGBA color.
- `stroke`: optional RGBA color.
- `stroke_width`: positive preview width in pixels.
- `opacity`: optional multiplier from `0.0` through `1.0`.
- `visible`: optional boolean.

Child nodes inherit style from parent groups unless they override it.

## Rendering Semantics

Preview rendering projects primitives through the document camera and emits SVG.
The renderer uses painter-style depth sorting for surfaces and stable input
order for equal depths. Perspective clipping is conservative: primitives with
vertices outside the near/far range may be skipped instead of clipped.

## Interop Guidance

Use X3D when browser-integrated declarative 3D, interaction, or the ISO/Web3D
ecosystem is the target. Use glTF/GLB for runtime 3D asset delivery. Use
OpenUSD for production scene composition. Use this profile for lightweight Rust
scene fixtures and SVG report previews.
