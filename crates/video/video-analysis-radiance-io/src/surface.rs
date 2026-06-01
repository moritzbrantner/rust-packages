//! Library-owned runtime surface for `video-analysis-radiance-io`.

use runtime_core::{
    OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation, SurfaceRequest,
    SurfaceResponse,
};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use video_analysis_gaussian_splatting::{
    GaussianSplat3d, GaussianSplatScene, Quaternion, SphericalHarmonicsRgb,
};
use video_analysis_radiance_fields::{CameraModel, ColorRgb, Vec2, Vec3};

use crate::{
    inspect_colmap_camera_support, ColmapCamera, ColmapDataset, ColmapImage, ColmapPoint2d,
    ColmapPoint3d, ColmapTrackElement,
};

/// Returns the package surface exposed by every transport wrapper.
pub fn package_surface() -> PackageSurface {
    PackageSurface {
        library: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: RuntimeCapabilities::pure_rust(),
        operations: vec![
            operation(
                "describe",
                "Describe package",
                "COLMAP, Nerfstudio, and PLY I/O for video-analysis radiance workflows.",
                serde_json::json!({"type": "object", "additionalProperties": true}),
                serde_json::json!({
                    "type": "object",
                    "required": ["library", "version", "operationCount"]
                }),
                serde_json::json!({"includeOperations": true}),
            ),
            operation(
                "radiance.io.colmapCameraSupport",
                "COLMAP camera support",
                "Inspect which COLMAP camera models can be converted by the current pure Rust helpers.",
                serde_json::json!({
                    "type": "object",
                    "required": ["dataset"],
                    "properties": {"dataset": {"type": "object"}}
                }),
                serde_json::json!({"type": "object", "required": ["cameras"]}),
                serde_json::json!({
                    "dataset": {
                        "cameras": [{"id": 1, "rawModel": "PINHOLE", "width": 640, "height": 480, "params": [500, 500, 320, 240]}],
                        "images": [],
                        "points3d": []
                    }
                }),
            ),
            operation(
                "radiance.io.colmapSummary",
                "COLMAP summary",
                "Summarize COLMAP camera, image, and point counts with camera support totals.",
                serde_json::json!({
                    "type": "object",
                    "required": ["dataset"],
                    "properties": {"dataset": {"type": "object"}}
                }),
                serde_json::json!({
                    "type": "object",
                    "required": ["cameraCount", "imageCount", "pointCount"]
                }),
                serde_json::json!({"dataset": {"cameras": [], "images": [], "points3d": []}}),
            ),
            operation(
                "radiance.io.gaussianSplatSummary",
                "Gaussian splat summary",
                "Summarize an in-memory Gaussian splat scene without reading model files.",
                serde_json::json!({
                    "type": "object",
                    "required": ["scene"],
                    "properties": {"scene": {"type": "object"}}
                }),
                serde_json::json!({"type": "object", "required": ["primitiveCount"]}),
                serde_json::json!({"scene": {"splats": []}}),
            ),
        ],
    }
}

