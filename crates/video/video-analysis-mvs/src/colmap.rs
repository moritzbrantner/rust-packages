use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

use three_d_processing_core::{Point3, PointCloud};
use video_analysis_core::{DetectError, Result};

use crate::{dense_report, DenseReconstruction, DenseReconstructor, MvsOutput, MvsRequest};

#[derive(Debug, Clone, PartialEq, Eq)]
/// Configuration for the COLMAP dense MVS backend.
pub struct ColmapMvsConfig {
    /// COLMAP command path.
    pub colmap_command: PathBuf,
    /// Directory containing source images.
    pub image_dir: PathBuf,
    /// Directory containing the sparse binary COLMAP model.
    pub sparse_model_dir: PathBuf,
    /// Dense COLMAP workspace directory.
    pub workspace_dir: PathBuf,
    /// Expected fused PLY output path.
    pub fused_ply_path: PathBuf,
    /// Whether to let COLMAP use GPU PatchMatch.
    pub use_gpu: bool,
    /// Maximum number of fused PLY vertices to load into memory.
    pub max_loaded_points: usize,
}

impl Default for ColmapMvsConfig {
    fn default() -> Self {
        Self {
            colmap_command: PathBuf::from("colmap"),
            image_dir: PathBuf::from(".external-test-tools/colmap-runs/test-video/frames"),
            sparse_model_dir: PathBuf::from(".external-test-tools/colmap-runs/test-video/sparse/0"),
            workspace_dir: PathBuf::from(".external-test-tools/colmap-runs/test-video/dense"),
            fused_ply_path: PathBuf::from(
                ".external-test-tools/colmap-runs/test-video/dense/fused.ply",
            ),
            use_gpu: false,
            max_loaded_points: 50_000,
        }
    }
}

impl ColmapMvsConfig {
    /// Validates this value.
    pub fn validate(&self) -> Result<()> {
        if self.max_loaded_points == 0 {
            return Err(invalid_argument("max loaded points must be positive"));
        }
        if !self.image_dir.is_dir() {
            return Err(invalid_argument(format!(
                "COLMAP MVS image dir does not exist: {}",
                self.image_dir.display()
            )));
        }
        let image_count = fs::read_dir(&self.image_dir)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| is_image_file(&entry.path()))
            .count();
        if image_count < 2 {
            return Err(invalid_argument(
                "COLMAP MVS image dir must contain at least two images",
            ));
        }
        for name in ["cameras.bin", "images.bin", "points3D.bin"] {
            let path = self.sparse_model_dir.join(name);
            if !path.is_file() {
                return Err(invalid_argument(format!(
                    "COLMAP MVS sparse model is missing {}",
                    path.display()
                )));
            }
        }
        if let Some(parent) = self.workspace_dir.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
/// COLMAP dense MVS backend.
pub struct ColmapMvsBackend {
    config: ColmapMvsConfig,
}

impl ColmapMvsBackend {
    /// Creates a new backend.
    pub fn new(config: ColmapMvsConfig) -> Result<Self> {
        config.validate()?;
        Ok(Self { config })
    }

    /// Returns config.
    pub fn config(&self) -> &ColmapMvsConfig {
        &self.config
    }

    /// Reads the fused PLY artifact report for the current config.
    pub fn artifact_report(&self) -> Result<ColmapMvsArtifactReport> {
        artifact_report_from_ply(
            &self.config.workspace_dir,
            &self.config.fused_ply_path,
            self.config.max_loaded_points,
        )
    }
}

impl DenseReconstructor for ColmapMvsBackend {
    fn name(&self) -> &'static str {
        "colmap-mvs-backend"
    }

