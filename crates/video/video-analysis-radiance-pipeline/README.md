# video-analysis-radiance-pipeline

Library-first project loading, validation, summaries, and CPU Gaussian preview
rendering for `video-analysis` radiance workflows.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use std::path::PathBuf;

use video_analysis_gaussian_splatting::{ProjectionConfig, SplatRenderConfig};
use video_analysis_radiance_pipeline::{
    GaussianPreviewRequest, RadianceProject, RadianceProjectPaths, RadianceViewSource,
};

let project = RadianceProject::from_paths(&RadianceProjectPaths {
    colmap_text_dir: Some(PathBuf::from("scene/colmap")),
    nerfstudio_transforms_json: Some(PathBuf::from("scene/transforms.json")),
    gaussian_splat_ply: Some(PathBuf::from("scene/splats.ply")),
})?;

let preview = project.render_gaussian_preview(&GaussianPreviewRequest {
    source: RadianceViewSource::Nerfstudio,
    view_index: 0,
    projection: ProjectionConfig::default(),
    render: SplatRenderConfig::new(640, 480)?,
    min_opacity: Some(0.1),
    downsample_stride: Some(4),
})?;

let _ = preview;
```

Preview rendering is CPU-only and always targets an explicit view source. The
crate does not currently normalize distorted COLMAP camera models into direct
ray/view conversion.