/// Runs one library-owned operation.
pub fn run_surface_operation(request: SurfaceRequest) -> Result<SurfaceResponse, String> {
    match request.operation.as_str() {
        "describe" => Ok(describe_value(request.input)),
        "radiance.io.colmapCameraSupport" => {
            let input = parse_input::<DatasetRequest>(request.input)?;
            let dataset = input.dataset.into_dataset();
            let cameras = inspect_colmap_camera_support(&dataset)
                .into_iter()
                .map(|support| {
                    serde_json::json!({
                        "cameraId": support.camera_id,
                        "rawModel": support.raw_model,
                        "model": camera_model_name(&support.model),
                        "supportedForViewConversion": support.supported_for_view_conversion,
                        "supportedForReconstructionConversion": support.supported_for_reconstruction_conversion,
                        "reason": support.reason
                    })
                })
                .collect::<Vec<_>>();
            Ok(surface_response(
                request.operation,
                serde_json::json!({"cameras": cameras}),
            ))
        }
        "radiance.io.colmapSummary" => {
            let input = parse_input::<DatasetRequest>(request.input)?;
            let dataset = input.dataset.into_dataset();
            let support = inspect_colmap_camera_support(&dataset);
            let supported = support
                .iter()
                .filter(|camera| {
                    camera.supported_for_view_conversion
                        && camera.supported_for_reconstruction_conversion
                })
                .count();
            Ok(surface_response(
                request.operation,
                serde_json::json!({
                    "cameraCount": dataset.cameras.len(),
                    "imageCount": dataset.images.len(),
                    "pointCount": dataset.points.len(),
                    "supportedCameraCount": supported,
                    "unsupportedCameraCount": support.len().saturating_sub(supported)
                }),
            ))
        }
        "radiance.io.gaussianSplatSummary" => {
            let input = parse_input::<GaussianSceneRequest>(request.input)?;
            let scene = input.scene.into_scene();
            let stats = scene.stats().map_err(|err| err.to_string())?;
            let sh_degrees = scene
                .splats
                .iter()
                .map(|splat| splat.sh.degree)
                .collect::<Vec<_>>();
            let max_sh_degree = sh_degrees.iter().copied().max();
            Ok(surface_response(
                request.operation,
                serde_json::json!({
                    "primitiveCount": stats.count,
                    "bounds": stats.bounds.map(|bounds| serde_json::json!({
                        "min": vec3_value(bounds.min),
                        "max": vec3_value(bounds.max)
                    })),
                    "meanOpacity": stats.mean_opacity,
                    "minScale": vec3_value(stats.min_scale),
                    "maxScale": vec3_value(stats.max_scale),
                    "sphericalHarmonics": {
                        "degrees": sh_degrees,
                        "maxDegree": max_sh_degree
                    }
                }),
            ))
        }
        operation => Err(format!(
            "unsupported operation `{operation}` for {}",
            env!("CARGO_PKG_NAME")
        )),
    }
}