    fn reconstruct_dense(&mut self, request: &MvsRequest) -> Result<MvsOutput> {
        request.validate()?;
        self.config.validate()?;
        run_colmap_command(
            "image_undistorter",
            Command::new(&self.config.colmap_command)
                .arg("image_undistorter")
                .arg("--image_path")
                .arg(&self.config.image_dir)
                .arg("--input_path")
                .arg(&self.config.sparse_model_dir)
                .arg("--output_path")
                .arg(&self.config.workspace_dir)
                .arg("--output_type")
                .arg("COLMAP"),
        )?;
        let mut patch_match = Command::new(&self.config.colmap_command);
        patch_match
            .arg("patch_match_stereo")
            .arg("--workspace_path")
            .arg(&self.config.workspace_dir)
            .arg("--workspace_format")
            .arg("COLMAP")
            .arg("--PatchMatchStereo.geom_consistency")
            .arg("true");
        if !self.config.use_gpu {
            patch_match.arg("--PatchMatchStereo.gpu_index").arg("-1");
        }
        run_colmap_command("patch_match_stereo", &mut patch_match)?;
        run_colmap_command(
            "stereo_fusion",
            Command::new(&self.config.colmap_command)
                .arg("stereo_fusion")
                .arg("--workspace_path")
                .arg(&self.config.workspace_dir)
                .arg("--workspace_format")
                .arg("COLMAP")
                .arg("--input_type")
                .arg("geometric")
                .arg("--output_path")
                .arg(&self.config.fused_ply_path),
        )?;
        if !self.config.fused_ply_path.is_file()
            || self.config.fused_ply_path.metadata()?.len() == 0
        {
            return Err(DetectError::Source(format!(
                "COLMAP stereo_fusion did not create a non-empty fused PLY at {}",
                self.config.fused_ply_path.display()
            )));
        }
        let report = self.artifact_report()?;
        if report.loaded_point_count == 0 {
            return Err(DetectError::Source(
                "COLMAP fused PLY contained no loadable finite vertices".to_string(),
            ));
        }
        let dense = DenseReconstruction::new(
            read_ply_point_cloud(&self.config.fused_ply_path, self.config.max_loaded_points)?
                .point_cloud,
        );
        let output_report = dense_report(self.name(), request.views.views.len(), &dense);
        Ok(MvsOutput {
            dense,
            report: output_report,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Report describing COLMAP dense MVS artifacts.
pub struct ColmapMvsArtifactReport {
    /// Dense workspace directory.
    pub workspace_dir: PathBuf,
    /// Fused PLY path.
    pub fused_ply_path: PathBuf,
    /// Fused PLY size in bytes.
    pub fused_ply_size_bytes: u64,
    /// Full vertex count declared by the PLY header.
    pub fused_vertex_count: usize,
    /// Number of finite points loaded into the returned point cloud.
    pub loaded_point_count: usize,
}

#[derive(Debug, Clone)]
struct PlyPointCloudRead {
    point_cloud: PointCloud,
    vertex_count: usize,
    loaded_point_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlyFormat {
    Ascii,
    BinaryLittleEndian,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlyScalarKind {
    Float,
    Double,
    Char,
    UChar,
    Short,
    UShort,
    Int,
    UInt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlyProperty {
    name: String,
    kind: PlyScalarKind,
}

fn invalid_argument(message: impl Into<String>) -> DetectError {
    DetectError::InvalidArgument(message.into())
}

fn is_image_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "jpg" | "jpeg" | "png"
            )
        })
        .unwrap_or(false)
}

fn run_colmap_command(label: &str, command: &mut Command) -> Result<()> {
    let output = command.output()?;
    if output.status.success() {
        return Ok(());
    }
    let code = output
        .status
        .code()
        .map(|value| value.to_string())
        .unwrap_or_else(|| "signal".to_string());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(DetectError::Source(format!(
        "COLMAP {label} failed with exit code {code}; stderr: {}; stdout tail: {}",
        tail(&stderr),
        tail(&stdout)
    )))
}

fn tail(value: &str) -> String {
    const MAX: usize = 2000;
    if value.len() <= MAX {
        value.to_string()
    } else {
        value[value.len() - MAX..].to_string()
    }
}

fn artifact_report_from_ply(
    workspace_dir: &Path,
    fused_ply_path: &Path,
    max_loaded_points: usize,
) -> Result<ColmapMvsArtifactReport> {
    let read = read_ply_point_cloud(fused_ply_path, max_loaded_points)?;
    Ok(ColmapMvsArtifactReport {
        workspace_dir: workspace_dir.to_path_buf(),
        fused_ply_path: fused_ply_path.to_path_buf(),
        fused_ply_size_bytes: fused_ply_path.metadata()?.len(),
        fused_vertex_count: read.vertex_count,
        loaded_point_count: read.loaded_point_count,
    })
}

fn read_ply_point_cloud(path: &Path, max_loaded_points: usize) -> Result<PlyPointCloudRead> {
    let mut bytes = Vec::new();
    fs::File::open(path)?.read_to_end(&mut bytes)?;
    let (header, data_start) = split_ply_header(&bytes)?;
    let (format, vertex_count, properties) = parse_ply_header(header)?;
    let points = match format {
        PlyFormat::Ascii => read_ascii_ply_points(
            &bytes[data_start..],
            vertex_count,
            &properties,
            max_loaded_points,
        )?,
        PlyFormat::BinaryLittleEndian => read_binary_ply_points(
            &bytes[data_start..],
            vertex_count,
            &properties,
            max_loaded_points,
        )?,
    };
    let loaded_point_count = points.len();
    Ok(PlyPointCloudRead {
        point_cloud: PointCloud::new(points)?,
        vertex_count,
        loaded_point_count,
    })
}

fn split_ply_header(bytes: &[u8]) -> Result<(&str, usize)> {
    let marker = b"end_header";
    let offset = bytes
        .windows(marker.len())
        .position(|window| window == marker)
        .ok_or_else(|| invalid_argument("PLY is missing end_header"))?;
    let after_marker = offset + marker.len();
    let data_start =
        if bytes.get(after_marker) == Some(&b'\r') && bytes.get(after_marker + 1) == Some(&b'\n') {
            after_marker + 2
        } else if bytes.get(after_marker) == Some(&b'\n') {
            after_marker + 1
        } else {
            after_marker
        };
    let header = std::str::from_utf8(&bytes[..after_marker])
        .map_err(|error| invalid_argument(format!("PLY header must be UTF-8: {error}")))?;
    Ok((header, data_start))
}

fn parse_ply_header(header: &str) -> Result<(PlyFormat, usize, Vec<PlyProperty>)> {
    let mut lines = header.lines();
    if lines.next().map(str::trim) != Some("ply") {
        return Err(invalid_argument("PLY is missing magic header"));
    }
    let mut format = None;
    let mut vertex_count = None;
    let mut in_vertex = false;
    let mut properties = Vec::new();
    for line in lines {
        let parts = line.split_whitespace().collect::<Vec<_>>();
        match parts.as_slice() {
            ["format", "ascii", "1.0"] => format = Some(PlyFormat::Ascii),
            ["format", "binary_little_endian", "1.0"] => {
                format = Some(PlyFormat::BinaryLittleEndian)
            }
            ["format", other, ..] => {
                return Err(invalid_argument(format!("unsupported PLY format {other}")));
            }
            ["element", "vertex", count] => {
                vertex_count = Some(
                    count
                        .parse::<usize>()
                        .map_err(|_| invalid_argument("invalid PLY vertex count"))?,
                );
                in_vertex = true;
            }
            ["element", ..] => in_vertex = false,
            ["property", kind, name] if in_vertex => properties.push(PlyProperty {
                name: (*name).to_string(),
                kind: parse_ply_scalar_kind(kind)?,
            }),
            _ => {}
        }
    }
    let vertex_count =
        vertex_count.ok_or_else(|| invalid_argument("PLY is missing vertex element"))?;
    for name in ["x", "y", "z"] {
        if !properties.iter().any(|property| property.name == name) {
            return Err(invalid_argument(format!(
                "PLY vertex is missing property {name}"
            )));
        }
    }
    Ok((
        format.ok_or_else(|| invalid_argument("PLY is missing format"))?,
        vertex_count,
        properties,
    ))
}

fn parse_ply_scalar_kind(value: &str) -> Result<PlyScalarKind> {
    match value {
        "float" | "float32" => Ok(PlyScalarKind::Float),
        "double" | "float64" => Ok(PlyScalarKind::Double),
        "char" | "int8" => Ok(PlyScalarKind::Char),
        "uchar" | "uint8" => Ok(PlyScalarKind::UChar),
        "short" | "int16" => Ok(PlyScalarKind::Short),
        "ushort" | "uint16" => Ok(PlyScalarKind::UShort),
        "int" | "int32" => Ok(PlyScalarKind::Int),
        "uint" | "uint32" => Ok(PlyScalarKind::UInt),
        other => Err(invalid_argument(format!(
            "unsupported PLY scalar type {other}"
        ))),
    }
}

fn read_ascii_ply_points(
    bytes: &[u8],
    vertex_count: usize,
    properties: &[PlyProperty],
    max_loaded_points: usize,
) -> Result<Vec<Point3>> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| invalid_argument(format!("ASCII PLY data must be UTF-8: {error}")))?;
    let mut points = Vec::new();
    for line in text.lines().take(vertex_count) {
        if points.len() >= max_loaded_points {
            break;
        }
        let values = line.split_whitespace().collect::<Vec<_>>();
        if values.len() < properties.len() {
            return Err(invalid_argument("ASCII PLY vertex row has too few values"));
        }
        let mut xyz = [0.0; 3];
        for (index, property) in properties.iter().enumerate() {
            let value = values[index]
                .parse::<f32>()
                .map_err(|_| invalid_argument("invalid ASCII PLY scalar"))?;
            assign_xyz(&mut xyz, &property.name, value);
        }
        push_finite_point(&mut points, xyz);
    }
    Ok(points)
}

fn read_binary_ply_points(
    bytes: &[u8],
    vertex_count: usize,
    properties: &[PlyProperty],
    max_loaded_points: usize,
) -> Result<Vec<Point3>> {
    let mut offset = 0;
    let mut points = Vec::new();
    for _ in 0..vertex_count {
        let mut xyz = [0.0; 3];
        for property in properties {
            let value = read_binary_scalar(bytes, &mut offset, property.kind)?;
            assign_xyz(&mut xyz, &property.name, value);
        }
        if points.len() < max_loaded_points {
            push_finite_point(&mut points, xyz);
        }
    }
    Ok(points)
}

fn assign_xyz(xyz: &mut [f32; 3], name: &str, value: f32) {
    match name {
        "x" => xyz[0] = value,
        "y" => xyz[1] = value,
        "z" => xyz[2] = value,
        _ => {}
    }
}

fn push_finite_point(points: &mut Vec<Point3>, xyz: [f32; 3]) {
    if xyz.iter().all(|value| value.is_finite()) {
        points.push(Point3::new(xyz[0], xyz[1], xyz[2]));
    }
}

fn read_binary_scalar(bytes: &[u8], offset: &mut usize, kind: PlyScalarKind) -> Result<f32> {
    macro_rules! take {
        ($len:expr) => {{
            if *offset + $len > bytes.len() {
                return Err(invalid_argument(
                    "binary PLY ended before all vertices were read",
                ));
            }
            let slice = &bytes[*offset..*offset + $len];
            *offset += $len;
            slice
        }};
    }
    Ok(match kind {
        PlyScalarKind::Float => f32::from_le_bytes(take!(4).try_into().unwrap()),
        PlyScalarKind::Double => f64::from_le_bytes(take!(8).try_into().unwrap()) as f32,
        PlyScalarKind::Char => i8::from_le_bytes(take!(1).try_into().unwrap()) as f32,
        PlyScalarKind::UChar => u8::from_le_bytes(take!(1).try_into().unwrap()) as f32,
        PlyScalarKind::Short => i16::from_le_bytes(take!(2).try_into().unwrap()) as f32,
        PlyScalarKind::UShort => u16::from_le_bytes(take!(2).try_into().unwrap()) as f32,
        PlyScalarKind::Int => i32::from_le_bytes(take!(4).try_into().unwrap()) as f32,
        PlyScalarKind::UInt => u32::from_le_bytes(take!(4).try_into().unwrap()) as f32,
    })
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::tempdir;
    use video_analysis_radiance_fields::{CameraIntrinsics, CameraPose, CameraView, CameraViewSet};
    use video_analysis_reconstruction::SparseReconstruction;

    use super::*;

    fn config_in(temp: &Path) -> ColmapMvsConfig {
        ColmapMvsConfig {
            colmap_command: PathBuf::from("colmap"),
            image_dir: temp.join("frames"),
            sparse_model_dir: temp.join("sparse/0"),
            workspace_dir: temp.join("dense"),
            fused_ply_path: temp.join("dense/fused.ply"),
            use_gpu: false,
            max_loaded_points: 50_000,
        }
    }

    fn write_valid_inputs(config: &ColmapMvsConfig) {
        fs::create_dir_all(&config.image_dir).unwrap();
        fs::write(config.image_dir.join("a.jpg"), b"fixture").unwrap();
        fs::write(config.image_dir.join("b.jpg"), b"fixture").unwrap();
        fs::create_dir_all(&config.sparse_model_dir).unwrap();
        for name in ["cameras.bin", "images.bin", "points3D.bin"] {
            fs::write(config.sparse_model_dir.join(name), b"fixture").unwrap();
        }
    }

    fn request() -> MvsRequest {
        let intrinsics = CameraIntrinsics::pinhole(16, 16, 1.0).unwrap();
        let views = CameraViewSet {
            views: vec![
                CameraView {
                    id: 1,
                    name: "a.jpg".to_string(),
                    intrinsics,
                    distortion: None,
                    pose: CameraPose::identity(),
                },
                CameraView {
                    id: 2,
                    name: "b.jpg".to_string(),
                    intrinsics,
                    distortion: None,
                    pose: CameraPose::identity(),
                },
            ],
        };
        MvsRequest::new(SparseReconstruction::new(), views).unwrap()
    }

    #[test]
    fn config_rejects_missing_image_dir() {
        let temp = tempdir().unwrap();
        let config = config_in(temp.path());

        let error = config.validate().unwrap_err();

        assert!(error.to_string().contains("image dir"));
    }

    #[test]
    fn config_rejects_missing_sparse_binary_files() {
        let temp = tempdir().unwrap();
        let config = config_in(temp.path());
        fs::create_dir_all(&config.image_dir).unwrap();
        fs::write(config.image_dir.join("a.jpg"), b"fixture").unwrap();
        fs::write(config.image_dir.join("b.jpg"), b"fixture").unwrap();

        let error = config.validate().unwrap_err();

        assert!(error.to_string().contains("cameras.bin"));
    }

    #[test]
    fn ply_parser_reads_minimal_ascii_ply() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("cloud.ply");
        fs::write(
            &path,
            "ply\nformat ascii 1.0\nelement vertex 2\nproperty float x\nproperty float y\nproperty float z\nproperty uchar red\nend_header\n1 2 3 255\n4 5 6 0\n",
        )
        .unwrap();

        let read = read_ply_point_cloud(&path, 50_000).unwrap();

        assert_eq!(read.vertex_count, 2);
        assert_eq!(read.point_cloud.points().len(), 2);
        assert_eq!(read.point_cloud.points()[1], Point3::new(4.0, 5.0, 6.0));
    }

    #[test]
    fn ply_parser_reads_minimal_binary_little_endian_ply() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("cloud.ply");
        let mut bytes = b"ply\nformat binary_little_endian 1.0\nelement vertex 1\nproperty float x\nproperty float y\nproperty float z\nend_header\n".to_vec();
        for value in [1.0_f32, 2.0, 3.0] {
            bytes.write_all(&value.to_le_bytes()).unwrap();
        }
        fs::write(&path, bytes).unwrap();

        let read = read_ply_point_cloud(&path, 50_000).unwrap();

        assert_eq!(read.vertex_count, 1);
        assert_eq!(read.point_cloud.points(), &[Point3::new(1.0, 2.0, 3.0)]);
    }

    #[test]
    fn backend_command_failure_reports_command_label_and_stderr() {
        let temp = tempdir().unwrap();
        let mut config = config_in(temp.path());
        config.colmap_command = PathBuf::from("false");
        write_valid_inputs(&config);
        let mut backend = ColmapMvsBackend::new(config).unwrap();

        let error = backend.reconstruct_dense(&request()).unwrap_err();

        assert!(error.to_string().contains("image_undistorter"));
        assert!(error.to_string().contains("stderr"));
    }
}
