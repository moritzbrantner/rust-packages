#![doc = include_str!("../README.md")]

pub mod surface;
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use video_analysis_core::DetectError;
use video_analysis_gaussian_splatting::{
    GaussianSceneStats, GaussianSplat3d, GaussianSplatScene, Quaternion, SphericalHarmonicsRgb,
};
use video_analysis_radiance_fields::{
    CameraDistortion, CameraIntrinsics, CameraModel, CameraPose, CameraView, CameraViewSet,
    ColorRgb, Vec2, Vec3,
};
use video_analysis_reconstruction::{
    CameraId, ImageId, ReconstructionCamera, ReconstructionImage, SparseReconstruction, Track,
    TrackElement,
};

#[derive(Debug, Error)]
/// Variants describing radiance I/O error.
pub enum RadianceIoError {
    #[error("I/O error: {0}")]
    /// The I/O variant.
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    /// The JSON variant.
    Json(#[from] serde_json::Error),
    #[error("parse error in {path}:{line}: {message}")]
    /// The parse variant.
    Parse {
        /// Filesystem path for this variant.
        path: String,
        /// Line number associated with this variant.
        line: usize,
        /// Diagnostic message for this variant.
        message: String,
    },
    #[error("unsupported camera model {model}")]
    /// The unsupported camera model variant.
    UnsupportedCameraModel {
        /// Model associated with this variant.
        model: String,
    },
    #[error("unsupported PLY format: {0}")]
    /// The unsupported PLY variant.
    UnsupportedPly(String),
    #[error("invalid data: {0}")]
    /// The invalid data variant.
    InvalidData(String),
}

impl From<DetectError> for RadianceIoError {
    fn from(value: DetectError) -> Self {
        Self::InvalidData(value.to_string())
    }
}

/// Type alias for I/O result.
pub type IoResult<T> = std::result::Result<T, RadianceIoError>;

#[derive(Debug, Clone, PartialEq)]
/// Data type for COLMAP dataset.
pub struct ColmapDataset {
    /// The cameras value.
    pub cameras: Vec<ColmapCamera>,
    /// The images value.
    pub images: Vec<ColmapImage>,
    /// The points value.
    pub points: Vec<ColmapPoint3d>,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for COLMAP camera.
pub struct ColmapCamera {
    /// Identifier for this value.
    pub id: u32,
    /// The model value.
    pub model: CameraModel,
    /// The raw model value.
    pub raw_model: String,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// The params value.
    pub params: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for COLMAP image.
pub struct ColmapImage {
    /// Identifier for this value.
    pub id: u32,
    /// The qw value.
    pub qw: f32,
    /// The qx value.
    pub qx: f32,
    /// The qy value.
    pub qy: f32,
    /// The qz value.
    pub qz: f32,
    /// The tx value.
    pub tx: f32,
    /// The ty value.
    pub ty: f32,
    /// The tz value.
    pub tz: f32,
    /// The camera identifier value.
    pub camera_id: u32,
    /// Human-readable name for this value.
    pub name: String,
    /// The points2d value.
    pub points2d: Vec<ColmapPoint2d>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Data type for COLMAP point2d.
pub struct ColmapPoint2d {
    /// The xy value.
    pub xy: Vec2,
    /// The point3d identifier value.
    pub point3d_id: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for COLMAP point3d.
pub struct ColmapPoint3d {
    /// Identifier for this value.
    pub id: u64,
    /// The xyz value.
    pub xyz: Vec3,
    /// The color value.
    pub color: ColorRgb,
    /// The error value.
    pub error: f32,
    /// The track value.
    pub track: Vec<ColmapTrackElement>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Data type for COLMAP track element.
pub struct ColmapTrackElement {
    /// The image identifier value.
    pub image_id: u32,
    /// The point2d index value.
    pub point2d_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Data type for COLMAP camera support.
pub struct ColmapCameraSupport {
    /// The camera identifier value.
    pub camera_id: u32,
    /// The raw model value.
    pub raw_model: String,
    /// The model value.
    pub model: CameraModel,
    /// The supported for view conversion value.
    pub supported_for_view_conversion: bool,
    /// The supported for reconstruction conversion value.
    pub supported_for_reconstruction_conversion: bool,
    /// The reason value.
    pub reason: Option<String>,
}

/// Reads COLMAP text dir.
pub fn read_colmap_text_dir(path: impl AsRef<Path>) -> IoResult<ColmapDataset> {
    let path = path.as_ref();
    Ok(ColmapDataset {
        cameras: read_colmap_cameras(path.join("cameras.txt"))?,
        images: read_colmap_images(path.join("images.txt"))?,
        points: read_colmap_points(path.join("points3D.txt"))?,
    })
}

/// Returns inspect COLMAP camera support.
pub fn inspect_colmap_camera_support(dataset: &ColmapDataset) -> Vec<ColmapCameraSupport> {
    dataset
        .cameras
        .iter()
        .map(|camera| match camera.raw_model.as_str() {
            "SIMPLE_PINHOLE" | "PINHOLE" => ColmapCameraSupport {
                camera_id: camera.id,
                raw_model: camera.raw_model.clone(),
                model: camera.model.clone(),
                supported_for_view_conversion: true,
                supported_for_reconstruction_conversion: true,
                reason: None,
            },
            "SIMPLE_RADIAL" => ColmapCameraSupport {
                camera_id: camera.id,
                raw_model: camera.raw_model.clone(),
                model: camera.model.clone(),
                supported_for_view_conversion: true,
                supported_for_reconstruction_conversion: true,
                reason: None,
            },
            "RADIAL" | "OPENCV" => ColmapCameraSupport {
                camera_id: camera.id,
                raw_model: camera.raw_model.clone(),
                model: camera.model.clone(),
                supported_for_view_conversion: false,
                supported_for_reconstruction_conversion: false,
                reason: Some(
                    "pipeline MVP preserves distortion metadata at the I/O layer but does not normalize this camera model into direct ray/view conversion".to_string(),
                ),
            },
            other => ColmapCameraSupport {
                camera_id: camera.id,
                raw_model: camera.raw_model.clone(),
                model: camera.model.clone(),
                supported_for_view_conversion: false,
                supported_for_reconstruction_conversion: false,
                reason: Some(format!("unsupported COLMAP camera model `{other}`")),
            },
        })
        .collect()
}

/// Writes COLMAP text dir.
pub fn write_colmap_text_dir(path: impl AsRef<Path>, dataset: &ColmapDataset) -> IoResult<()> {
    let path = path.as_ref();
    fs::create_dir_all(path)?;
    write_colmap_cameras(path.join("cameras.txt"), &dataset.cameras)?;
    write_colmap_images(path.join("images.txt"), &dataset.images)?;
    write_colmap_points(path.join("points3D.txt"), &dataset.points)?;
    Ok(())
}

/// Returns COLMAP to view set.
pub fn colmap_to_view_set(dataset: &ColmapDataset) -> IoResult<CameraViewSet> {
    let cameras = dataset
        .cameras
        .iter()
        .map(|camera| (camera.id, camera))
        .collect::<BTreeMap<_, _>>();
    let mut views = Vec::with_capacity(dataset.images.len());
    for image in &dataset.images {
        let camera = cameras.get(&image.camera_id).ok_or_else(|| {
            RadianceIoError::InvalidData(format!(
                "image {} references missing camera {}",
                image.id, image.camera_id
            ))
        })?;
        let (intrinsics, distortion) = colmap_camera_intrinsics(camera)?;
        views.push(CameraView {
            id: image.id,
            name: image.name.clone(),
            intrinsics,
            distortion,
            pose: CameraPose::from_colmap_world_to_camera(
                image.qw, image.qx, image.qy, image.qz, image.tx, image.ty, image.tz,
            )?,
        });
    }
    let view_set = CameraViewSet { views };
    view_set.validate()?;
    Ok(view_set)
}

/// Returns COLMAP to sparse reconstruction.
pub fn colmap_to_sparse_reconstruction(dataset: &ColmapDataset) -> IoResult<SparseReconstruction> {
    let view_set = colmap_to_view_set(dataset)?;
    let mut reconstruction = SparseReconstruction::new();
    for camera in &dataset.cameras {
        let (intrinsics, _) = colmap_camera_intrinsics(camera)?;
        reconstruction.add_camera(ReconstructionCamera::new(CameraId(camera.id), intrinsics)?)?;
    }
    for (image, view) in dataset.images.iter().zip(view_set.views.iter()) {
        let mut reconstruction_image = ReconstructionImage::new(
            ImageId(image.id),
            CameraId(image.camera_id),
            image.name.clone(),
            view.pose,
        )?;
        for point in &image.points2d {
            reconstruction_image
                .add_feature(video_analysis_reconstruction::Feature2d::new(point.xy)?)?;
        }
        reconstruction.add_image(reconstruction_image)?;
    }
    for point in &dataset.points {
        if point.track.len() < 2 {
            continue;
        }
        let track = Track::new(
            point
                .track
                .iter()
                .map(|element| TrackElement::new(ImageId(element.image_id), element.point2d_index))
                .collect::<Vec<_>>(),
        )?;
        reconstruction.insert_point(point.xyz, point.color, track, point.error)?;
    }
    Ok(reconstruction)
}

fn colmap_camera_intrinsics(
    camera: &ColmapCamera,
) -> IoResult<(CameraIntrinsics, Option<CameraDistortion>)> {
    match camera.raw_model.as_str() {
        "SIMPLE_PINHOLE" => {
            require_params(camera, 3)?;
            Ok((
                CameraIntrinsics::new(
                    camera.width,
                    camera.height,
                    camera.params[0],
                    camera.params[0],
                    camera.params[1],
                    camera.params[2],
                )?,
                None,
            ))
        }
        "PINHOLE" => {
            require_params(camera, 4)?;
            Ok((
                CameraIntrinsics::new(
                    camera.width,
                    camera.height,
                    camera.params[0],
                    camera.params[1],
                    camera.params[2],
                    camera.params[3],
                )?,
                None,
            ))
        }
        "SIMPLE_RADIAL" => {
            require_params(camera, 4)?;
            Ok((
                CameraIntrinsics::new(
                    camera.width,
                    camera.height,
                    camera.params[0],
                    camera.params[0],
                    camera.params[1],
                    camera.params[2],
                )?,
                Some(CameraDistortion {
                    model: CameraModel::SimpleRadial,
                    params: vec![camera.params[3]],
                }),
            ))
        }
        "RADIAL" | "OPENCV" => Err(RadianceIoError::UnsupportedCameraModel {
            model: camera.raw_model.clone(),
        }),
        _ => Err(RadianceIoError::UnsupportedCameraModel {
            model: camera.raw_model.clone(),
        }),
    }
}

fn require_params(camera: &ColmapCamera, len: usize) -> IoResult<()> {
    if camera.params.len() < len {
        return Err(RadianceIoError::InvalidData(format!(
            "camera {} model {} requires at least {len} params",
            camera.id, camera.raw_model
        )));
    }
    Ok(())
}

fn read_colmap_cameras(path: impl AsRef<Path>) -> IoResult<Vec<ColmapCamera>> {
    let path = path.as_ref();
    let mut cameras = Vec::new();
    for (line_index, line) in colmap_data_lines(path)? {
        let parts = line.split_whitespace().collect::<Vec<_>>();
        if parts.len() < 5 {
            return parse_error(path, line_index, "camera line requires at least 5 fields");
        }
        let raw_model = parts[1].to_string();
        cameras.push(ColmapCamera {
            id: parse_part(path, line_index, parts[0], "camera id")?,
            model: camera_model_from_colmap(&raw_model),
            raw_model,
            width: parse_part(path, line_index, parts[2], "camera width")?,
            height: parse_part(path, line_index, parts[3], "camera height")?,
            params: parts[4..]
                .iter()
                .map(|value| parse_part(path, line_index, value, "camera param"))
                .collect::<IoResult<Vec<_>>>()?,
        });
    }
    Ok(cameras)
}

fn read_colmap_images(path: impl AsRef<Path>) -> IoResult<Vec<ColmapImage>> {
    let path = path.as_ref();
    let lines = colmap_data_lines(path)?;
    let mut images = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let (meta_line, meta) = &lines[index];
        let parts = meta.split_whitespace().collect::<Vec<_>>();
        if parts.len() < 10 {
            return parse_error(
                path,
                *meta_line,
                "image metadata line requires at least 10 fields",
            );
        }
        let points_line = lines
            .get(index + 1)
            .map(|(_, line)| line.as_str())
            .unwrap_or("");
        let points = parse_colmap_points2d(path, *meta_line + 1, points_line)?;
        images.push(ColmapImage {
            id: parse_part(path, *meta_line, parts[0], "image id")?,
            qw: parse_part(path, *meta_line, parts[1], "qw")?,
            qx: parse_part(path, *meta_line, parts[2], "qx")?,
            qy: parse_part(path, *meta_line, parts[3], "qy")?,
            qz: parse_part(path, *meta_line, parts[4], "qz")?,
            tx: parse_part(path, *meta_line, parts[5], "tx")?,
            ty: parse_part(path, *meta_line, parts[6], "ty")?,
            tz: parse_part(path, *meta_line, parts[7], "tz")?,
            camera_id: parse_part(path, *meta_line, parts[8], "camera id")?,
            name: parts[9..].join(" "),
            points2d: points,
        });
        index += 2;
    }
    Ok(images)
}

fn parse_colmap_points2d(
    path: &Path,
    line_index: usize,
    line: &str,
) -> IoResult<Vec<ColmapPoint2d>> {
    let parts = line.split_whitespace().collect::<Vec<_>>();
    if parts.is_empty() {
        return Ok(Vec::new());
    }
    if parts.len() % 3 != 0 {
        return parse_error(
            path,
            line_index,
            "points2D line must contain x y point3D_id triples",
        );
    }
    let mut points = Vec::new();
    for chunk in parts.chunks(3) {
        let point3d_id: i64 = parse_part(path, line_index, chunk[2], "point3D id")?;
        points.push(ColmapPoint2d {
            xy: Vec2::new(
                parse_part(path, line_index, chunk[0], "point x")?,
                parse_part(path, line_index, chunk[1], "point y")?,
            ),
            point3d_id: (point3d_id >= 0).then_some(point3d_id as u64),
        });
    }
    Ok(points)
}

fn read_colmap_points(path: impl AsRef<Path>) -> IoResult<Vec<ColmapPoint3d>> {
    let path = path.as_ref();
    let mut points = Vec::new();
    for (line_index, line) in colmap_data_lines(path)? {
        let parts = line.split_whitespace().collect::<Vec<_>>();
        if parts.len() < 8 {
            return parse_error(path, line_index, "point3D line requires at least 8 fields");
        }
        let track_parts = &parts[8..];
        if track_parts.len() % 2 != 0 {
            return parse_error(
                path,
                line_index,
                "point3D track must contain image/point pairs",
            );
        }
        let mut track = Vec::new();
        for chunk in track_parts.chunks(2) {
            track.push(ColmapTrackElement {
                image_id: parse_part(path, line_index, chunk[0], "track image id")?,
                point2d_index: parse_part(path, line_index, chunk[1], "track point2D index")?,
            });
        }
        let color = ColorRgb::new(
            parse_part::<f32>(path, line_index, parts[4], "red")? / 255.0,
            parse_part::<f32>(path, line_index, parts[5], "green")? / 255.0,
            parse_part::<f32>(path, line_index, parts[6], "blue")? / 255.0,
        );
        points.push(ColmapPoint3d {
            id: parse_part(path, line_index, parts[0], "point3D id")?,
            xyz: Vec3::new(
                parse_part(path, line_index, parts[1], "x")?,
                parse_part(path, line_index, parts[2], "y")?,
                parse_part(path, line_index, parts[3], "z")?,
            ),
            color,
            error: parse_part(path, line_index, parts[7], "error")?,
            track,
        });
    }
    Ok(points)
}

fn write_colmap_cameras(path: impl AsRef<Path>, cameras: &[ColmapCamera]) -> IoResult<()> {
    let mut writer = BufWriter::new(File::create(path)?);
    for camera in cameras {
        writeln!(
            writer,
            "{} {} {} {} {}",
            camera.id,
            camera.raw_model,
            camera.width,
            camera.height,
            join_f32(&camera.params)
        )?;
    }
    Ok(())
}

fn write_colmap_images(path: impl AsRef<Path>, images: &[ColmapImage]) -> IoResult<()> {
    let mut writer = BufWriter::new(File::create(path)?);
    for image in images {
        writeln!(
            writer,
            "{} {} {} {} {} {} {} {} {} {}",
            image.id,
            image.qw,
            image.qx,
            image.qy,
            image.qz,
            image.tx,
            image.ty,
            image.tz,
            image.camera_id,
            image.name
        )?;
        let points = image
            .points2d
            .iter()
            .map(|point| {
                format!(
                    "{} {} {}",
                    point.xy.x,
                    point.xy.y,
                    point
                        .point3d_id
                        .map(|id| id.to_string())
                        .unwrap_or_else(|| "-1".to_string())
                )
            })
            .collect::<Vec<_>>()
            .join(" ");
        writeln!(writer, "{points}")?;
    }
    Ok(())
}

fn write_colmap_points(path: impl AsRef<Path>, points: &[ColmapPoint3d]) -> IoResult<()> {
    let mut writer = BufWriter::new(File::create(path)?);
    for point in points {
        write!(
            writer,
            "{} {} {} {} {} {} {} {}",
            point.id,
            point.xyz.x,
            point.xyz.y,
            point.xyz.z,
            color_to_u8(point.color.r),
            color_to_u8(point.color.g),
            color_to_u8(point.color.b),
            point.error
        )?;
        for element in &point.track {
            write!(writer, " {} {}", element.image_id, element.point2d_index)?;
        }
        writeln!(writer)?;
    }
    Ok(())
}

fn colmap_data_lines(path: &Path) -> IoResult<Vec<(usize, String)>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut lines = Vec::new();
    for (line_index, line) in reader.lines().enumerate() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        lines.push((line_index + 1, trimmed.to_string()));
    }
    Ok(lines)
}

fn camera_model_from_colmap(model: &str) -> CameraModel {
    match model {
        "PINHOLE" => CameraModel::Pinhole,
        "SIMPLE_PINHOLE" => CameraModel::SimplePinhole,
        "RADIAL" => CameraModel::Radial,
        "SIMPLE_RADIAL" => CameraModel::SimpleRadial,
        "OPENCV" => CameraModel::OpenCv,
        other => CameraModel::Unsupported(other.to_string()),
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Data type for Nerfstudio transforms.
pub struct NerfstudioTransforms {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// The camera model value.
    pub camera_model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// The fl x value.
    pub fl_x: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// The fl y value.
    pub fl_y: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// The cx value.
    pub cx: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// The cy value.
    pub cy: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// The w value.
    pub w: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// The h value.
    pub h: Option<u32>,
    /// The frames value.
    pub frames: Vec<NerfstudioFrame>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Data type for Nerfstudio frame.
pub struct NerfstudioFrame {
    /// The file path value.
    pub file_path: String,
    /// The transform matrix value.
    pub transform_matrix: [[f32; 4]; 4],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// The fl x value.
    pub fl_x: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// The fl y value.
    pub fl_y: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// The cx value.
    pub cx: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// The cy value.
    pub cy: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// The w value.
    pub w: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// The h value.
    pub h: Option<u32>,
}

/// Reads Nerfstudio transforms.
pub fn read_nerfstudio_transforms(path: impl AsRef<Path>) -> IoResult<NerfstudioTransforms> {
    Ok(serde_json::from_reader(BufReader::new(File::open(path)?))?)
}

/// Writes Nerfstudio transforms.
pub fn write_nerfstudio_transforms(
    path: impl AsRef<Path>,
    value: &NerfstudioTransforms,
) -> IoResult<()> {
    serde_json::to_writer_pretty(BufWriter::new(File::create(path)?), value)?;
    Ok(())
}

/// Returns transforms to view set.
pub fn transforms_to_view_set(value: &NerfstudioTransforms) -> IoResult<CameraViewSet> {
    let mut views = Vec::with_capacity(value.frames.len());
    for (index, frame) in value.frames.iter().enumerate() {
        if frame.file_path.trim().is_empty() {
            return Err(RadianceIoError::InvalidData(
                "Nerfstudio frame file_path must not be empty".to_string(),
            ));
        }
        let matrix = frame.transform_matrix;
        if matrix.iter().flatten().any(|value| !value.is_finite()) {
            return Err(RadianceIoError::InvalidData(
                "Nerfstudio transform matrix must be finite".to_string(),
            ));
        }
        let width = frame.w.or(value.w).ok_or_else(|| {
            RadianceIoError::InvalidData("Nerfstudio transform is missing width".to_string())
        })?;
        let height = frame.h.or(value.h).ok_or_else(|| {
            RadianceIoError::InvalidData("Nerfstudio transform is missing height".to_string())
        })?;
        let fx = frame.fl_x.or(value.fl_x).ok_or_else(|| {
            RadianceIoError::InvalidData("Nerfstudio transform is missing fl_x".to_string())
        })?;
        let fy = frame.fl_y.or(value.fl_y).unwrap_or(fx);
        let cx = frame.cx.or(value.cx).unwrap_or((width as f32 - 1.0) * 0.5);
        let cy = frame.cy.or(value.cy).unwrap_or((height as f32 - 1.0) * 0.5);
        let position = Vec3::new(matrix[0][3], matrix[1][3], matrix[2][3]);
        let right = Vec3::new(matrix[0][0], matrix[1][0], matrix[2][0]);
        let up = Vec3::new(matrix[0][1], matrix[1][1], matrix[2][1]);
        let forward = Vec3::new(-matrix[0][2], -matrix[1][2], -matrix[2][2]);
        views.push(CameraView {
            id: index as u32,
            name: frame.file_path.clone(),
            intrinsics: CameraIntrinsics::new(width, height, fx, fy, cx, cy)?,
            distortion: value.camera_model.as_ref().and_then(|model| {
                let camera_model = camera_model_from_colmap(&model.to_ascii_uppercase());
                (!matches!(
                    camera_model,
                    CameraModel::Pinhole | CameraModel::SimplePinhole
                ))
                .then_some(CameraDistortion {
                    model: camera_model,
                    params: Vec::new(),
                })
            }),
            pose: CameraPose::new(position, right, up, forward)?,
        });
    }
    let view_set = CameraViewSet { views };
    view_set.validate()?;
    Ok(view_set)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlyProperty {
    name: String,
    kind: PlyScalarKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlyScalarKind {
    Float,
    Double,
    UChar,
    Int,
    UInt,
}

/// Reads gaussian splat PLY.
pub fn read_gaussian_splat_ply(path: impl AsRef<Path>) -> IoResult<GaussianSplatScene> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    if line.trim() != "ply" {
        return Err(RadianceIoError::UnsupportedPly(
            "missing ply magic header".to_string(),
        ));
    }
    let mut vertex_count = None;
    let mut properties = Vec::new();
    let mut in_vertex = false;
    loop {
        line.clear();
        if reader.read_line(&mut line)? == 0 {
            return Err(RadianceIoError::UnsupportedPly(
                "missing end_header".to_string(),
            ));
        }
        let trimmed = line.trim();
        if trimmed == "end_header" {
            break;
        }
        let parts = trimmed.split_whitespace().collect::<Vec<_>>();
        match parts.as_slice() {
            ["format", "binary_little_endian", "1.0"] => {}
            ["format", format, ..] => {
                return Err(RadianceIoError::UnsupportedPly(format!(
                    "expected binary_little_endian, got {format}"
                )));
            }
            ["element", "vertex", count] => {
                vertex_count = Some(count.parse::<usize>().map_err(|_| {
                    RadianceIoError::UnsupportedPly("invalid vertex count".to_string())
                })?);
                in_vertex = true;
            }
            ["element", ..] => in_vertex = false,
            ["property", kind, name] if in_vertex => {
                properties.push(PlyProperty {
                    name: (*name).to_string(),
                    kind: parse_ply_scalar_kind(kind)?,
                });
            }
            _ => {}
        }
    }
    let vertex_count = vertex_count
        .ok_or_else(|| RadianceIoError::UnsupportedPly("missing vertex element".to_string()))?;
    require_ply_properties(&properties)?;
    let mut splats = Vec::with_capacity(vertex_count);
    for _ in 0..vertex_count {
        let values = read_ply_vertex(&mut reader, &properties)?;
        splats.push(values_to_splat(&values)?);
    }
    let scene = GaussianSplatScene { splats };
    scene.validate()?;
    Ok(scene)
}

/// Writes gaussian splat PLY.
pub fn write_gaussian_splat_ply(
    path: impl AsRef<Path>,
    scene: &GaussianSplatScene,
) -> IoResult<()> {
    scene.validate()?;
    let max_coeffs = scene
        .splats
        .iter()
        .map(|splat| splat.sh.coeffs.len())
        .max()
        .unwrap_or(1);
    let rest_count = max_coeffs.saturating_sub(1) * 3;
    let mut writer = BufWriter::new(File::create(path)?);
    writeln!(writer, "ply")?;
    writeln!(writer, "format binary_little_endian 1.0")?;
    writeln!(writer, "element vertex {}", scene.splats.len())?;
    for name in ["x", "y", "z", "nx", "ny", "nz"] {
        writeln!(writer, "property float {name}")?;
    }
    for index in 0..3 {
        writeln!(writer, "property float f_dc_{index}")?;
    }
    for index in 0..rest_count {
        writeln!(writer, "property float f_rest_{index}")?;
    }
    writeln!(writer, "property float opacity")?;
    for index in 0..3 {
        writeln!(writer, "property float scale_{index}")?;
    }
    for index in 0..4 {
        writeln!(writer, "property float rot_{index}")?;
    }
    writeln!(writer, "end_header")?;
    for splat in &scene.splats {
        write_f32(&mut writer, splat.mean.x)?;
        write_f32(&mut writer, splat.mean.y)?;
        write_f32(&mut writer, splat.mean.z)?;
        for _ in 0..3 {
            write_f32(&mut writer, 0.0)?;
        }
        let dc = splat.sh.coeffs.first().copied().unwrap_or([0.0; 3]);
        for value in dc {
            write_f32(&mut writer, value)?;
        }
        for coeff_index in 1..max_coeffs {
            let coeff = splat
                .sh
                .coeffs
                .get(coeff_index)
                .copied()
                .unwrap_or([0.0; 3]);
            for value in coeff {
                write_f32(&mut writer, value)?;
            }
        }
        write_f32(&mut writer, splat.opacity_logit)?;
        write_f32(&mut writer, splat.scale_log.x)?;
        write_f32(&mut writer, splat.scale_log.y)?;
        write_f32(&mut writer, splat.scale_log.z)?;
        write_f32(&mut writer, splat.rotation.x)?;
        write_f32(&mut writer, splat.rotation.y)?;
        write_f32(&mut writer, splat.rotation.z)?;
        write_f32(&mut writer, splat.rotation.w)?;
    }
    writer.flush()?;
    Ok(())
}

/// Writes preview point cloud PLY.
pub fn write_preview_point_cloud_ply(
    path: impl AsRef<Path>,
    scene: &GaussianSplatScene,
) -> IoResult<()> {
    scene.validate()?;
    let mut writer = BufWriter::new(File::create(path)?);
    writeln!(writer, "ply")?;
    writeln!(writer, "format ascii 1.0")?;
    writeln!(writer, "element vertex {}", scene.splats.len())?;
    writeln!(writer, "property float x")?;
    writeln!(writer, "property float y")?;
    writeln!(writer, "property float z")?;
    writeln!(writer, "property uchar red")?;
    writeln!(writer, "property uchar green")?;
    writeln!(writer, "property uchar blue")?;
    writeln!(writer, "end_header")?;
    for splat in &scene.splats {
        let color = splat.preview_color();
        writeln!(
            writer,
            "{} {} {} {} {} {}",
            splat.mean.x,
            splat.mean.y,
            splat.mean.z,
            color_to_u8(color.r),
            color_to_u8(color.g),
            color_to_u8(color.b)
        )?;
    }
    Ok(())
}

fn require_ply_properties(properties: &[PlyProperty]) -> IoResult<()> {
    for name in [
        "x", "y", "z", "f_dc_0", "f_dc_1", "f_dc_2", "opacity", "scale_0", "scale_1", "scale_2",
        "rot_0", "rot_1", "rot_2", "rot_3",
    ] {
        if !properties.iter().any(|property| property.name == name) {
            return Err(RadianceIoError::UnsupportedPly(format!(
                "missing required property {name}"
            )));
        }
    }
    Ok(())
}

fn read_ply_vertex<R: Read>(
    reader: &mut R,
    properties: &[PlyProperty],
) -> IoResult<BTreeMap<String, f32>> {
    let mut values = BTreeMap::new();
    for property in properties {
        values.insert(
            property.name.clone(),
            read_ply_scalar(reader, property.kind)?,
        );
    }
    Ok(values)
}

fn values_to_splat(values: &BTreeMap<String, f32>) -> IoResult<GaussianSplat3d> {
    let mut rest = values
        .iter()
        .filter_map(|(name, value)| {
            name.strip_prefix("f_rest_")
                .and_then(|index| index.parse::<usize>().ok())
                .map(|index| (index, *value))
        })
        .collect::<Vec<_>>();
    rest.sort_by_key(|(index, _)| *index);
    if rest.len() % 3 != 0 {
        return Err(RadianceIoError::UnsupportedPly(
            "f_rest property count must be divisible by 3".to_string(),
        ));
    }
    let coeff_count = 1 + rest.len() / 3;
    let degree = infer_sh_degree(coeff_count)?;
    let mut coeffs = vec![[
        required_value(values, "f_dc_0")?,
        required_value(values, "f_dc_1")?,
        required_value(values, "f_dc_2")?,
    ]];
    for chunk in rest.chunks(3) {
        coeffs.push([chunk[0].1, chunk[1].1, chunk[2].1]);
    }
    let splat = GaussianSplat3d {
        mean: Vec3::new(
            required_value(values, "x")?,
            required_value(values, "y")?,
            required_value(values, "z")?,
        ),
        scale_log: Vec3::new(
            required_value(values, "scale_0")?,
            required_value(values, "scale_1")?,
            required_value(values, "scale_2")?,
        ),
        rotation: Quaternion::new(
            required_value(values, "rot_0")?,
            required_value(values, "rot_1")?,
            required_value(values, "rot_2")?,
            required_value(values, "rot_3")?,
        )
        .normalize()?,
        opacity_logit: required_value(values, "opacity")?,
        sh: SphericalHarmonicsRgb { degree, coeffs },
    };
    splat.validate()?;
    Ok(splat)
}

fn infer_sh_degree(coeff_count: usize) -> IoResult<u8> {
    for degree in 0..=8 {
        let len = (degree + 1) * (degree + 1);
        if len == coeff_count {
            return Ok(degree as u8);
        }
    }
    Err(RadianceIoError::UnsupportedPly(format!(
        "unsupported spherical harmonics coefficient count {coeff_count}"
    )))
}

fn required_value(values: &BTreeMap<String, f32>, name: &str) -> IoResult<f32> {
    values
        .get(name)
        .copied()
        .ok_or_else(|| RadianceIoError::UnsupportedPly(format!("missing property {name}")))
}

fn parse_ply_scalar_kind(kind: &str) -> IoResult<PlyScalarKind> {
    match kind {
        "float" | "float32" => Ok(PlyScalarKind::Float),
        "double" | "float64" => Ok(PlyScalarKind::Double),
        "uchar" | "uint8" => Ok(PlyScalarKind::UChar),
        "int" | "int32" => Ok(PlyScalarKind::Int),
        "uint" | "uint32" => Ok(PlyScalarKind::UInt),
        other => Err(RadianceIoError::UnsupportedPly(format!(
            "unsupported property scalar type {other}"
        ))),
    }
}

fn read_ply_scalar<R: Read>(reader: &mut R, kind: PlyScalarKind) -> IoResult<f32> {
    Ok(match kind {
        PlyScalarKind::Float => {
            let mut bytes = [0_u8; 4];
            reader.read_exact(&mut bytes)?;
            f32::from_le_bytes(bytes)
        }
        PlyScalarKind::Double => {
            let mut bytes = [0_u8; 8];
            reader.read_exact(&mut bytes)?;
            f64::from_le_bytes(bytes) as f32
        }
        PlyScalarKind::UChar => {
            let mut bytes = [0_u8; 1];
            reader.read_exact(&mut bytes)?;
            bytes[0] as f32
        }
        PlyScalarKind::Int => {
            let mut bytes = [0_u8; 4];
            reader.read_exact(&mut bytes)?;
            i32::from_le_bytes(bytes) as f32
        }
        PlyScalarKind::UInt => {
            let mut bytes = [0_u8; 4];
            reader.read_exact(&mut bytes)?;
            u32::from_le_bytes(bytes) as f32
        }
    })
}

fn write_f32(writer: &mut impl Write, value: f32) -> IoResult<()> {
    writer.write_all(&value.to_le_bytes())?;
    Ok(())
}

fn parse_part<T: std::str::FromStr>(
    path: &Path,
    line: usize,
    value: &str,
    name: &str,
) -> IoResult<T> {
    value.parse::<T>().map_err(|_| RadianceIoError::Parse {
        path: path.display().to_string(),
        line,
        message: format!("invalid {name}: {value}"),
    })
}

fn parse_error<T>(path: &Path, line: usize, message: impl Into<String>) -> IoResult<T> {
    Err(RadianceIoError::Parse {
        path: path.display().to_string(),
        line,
        message: message.into(),
    })
}

fn join_f32(values: &[f32]) -> String {
    values
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

fn color_to_u8(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

#[allow(dead_code)]
fn _assert_stats_is_public(_: Option<GaussianSceneStats>) {}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::tempdir;

    use super::*;

    fn write_fixture(path: &Path, content: &str) {
        let mut file = File::create(path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }

    #[test]
    fn colmap_parser_reads_minimal_dataset_and_converts_pinhole_views() {
        let dir = tempdir().unwrap();
        write_fixture(
            &dir.path().join("cameras.txt"),
            "1 PINHOLE 100 80 50 51 49 39\n",
        );
        write_fixture(
            &dir.path().join("images.txt"),
            "1 1 0 0 0 0 0 0 1 image.png\n10 11 1 20 21 -1\n",
        );
        write_fixture(
            &dir.path().join("points3D.txt"),
            "1 0 0 4 255 0 0 0.5 1 0 2 0\n",
        );

        let dataset = read_colmap_text_dir(dir.path()).unwrap();
        assert_eq!(dataset.cameras.len(), 1);
        assert_eq!(dataset.images[0].points2d.len(), 2);
        let views = colmap_to_view_set(&dataset).unwrap();
        assert_eq!(views.view_count(), 1);
        assert_eq!(views.views[0].intrinsics.fx, 50.0);
        let reconstruction = colmap_to_sparse_reconstruction(&dataset).unwrap();
        assert_eq!(reconstruction.points().len(), 1);
    }

    #[test]
    fn colmap_parser_preserves_opencv_camera_model_and_direct_conversion_rejects_it() {
        let dataset = ColmapDataset {
            cameras: vec![ColmapCamera {
                id: 1,
                model: CameraModel::OpenCv,
                raw_model: "OPENCV".to_string(),
                width: 100,
                height: 80,
                params: vec![50.0, 51.0, 49.0, 39.0, 0.01, 0.02, 0.0, 0.0],
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
                name: "image.png".to_string(),
                points2d: Vec::new(),
            }],
            points: Vec::new(),
        };

        assert_eq!(dataset.cameras[0].model, CameraModel::OpenCv);
        assert!(matches!(
            colmap_to_view_set(&dataset),
            Err(RadianceIoError::UnsupportedCameraModel { .. })
        ));
    }

    #[test]
    fn colmap_parser_converts_simple_radial_views() {
        let dataset = ColmapDataset {
            cameras: vec![ColmapCamera {
                id: 1,
                model: CameraModel::SimpleRadial,
                raw_model: "SIMPLE_RADIAL".to_string(),
                width: 100,
                height: 80,
                params: vec![50.0, 49.0, 39.0, 0.01],
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
                name: "image.png".to_string(),
                points2d: Vec::new(),
            }],
            points: Vec::new(),
        };

        let views = colmap_to_view_set(&dataset).unwrap();

        assert_eq!(views.views[0].intrinsics.fx, 50.0);
        assert_eq!(views.views[0].intrinsics.fy, 50.0);
        assert_eq!(views.views[0].intrinsics.cx, 49.0);
        assert_eq!(views.views[0].intrinsics.cy, 39.0);
        assert_eq!(
            views.views[0].distortion,
            Some(CameraDistortion {
                model: CameraModel::SimpleRadial,
                params: vec![0.01],
            })
        );
    }

    #[test]
    fn colmap_to_sparse_reconstruction_accepts_simple_radial() {
        let dataset = ColmapDataset {
            cameras: vec![ColmapCamera {
                id: 1,
                model: CameraModel::SimpleRadial,
                raw_model: "SIMPLE_RADIAL".to_string(),
                width: 100,
                height: 80,
                params: vec![50.0, 49.0, 39.0, 0.01],
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
                name: "image.png".to_string(),
                points2d: Vec::new(),
            }],
            points: Vec::new(),
        };

        let reconstruction = colmap_to_sparse_reconstruction(&dataset).unwrap();

        assert_eq!(reconstruction.cameras().len(), 1);
    }

    #[test]
    fn inspect_colmap_camera_support_marks_pinhole_models_supported() {
        let dataset = ColmapDataset {
            cameras: vec![ColmapCamera {
                id: 7,
                model: CameraModel::Pinhole,
                raw_model: "PINHOLE".to_string(),
                width: 64,
                height: 48,
                params: vec![50.0, 50.0, 32.0, 24.0],
            }],
            images: Vec::new(),
            points: Vec::new(),
        };

        assert_eq!(
            inspect_colmap_camera_support(&dataset),
            vec![ColmapCameraSupport {
                camera_id: 7,
                raw_model: "PINHOLE".to_string(),
                model: CameraModel::Pinhole,
                supported_for_view_conversion: true,
                supported_for_reconstruction_conversion: true,
                reason: None,
            }]
        );
    }

    #[test]
    fn inspect_colmap_camera_support_marks_radial_and_opencv_models_unsupported() {
        let dataset = ColmapDataset {
            cameras: vec![ColmapCamera {
                id: 3,
                model: CameraModel::OpenCv,
                raw_model: "OPENCV".to_string(),
                width: 64,
                height: 48,
                params: vec![50.0, 50.0, 32.0, 24.0, 0.1, 0.2, 0.0, 0.0],
            }],
            images: Vec::new(),
            points: Vec::new(),
        };

        let support = inspect_colmap_camera_support(&dataset);
        assert_eq!(support.len(), 1);
        assert!(!support[0].supported_for_view_conversion);
        assert!(!support[0].supported_for_reconstruction_conversion);
        assert!(support[0]
            .reason
            .as_deref()
            .unwrap()
            .contains("pipeline MVP preserves distortion metadata"));
    }

    #[test]
    fn inspect_colmap_camera_support_marks_simple_radial_supported() {
        let dataset = ColmapDataset {
            cameras: vec![ColmapCamera {
                id: 8,
                model: CameraModel::SimpleRadial,
                raw_model: "SIMPLE_RADIAL".to_string(),
                width: 64,
                height: 48,
                params: vec![50.0, 32.0, 24.0, 0.1],
            }],
            images: Vec::new(),
            points: Vec::new(),
        };

        assert_eq!(
            inspect_colmap_camera_support(&dataset),
            vec![ColmapCameraSupport {
                camera_id: 8,
                raw_model: "SIMPLE_RADIAL".to_string(),
                model: CameraModel::SimpleRadial,
                supported_for_view_conversion: true,
                supported_for_reconstruction_conversion: true,
                reason: None,
            }]
        );
    }

    #[test]
    fn inspect_colmap_camera_support_marks_unknown_models_unsupported() {
        let dataset = ColmapDataset {
            cameras: vec![ColmapCamera {
                id: 5,
                model: CameraModel::Unsupported("FISHEYE".to_string()),
                raw_model: "FISHEYE".to_string(),
                width: 64,
                height: 48,
                params: vec![50.0, 50.0, 32.0, 24.0],
            }],
            images: Vec::new(),
            points: Vec::new(),
        };

        let support = inspect_colmap_camera_support(&dataset);
        assert_eq!(support.len(), 1);
        assert!(!support[0].supported_for_view_conversion);
        assert!(!support[0].supported_for_reconstruction_conversion);
        assert_eq!(
            support[0].reason.as_deref(),
            Some("unsupported COLMAP camera model `FISHEYE`")
        );
    }

    #[test]
    fn colmap_parser_reports_line_errors() {
        let dir = tempdir().unwrap();
        write_fixture(&dir.path().join("cameras.txt"), "bad camera\n");
        write_fixture(&dir.path().join("images.txt"), "");
        write_fixture(&dir.path().join("points3D.txt"), "");

        let error = read_colmap_text_dir(dir.path()).unwrap_err();
        assert!(error.to_string().contains("cameras.txt:1"));
    }

    #[test]
    fn nerfstudio_transforms_round_trip_and_convert_to_views() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("transforms.json");
        let transforms = NerfstudioTransforms {
            camera_model: Some("OPENCV".to_string()),
            fl_x: Some(50.0),
            fl_y: Some(51.0),
            cx: Some(49.0),
            cy: Some(39.0),
            w: Some(100),
            h: Some(80),
            frames: vec![NerfstudioFrame {
                file_path: "image.png".to_string(),
                transform_matrix: [
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, -1.0, 1.0],
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

        write_nerfstudio_transforms(&path, &transforms).unwrap();
        let loaded = read_nerfstudio_transforms(&path).unwrap();
        assert_eq!(loaded, transforms);
        let views = transforms_to_view_set(&loaded).unwrap();
        assert_eq!(views.view_count(), 1);
        assert!(views.views[0].distortion.is_some());
    }

    #[test]
    fn nerfstudio_transforms_reject_bad_frames() {
        let transforms = NerfstudioTransforms {
            camera_model: None,
            fl_x: Some(50.0),
            fl_y: None,
            cx: None,
            cy: None,
            w: Some(100),
            h: Some(80),
            frames: vec![NerfstudioFrame {
                file_path: String::new(),
                transform_matrix: [[0.0; 4]; 4],
                fl_x: None,
                fl_y: None,
                cx: None,
                cy: None,
                w: None,
                h: None,
            }],
        };
        assert!(transforms_to_view_set(&transforms).is_err());
    }

    fn sample_splat(x: f32) -> GaussianSplat3d {
        GaussianSplat3d {
            mean: Vec3::new(x, 0.0, 1.0),
            scale_log: Vec3::new(0.0, 0.0, 0.0),
            rotation: Quaternion::IDENTITY,
            opacity_logit: 1.0,
            sh: SphericalHarmonicsRgb::dc(ColorRgb::WHITE),
        }
    }

    #[test]
    fn gaussian_splat_ply_round_trips_binary_little_endian() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("splats.ply");
        let scene = GaussianSplatScene {
            splats: vec![sample_splat(0.0), sample_splat(1.0)],
        };

        write_gaussian_splat_ply(&path, &scene).unwrap();
        let loaded = read_gaussian_splat_ply(&path).unwrap();

        assert_eq!(loaded.splats.len(), 2);
        assert_eq!(loaded.splats[1].mean.x, 1.0);
    }

    #[test]
    fn gaussian_splat_ply_rejects_missing_required_properties() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("bad.ply");
        let mut file = File::create(&path).unwrap();
        file.write_all(
            b"ply\nformat binary_little_endian 1.0\nelement vertex 1\nproperty float x\nend_header\n",
        )
        .unwrap();

        let error = read_gaussian_splat_ply(&path).unwrap_err();
        assert!(error.to_string().contains("missing required property"));
    }

    #[test]
    fn preview_point_cloud_ply_writes_ascii_rgb() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("preview.ply");
        let scene = GaussianSplatScene {
            splats: vec![sample_splat(0.0)],
        };

        write_preview_point_cloud_ply(&path, &scene).unwrap();
        let text = fs::read_to_string(path).unwrap();

        assert!(text.contains("format ascii 1.0"));
        assert!(text.contains("property uchar red"));
    }
}
