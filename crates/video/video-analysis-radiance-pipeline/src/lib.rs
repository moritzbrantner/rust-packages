#![doc = include_str!("../README.md")]

use std::path::PathBuf;
use std::{error, fmt};

use thiserror::Error;
use video_analysis_core::DetectError;
use video_analysis_gaussian_splatting::{
    project_scene, render_projected_splats, GaussianScene, GaussianSplatScene, ProjectionConfig,
    SplatPixel, SplatRenderConfig,
};
use video_analysis_radiance_fields::{AxisAlignedBounds, CameraViewSet};
use video_analysis_radiance_io::{
    colmap_to_sparse_reconstruction, colmap_to_view_set, inspect_colmap_camera_support,
    read_colmap_text_dir, read_gaussian_splat_ply, read_nerfstudio_transforms,
    transforms_to_view_set, ColmapCameraSupport, ColmapDataset, NerfstudioTransforms,
    RadianceIoError,
};
use video_analysis_reconstruction::SparseReconstruction;

fn invalid_argument(message: impl Into<String>) -> DetectError {
    DetectError::InvalidArgument(message.into())
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
/// Data type for radiance project paths.
pub struct RadianceProjectPaths {
    /// The COLMAP text dir value.
    pub colmap_text_dir: Option<PathBuf>,
    /// The Nerfstudio transforms JSON value.
    pub nerfstudio_transforms_json: Option<PathBuf>,
    /// The gaussian splat PLY value.
    pub gaussian_splat_ply: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for COLMAP project.
pub struct ColmapProject {
    /// The dataset value.
    pub dataset: ColmapDataset,
    /// The view set value.
    pub view_set: CameraViewSet,
    /// The sparse reconstruction value.
    pub sparse_reconstruction: SparseReconstruction,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for Nerfstudio project.
pub struct NerfstudioProject {
    /// The transforms value.
    pub transforms: NerfstudioTransforms,
    /// The view set value.
    pub view_set: CameraViewSet,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for radiance project.
pub struct RadianceProject {
    /// The COLMAP value.
    pub colmap: Option<ColmapProject>,
    /// The Nerfstudio value.
    pub nerfstudio: Option<NerfstudioProject>,
    /// The gaussian splats value.
    pub gaussian_splats: Option<GaussianSplatScene>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Variants describing radiance view source.
pub enum RadianceViewSource {
    /// The COLMAP variant.
    Colmap,
    /// The Nerfstudio variant.
    Nerfstudio,
}

impl fmt::Display for RadianceViewSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Colmap => f.write_str("Colmap"),
            Self::Nerfstudio => f.write_str("Nerfstudio"),
        }
    }
}

impl error::Error for RadianceViewSource {}

#[derive(Debug, Clone, PartialEq)]
/// Data type for radiance project summary.
pub struct RadianceProjectSummary {
    /// The available view sources value.
    pub available_view_sources: Vec<RadianceViewSource>,
    /// The COLMAP camera count value.
    pub colmap_camera_count: usize,
    /// The COLMAP image count value.
    pub colmap_image_count: usize,
    /// The COLMAP point count value.
    pub colmap_point_count: usize,
    /// The Nerfstudio frame count value.
    pub nerfstudio_frame_count: usize,
    /// The gaussian splat count value.
    pub gaussian_splat_count: usize,
    /// The camera center bounds value.
    pub camera_center_bounds: Option<AxisAlignedBounds>,
    /// The gaussian bounds value.
    pub gaussian_bounds: Option<AxisAlignedBounds>,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for gaussian preview request.
pub struct GaussianPreviewRequest {
    /// The source value.
    pub source: RadianceViewSource,
    /// The view index value.
    pub view_index: usize,
    /// The projection value.
    pub projection: ProjectionConfig,
    /// The render value.
    pub render: SplatRenderConfig,
    /// The min opacity value.
    pub min_opacity: Option<f32>,
    /// The downsample stride value.
    pub downsample_stride: Option<usize>,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for gaussian preview image.
pub struct GaussianPreviewImage {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// The pixels value.
    pub pixels: Vec<SplatPixel>,
}

#[derive(Debug, Error)]
/// Variants describing radiance pipeline error.
pub enum RadiancePipelineError {
    #[error("I/O error: {0}")]
    /// The I/O variant.
    Io(#[from] std::io::Error),
    #[error("{0}")]
    /// The detect variant.
    Detect(#[from] DetectError),
    #[error("{0}")]
    /// The radiance I/O variant.
    RadianceIo(#[from] RadianceIoError),
    #[error("at least one radiance input path must be provided")]
    /// The missing inputs variant.
    MissingInputs,
    #[error("missing requested view source: {0:?}")]
    /// The missing view source variant.
    MissingViewSource(RadianceViewSource),
    #[error("missing Gaussian splats")]
    /// The missing gaussian splats variant.
    MissingGaussianSplats,
    #[error(
        "requested {source:?} view index {requested}, but only {available} view(s) are available"
    )]
    /// The view index out of range variant.
    ViewIndexOutOfRange {
        /// Source value for this variant.
        source: RadianceViewSource,
        /// The requested value for this variant.
        requested: usize,
        /// The available value for this variant.
        available: usize,
    },
    #[error("unsupported COLMAP camera models: {0:?}")]
    /// The unsupported COLMAP camera models variant.
    UnsupportedColmapCameraModels(Vec<ColmapCameraSupport>),
}

impl RadianceProject {
    /// Builds this value from paths.
    pub fn from_paths(paths: &RadianceProjectPaths) -> Result<Self, RadiancePipelineError> {
        if paths.colmap_text_dir.is_none()
            && paths.nerfstudio_transforms_json.is_none()
            && paths.gaussian_splat_ply.is_none()
        {
            return Err(RadiancePipelineError::MissingInputs);
        }

        let colmap = if let Some(path) = &paths.colmap_text_dir {
            let dataset = read_colmap_text_dir(path)?;
            let unsupported = unsupported_colmap_camera_models(&dataset);
            if !unsupported.is_empty() {
                return Err(RadiancePipelineError::UnsupportedColmapCameraModels(
                    unsupported,
                ));
            }
            let view_set = colmap_to_view_set(&dataset)?;
            let sparse_reconstruction = colmap_to_sparse_reconstruction(&dataset)?;
            Some(ColmapProject {
                dataset,
                view_set,
                sparse_reconstruction,
            })
        } else {
            None
        };

        let nerfstudio = if let Some(path) = &paths.nerfstudio_transforms_json {
            let transforms = read_nerfstudio_transforms(path)?;
            let view_set = transforms_to_view_set(&transforms)?;
            Some(NerfstudioProject {
                transforms,
                view_set,
            })
        } else {
            None
        };

        let gaussian_splats = if let Some(path) = &paths.gaussian_splat_ply {
            Some(read_gaussian_splat_ply(path)?)
        } else {
            None
        };

        Ok(Self {
            colmap,
            nerfstudio,
            gaussian_splats,
        })
    }

    /// Validates this value.
    pub fn validate(&self) -> Result<(), RadiancePipelineError> {
        if let Some(colmap) = &self.colmap {
            validate_colmap_project(colmap)?;
        }
        if let Some(nerfstudio) = &self.nerfstudio {
            validate_nerfstudio_project(nerfstudio)?;
        }
        if let Some(gaussian_splats) = &self.gaussian_splats {
            gaussian_splats.validate()?;
        }
        Ok(())
    }

    /// Returns summary.
    pub fn summary(&self) -> Result<RadianceProjectSummary, RadiancePipelineError> {
        self.validate()?;

        let mut available_view_sources = Vec::new();
        if self.colmap.is_some() {
            available_view_sources.push(RadianceViewSource::Colmap);
        }
        if self.nerfstudio.is_some() {
            available_view_sources.push(RadianceViewSource::Nerfstudio);
        }

        let camera_center_bounds = if let Some(colmap) = &self.colmap {
            colmap.view_set.camera_center_bounds()?
        } else if let Some(nerfstudio) = &self.nerfstudio {
            nerfstudio.view_set.camera_center_bounds()?
        } else {
            None
        };

        let gaussian_bounds = self
            .gaussian_splats
            .as_ref()
            .map(|scene| scene.stats())
            .transpose()?
            .and_then(|stats| stats.bounds);

        Ok(RadianceProjectSummary {
            available_view_sources,
            colmap_camera_count: self
                .colmap
                .as_ref()
                .map_or(0, |colmap| colmap.dataset.cameras.len()),
            colmap_image_count: self
                .colmap
                .as_ref()
                .map_or(0, |colmap| colmap.dataset.images.len()),
            colmap_point_count: self
                .colmap
                .as_ref()
                .map_or(0, |colmap| colmap.dataset.points.len()),
            nerfstudio_frame_count: self
                .nerfstudio
                .as_ref()
                .map_or(0, |nerfstudio| nerfstudio.transforms.frames.len()),
            gaussian_splat_count: self
                .gaussian_splats
                .as_ref()
                .map_or(0, |scene| scene.splats.len()),
            camera_center_bounds,
            gaussian_bounds,
        })
    }

    /// Returns render gaussian preview.
    pub fn render_gaussian_preview(
        &self,
        request: &GaussianPreviewRequest,
    ) -> Result<GaussianPreviewImage, RadiancePipelineError> {
        self.validate()?;

        let view_set = self.view_set_for_source(request.source)?;
        let view = view_set.views.get(request.view_index).ok_or(
            RadiancePipelineError::ViewIndexOutOfRange {
                source: request.source,
                requested: request.view_index,
                available: view_set.views.len(),
            },
        )?;

        let mut splat_scene = self
            .gaussian_splats
            .clone()
            .ok_or(RadiancePipelineError::MissingGaussianSplats)?;

        if let Some(min_opacity) = request.min_opacity {
            splat_scene.retain_opacity_at_least(min_opacity)?;
        }
        if let Some(stride) = request.downsample_stride {
            splat_scene = splat_scene.downsample_stride(stride)?;
        }

        let preview_scene = GaussianScene::new(
            splat_scene
                .splats
                .iter()
                .map(|splat| splat.to_preview_gaussian())
                .collect::<Result<Vec<_>, _>>()?,
        )?;
        let projected = project_scene(
            &preview_scene,
            view.intrinsics,
            view.pose,
            request.projection,
        )?;
        let pixels = render_projected_splats(&projected, request.render)?;

        Ok(GaussianPreviewImage {
            width: request.render.width,
            height: request.render.height,
            pixels,
        })
    }

    fn view_set_for_source(
        &self,
        source: RadianceViewSource,
    ) -> Result<&CameraViewSet, RadiancePipelineError> {
        match source {
            RadianceViewSource::Colmap => self
                .colmap
                .as_ref()
                .map(|project| &project.view_set)
                .ok_or(RadiancePipelineError::MissingViewSource(source)),
            RadianceViewSource::Nerfstudio => self
                .nerfstudio
                .as_ref()
                .map(|project| &project.view_set)
                .ok_or(RadiancePipelineError::MissingViewSource(source)),
        }
    }
}

fn unsupported_colmap_camera_models(dataset: &ColmapDataset) -> Vec<ColmapCameraSupport> {
    inspect_colmap_camera_support(dataset)
        .into_iter()
        .filter(|support| {
            !support.supported_for_view_conversion
                || !support.supported_for_reconstruction_conversion
        })
        .collect()
}

fn validate_colmap_project(project: &ColmapProject) -> Result<(), RadiancePipelineError> {
    let unsupported = unsupported_colmap_camera_models(&project.dataset);
    if !unsupported.is_empty() {
        return Err(RadiancePipelineError::UnsupportedColmapCameraModels(
            unsupported,
        ));
    }

    let expected_view_set = colmap_to_view_set(&project.dataset)?;
    if project.view_set != expected_view_set {
        return Err(invalid_argument(
            "stored COLMAP view set does not match the loaded dataset conversion",
        )
        .into());
    }
    project.view_set.validate()?;

    let expected_reconstruction = colmap_to_sparse_reconstruction(&project.dataset)?;
    if project.sparse_reconstruction != expected_reconstruction {
        return Err(invalid_argument(
            "stored COLMAP sparse reconstruction does not match the loaded dataset conversion",
        )
        .into());
    }
    for camera in project.sparse_reconstruction.cameras().values() {
        camera.intrinsics.validate()?;
    }
    for image in project.sparse_reconstruction.images().values() {
        image.validate()?;
    }
    for point in project.sparse_reconstruction.points().values() {
        point.validate()?;
    }

    Ok(())
}

fn validate_nerfstudio_project(project: &NerfstudioProject) -> Result<(), RadiancePipelineError> {
    let expected_view_set = transforms_to_view_set(&project.transforms)?;
    if project.view_set != expected_view_set {
        return Err(invalid_argument(
            "stored Nerfstudio view set does not match the loaded transforms conversion",
        )
        .into());
    }
    project.view_set.validate()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use video_analysis_gaussian_splatting::{GaussianSplat3d, Quaternion, SphericalHarmonicsRgb};
    use video_analysis_radiance_fields::{CameraModel, ColorRgb, Vec3};
    use video_analysis_radiance_io::{
        ColmapCamera, ColmapImage, ColmapPoint3d, ColmapTrackElement, NerfstudioFrame,
    };

    fn minimal_colmap_project() -> ColmapProject {
        let dataset = ColmapDataset {
            cameras: vec![ColmapCamera {
                id: 1,
                model: CameraModel::Pinhole,
                raw_model: "PINHOLE".to_string(),
                width: 64,
                height: 48,
                params: vec![50.0, 50.0, 32.0, 24.0],
            }],
            images: vec![ColmapImage {
                id: 1,
                qw: 1.0,
                qx: 0.0,
                qy: 0.0,
                qz: 0.0,
                tx: 0.0,
                ty: 0.0,
                tz: 0.0,
                camera_id: 1,
                name: "frame_0001.png".to_string(),
                points2d: Vec::new(),
            }],
            points: vec![ColmapPoint3d {
                id: 1,
                xyz: Vec3::new(0.0, 0.0, 1.0),
                color: ColorRgb::new(1.0, 0.0, 0.0),
                error: 0.1,
                track: vec![ColmapTrackElement {
                    image_id: 1,
                    point2d_index: 0,
                }],
            }],
        };

        ColmapProject {
            view_set: colmap_to_view_set(&dataset).unwrap(),
            sparse_reconstruction: colmap_to_sparse_reconstruction(&dataset).unwrap(),
            dataset,
        }
    }

    fn minimal_nerfstudio_project() -> NerfstudioProject {
        let transforms = NerfstudioTransforms {
            camera_model: Some("PINHOLE".to_string()),
            fl_x: Some(50.0),
            fl_y: Some(50.0),
            cx: Some(32.0),
            cy: Some(24.0),
            w: Some(64),
            h: Some(48),
            frames: vec![NerfstudioFrame {
                file_path: "images/frame_0001.png".to_string(),
                transform_matrix: [
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, -1.0, 0.0],
                    [0.0, 0.0, 0.0, 1.0],
                ],
                fl_x: None,
                fl_y: None,
                cx: None,
                cy: None,
                w: None,
                h: None,
            }],
        };

        NerfstudioProject {
            view_set: transforms_to_view_set(&transforms).unwrap(),
            transforms,
        }
    }

    fn sample_splats(opacity_logit: f32) -> GaussianSplatScene {
        GaussianSplatScene {
            splats: vec![
                GaussianSplat3d {
                    mean: Vec3::new(0.0, 0.0, 1.0),
                    scale_log: Vec3::new(-1.5, -1.5, -1.5),
                    rotation: Quaternion::IDENTITY,
                    opacity_logit,
                    sh: SphericalHarmonicsRgb::dc(ColorRgb::WHITE),
                },
                GaussianSplat3d {
                    mean: Vec3::new(0.2, 0.0, 1.0),
                    scale_log: Vec3::new(-1.5, -1.5, -1.5),
                    rotation: Quaternion::IDENTITY,
                    opacity_logit,
                    sh: SphericalHarmonicsRgb::dc(ColorRgb::new(0.0, 1.0, 0.0)),
                },
                GaussianSplat3d {
                    mean: Vec3::new(-0.2, 0.0, 1.0),
                    scale_log: Vec3::new(-1.5, -1.5, -1.5),
                    rotation: Quaternion::IDENTITY,
                    opacity_logit,
                    sh: SphericalHarmonicsRgb::dc(ColorRgb::new(1.0, 0.0, 0.0)),
                },
            ],
        }
    }

    fn preview_request() -> GaussianPreviewRequest {
        GaussianPreviewRequest {
            source: RadianceViewSource::Nerfstudio,
            view_index: 0,
            projection: ProjectionConfig::default(),
            render: SplatRenderConfig::new(32, 24).unwrap(),
            min_opacity: None,
            downsample_stride: None,
        }
    }

    fn alpha_sum(image: &GaussianPreviewImage) -> f32 {
        image.pixels.iter().map(|pixel| pixel.alpha).sum()
    }

    #[test]
    fn from_paths_rejects_all_none() {
        let error = RadianceProject::from_paths(&RadianceProjectPaths::default()).unwrap_err();
        assert!(matches!(error, RadiancePipelineError::MissingInputs));
    }

    #[test]
    fn summary_is_zeroed_for_absent_sources() {
        let summary = RadianceProject {
            colmap: None,
            nerfstudio: None,
            gaussian_splats: None,
        }
        .summary()
        .unwrap();

        assert!(summary.available_view_sources.is_empty());
        assert_eq!(summary.colmap_camera_count, 0);
        assert_eq!(summary.colmap_image_count, 0);
        assert_eq!(summary.colmap_point_count, 0);
        assert_eq!(summary.nerfstudio_frame_count, 0);
        assert_eq!(summary.gaussian_splat_count, 0);
        assert_eq!(summary.camera_center_bounds, None);
        assert_eq!(summary.gaussian_bounds, None);
    }

    #[test]
    fn render_preview_requires_gaussian_scene() {
        let error = RadianceProject {
            colmap: None,
            nerfstudio: Some(minimal_nerfstudio_project()),
            gaussian_splats: None,
        }
        .render_gaussian_preview(&preview_request())
        .unwrap_err();

        assert!(matches!(
            error,
            RadiancePipelineError::MissingGaussianSplats
        ));
    }

    #[test]
    fn render_preview_requires_requested_view_source() {
        let error = RadianceProject {
            colmap: None,
            nerfstudio: Some(minimal_nerfstudio_project()),
            gaussian_splats: Some(sample_splats(5.0)),
        }
        .render_gaussian_preview(&GaussianPreviewRequest {
            source: RadianceViewSource::Colmap,
            ..preview_request()
        })
        .unwrap_err();

        assert!(matches!(
            error,
            RadiancePipelineError::MissingViewSource(RadianceViewSource::Colmap)
        ));
    }

    #[test]
    fn render_preview_rejects_out_of_range_view_index() {
        let error = RadianceProject {
            colmap: None,
            nerfstudio: Some(minimal_nerfstudio_project()),
            gaussian_splats: Some(sample_splats(5.0)),
        }
        .render_gaussian_preview(&GaussianPreviewRequest {
            view_index: 1,
            ..preview_request()
        })
        .unwrap_err();

        assert!(matches!(
            error,
            RadiancePipelineError::ViewIndexOutOfRange {
                source: RadianceViewSource::Nerfstudio,
                requested: 1,
                available: 1,
            }
        ));
    }

    #[test]
    fn render_preview_applies_opacity_filter() {
        let project = RadianceProject {
            colmap: None,
            nerfstudio: Some(minimal_nerfstudio_project()),
            gaussian_splats: Some(sample_splats(-8.0)),
        };

        let unfiltered = project.render_gaussian_preview(&preview_request()).unwrap();
        let filtered = project
            .render_gaussian_preview(&GaussianPreviewRequest {
                min_opacity: Some(0.9),
                ..preview_request()
            })
            .unwrap();

        assert!(alpha_sum(&unfiltered) > 0.0);
        assert_eq!(alpha_sum(&filtered), 0.0);
    }

    #[test]
    fn render_preview_applies_downsample_stride() {
        let project = RadianceProject {
            colmap: None,
            nerfstudio: Some(minimal_nerfstudio_project()),
            gaussian_splats: Some(sample_splats(5.0)),
        };

        let full = project.render_gaussian_preview(&preview_request()).unwrap();
        let downsampled = project
            .render_gaussian_preview(&GaussianPreviewRequest {
                downsample_stride: Some(2),
                ..preview_request()
            })
            .unwrap();

        assert!(alpha_sum(&full) > 0.0);
        assert!(alpha_sum(&downsampled) > 0.0);
        assert_ne!(full.pixels, downsampled.pixels);
    }

    #[test]
    fn validate_accepts_partial_projects() {
        RadianceProject {
            colmap: None,
            nerfstudio: Some(minimal_nerfstudio_project()),
            gaussian_splats: None,
        }
        .validate()
        .unwrap();
    }

    #[test]
    fn validate_accepts_colmap_partial_project() {
        RadianceProject {
            colmap: Some(minimal_colmap_project()),
            nerfstudio: None,
            gaussian_splats: None,
        }
        .validate()
        .unwrap();
    }
}
