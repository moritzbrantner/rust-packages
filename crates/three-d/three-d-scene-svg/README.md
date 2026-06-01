# three-d-scene-svg

`moritzbrantner-three-d-scene-svg` provides a small, SVG-inspired document model for
declarative 3D scenes plus a deterministic SVG preview renderer. It is meant
for diagrams, reports, generated previews, test fixtures, and compact
programmatic 3D illustrations in the `moritzbrantner-video-analysis` workspace.

This crate does not try to replace existing interchange standards:

- [X3D](https://www.web3d.org/x3d/what-x3d) is the closest existing open
  standard for SVG-like declarative 3D scenes. It has XML, Classic VRML,
  VRML97, and JSON encodings and is maintained by the Web3D Consortium.
- [glTF](https://www.khronos.org/gltf/) is the dominant runtime asset delivery
  format for efficient 3D model loading, with JSON/GLB structure, meshes,
  materials, cameras, skins, and animation.
- [OpenUSD](https://aousd.org/) is the broader scene-description ecosystem for
  production scene composition and collaboration.

The gap this crate fills is narrower: a Rust-native, serde-friendly profile for
simple declarative 3D shapes that can be validated and rendered into ordinary
SVG previews without a GPU.

See [SPEC.md](SPEC.md) for the Scene Vector 3D profile.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use three_d_processing_core::Point3;
use three_d_scene_svg::{
    render_svg, Camera, ColorRgba, Node, SceneDocument, SceneViewport, Style,
};

let scene = SceneDocument::new(
    SceneViewport::new(640, 480)?,
    Camera::orthographic(
        Point3::new(3.0, 3.0, 3.0),
        Point3::new(0.0, 0.0, 0.0),
        4.0,
    )?,
    Node::group([
        Node::line(
            Point3::new(-1.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
        )
        .style(Style::stroke(ColorRgba::rgb(220, 40, 40), 2.0)),
    ]),
)?;

let svg = render_svg(&scene)?;
assert!(svg.contains("<svg"));
```

## Related crates

- `three-d-processing-core`
- `three-d-processing-mesh`
- `three-d-processing-io`

## Package surface

Workflow operations:

- `threeD.sceneSvg.summary`
- `threeD.sceneSvg.exportSvg`

Debug operations:

- `describe`
- `threeD.sceneSvg.renderPlan`

Runtime limits:

Operations are deterministic, local-first, and side-effect free. They return inline JSON reports and do not download models, write files, or run native tools.

Invalid input returns a clear error through `run_surface_operation`; successful
responses include `operation`, `title`, `message`, `summary`, and `result` while
keeping existing top-level domain fields for compatibility.