#[derive(Debug, Deserialize)]
struct DatasetRequest {
    dataset: ColmapDatasetInput,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ColmapDatasetInput {
    #[serde(default)]
    cameras: Vec<ColmapCameraInput>,
    #[serde(default)]
    images: Vec<ColmapImageInput>,
    #[serde(default, alias = "points", alias = "points3D")]
    points3d: Vec<ColmapPoint3dInput>,
}

impl ColmapDatasetInput {
    fn into_dataset(self) -> ColmapDataset {
        ColmapDataset {
            cameras: self
                .cameras
                .into_iter()
                .map(ColmapCameraInput::into_camera)
                .collect(),
            images: self
                .images
                .into_iter()
                .map(ColmapImageInput::into_image)
                .collect(),
            points: self
                .points3d
                .into_iter()
                .map(ColmapPoint3dInput::into_point)
                .collect(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ColmapCameraInput {
    id: u32,
    #[serde(default)]
    raw_model: Option<String>,
    #[serde(default)]
    model: Option<String>,
    width: u32,
    height: u32,
    #[serde(default)]
    params: Vec<f32>,
}

impl ColmapCameraInput {
    fn into_camera(self) -> ColmapCamera {
        let raw_model = self
            .raw_model
            .or(self.model)
            .unwrap_or_else(|| "PINHOLE".to_string());
        ColmapCamera {
            id: self.id,
            model: camera_model_from_name(&raw_model),
            raw_model,
            width: self.width,
            height: self.height,
            params: self.params,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ColmapImageInput {
    id: u32,
    #[serde(default = "default_one")]
    qw: f32,
    #[serde(default)]
    qx: f32,
    #[serde(default)]
    qy: f32,
    #[serde(default)]
    qz: f32,
    #[serde(default)]
    tx: f32,
    #[serde(default)]
    ty: f32,
    #[serde(default)]
    tz: f32,
    camera_id: u32,
    #[serde(default)]
    name: String,
    #[serde(default)]
    points2d: Vec<ColmapPoint2dInput>,
}

impl ColmapImageInput {
    fn into_image(self) -> ColmapImage {
        ColmapImage {
            id: self.id,
            qw: self.qw,
            qx: self.qx,
            qy: self.qy,
            qz: self.qz,
            tx: self.tx,
            ty: self.ty,
            tz: self.tz,
            camera_id: self.camera_id,
            name: self.name,
            points2d: self
                .points2d
                .into_iter()
                .map(ColmapPoint2dInput::into_point)
                .collect(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ColmapPoint2dInput {
    xy: [f32; 2],
    point3d_id: Option<u64>,
}

impl ColmapPoint2dInput {
    fn into_point(self) -> ColmapPoint2d {
        ColmapPoint2d {
            xy: Vec2::new(self.xy[0], self.xy[1]),
            point3d_id: self.point3d_id,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ColmapPoint3dInput {
    id: u64,
    xyz: [f32; 3],
    #[serde(default)]
    color: Option<[f32; 3]>,
    #[serde(default)]
    error: f32,
    #[serde(default)]
    track: Vec<ColmapTrackElementInput>,
}

impl ColmapPoint3dInput {
    fn into_point(self) -> ColmapPoint3d {
        let color = self.color.unwrap_or([0.0, 0.0, 0.0]);
        ColmapPoint3d {
            id: self.id,
            xyz: vec3(self.xyz),
            color: ColorRgb::new(color[0], color[1], color[2]),
            error: self.error,
            track: self
                .track
                .into_iter()
                .map(ColmapTrackElementInput::into_track)
                .collect(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ColmapTrackElementInput {
    image_id: u32,
    point2d_index: usize,
}

impl ColmapTrackElementInput {
    fn into_track(self) -> ColmapTrackElement {
        ColmapTrackElement {
            image_id: self.image_id,
            point2d_index: self.point2d_index,
        }
    }
}

#[derive(Debug, Deserialize)]
struct GaussianSceneRequest {
    scene: GaussianSceneInput,
}

#[derive(Debug, Deserialize)]
struct GaussianSceneInput {
    #[serde(default)]
    splats: Vec<GaussianSplatInput>,
}

impl GaussianSceneInput {
    fn into_scene(self) -> GaussianSplatScene {
        GaussianSplatScene {
            splats: self
                .splats
                .into_iter()
                .map(GaussianSplatInput::into_splat)
                .collect(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GaussianSplatInput {
    mean: [f32; 3],
    scale_log: [f32; 3],
    #[serde(default = "default_rotation")]
    rotation: [f32; 4],
    opacity_logit: f32,
    sh: SphericalHarmonicsInput,
}

impl GaussianSplatInput {
    fn into_splat(self) -> GaussianSplat3d {
        GaussianSplat3d {
            mean: vec3(self.mean),
            scale_log: vec3(self.scale_log),
            rotation: Quaternion::new(
                self.rotation[0],
                self.rotation[1],
                self.rotation[2],
                self.rotation[3],
            ),
            opacity_logit: self.opacity_logit,
            sh: SphericalHarmonicsRgb {
                degree: self.sh.degree,
                coeffs: self.sh.coeffs,
            },
        }
    }
}

#[derive(Debug, Deserialize)]
struct SphericalHarmonicsInput {
    degree: u8,
    coeffs: Vec<[f32; 3]>,
}

fn default_one() -> f32 {
    1.0
}

fn default_rotation() -> [f32; 4] {
    [0.0, 0.0, 0.0, 1.0]
}

fn camera_model_from_name(name: &str) -> CameraModel {
    match name {
        "PINHOLE" => CameraModel::Pinhole,
        "SIMPLE_PINHOLE" => CameraModel::SimplePinhole,
        "RADIAL" => CameraModel::Radial,
        "SIMPLE_RADIAL" => CameraModel::SimpleRadial,
        "OPENCV" => CameraModel::OpenCv,
        other => CameraModel::Unsupported(other.to_string()),
    }
}

fn camera_model_name(model: &CameraModel) -> serde_json::Value {
    match model {
        CameraModel::Pinhole => serde_json::json!("PINHOLE"),
        CameraModel::SimplePinhole => serde_json::json!("SIMPLE_PINHOLE"),
        CameraModel::Radial => serde_json::json!("RADIAL"),
        CameraModel::SimpleRadial => serde_json::json!("SIMPLE_RADIAL"),
        CameraModel::OpenCv => serde_json::json!("OPENCV"),
        CameraModel::Unsupported(value) => serde_json::json!({"unsupported": value}),
    }
}

fn vec3(value: [f32; 3]) -> Vec3 {
    Vec3::new(value[0], value[1], value[2])
}

fn vec3_value(value: Vec3) -> serde_json::Value {
    serde_json::json!([value.x, value.y, value.z])
}

fn operation(
    id: &str,
    name: &str,
    description: &str,
    input_schema: serde_json::Value,
    output_schema: serde_json::Value,
    example_request: serde_json::Value,
) -> SurfaceOperation {
    SurfaceOperation {
        id: OperationId::new(id),
        name: name.to_string(),
        description: Some(description.to_string()),
        input_schema,
        output_schema,
        example_request,
        wasm_supported: true,
        server_supported: true,
    }
}

fn describe_value(input: serde_json::Value) -> SurfaceResponse {
    let surface = package_surface();
    surface_response(
        OperationId::new("describe"),
        serde_json::json!({
            "library": surface.library,
            "version": surface.version,
            "operationCount": surface.operations.len(),
            "operations": surface
                .operations
                .iter()
                .map(|operation| operation.id.as_str())
                .collect::<Vec<_>>(),
            "input": input
        }),
    )
}

fn parse_input<T: DeserializeOwned>(input: serde_json::Value) -> Result<T, String> {
    serde_json::from_value(input).map_err(|err| format!("invalid request: {err}"))
}

fn surface_response(operation: OperationId, value: serde_json::Value) -> SurfaceResponse {
    SurfaceResponse {
        operation,
        value,
        diagnostics: Vec::new(),
        artifacts: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_colmap_camera_model_is_reported() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("radiance.io.colmapCameraSupport"),
            input: serde_json::json!({
                "dataset": {
                    "cameras": [{
                        "id": 1,
                        "rawModel": "FISHEYE",
                        "width": 640,
                        "height": 480,
                        "params": []
                    }],
                    "images": [],
                    "points3d": []
                }
            }),
        })
        .expect("camera support");

        assert_eq!(
            response.value["cameras"][0]["supportedForViewConversion"],
            false
        );
        assert!(response.value["cameras"][0]["reason"]
            .as_str()
            .unwrap()
            .contains("unsupported COLMAP camera model"));
    }

    #[test]
    fn empty_dataset_summary_is_stable() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("radiance.io.colmapSummary"),
            input: serde_json::json!({"dataset": {"cameras": [], "images": [], "points3d": []}}),
        })
        .expect("colmap summary");

        assert_eq!(response.value["cameraCount"], 0);
        assert_eq!(response.value["imageCount"], 0);
        assert_eq!(response.value["pointCount"], 0);
        assert_eq!(response.value["unsupportedCameraCount"], 0);
    }

    #[test]
    fn gaussian_splat_summary_rejects_malformed_values() {
        let err = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("radiance.io.gaussianSplatSummary"),
            input: serde_json::json!({
                "scene": {
                    "splats": [{
                        "mean": [0.0, 0.0, 0.0],
                        "scaleLog": [0.0, 0.0, 0.0],
                        "rotation": [0.0, 0.0, 0.0, 1.0],
                        "opacityLogit": 0.0,
                        "sh": {"degree": 1, "coeffs": [[0.0, 0.0, 0.0]]}
                    }]
                }
            }),
        })
        .expect_err("invalid spherical harmonics");

        assert!(err.contains("spherical harmonics degree"));
    }
}
