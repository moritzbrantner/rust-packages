#![doc = include_str!("../README.md")]

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde::{Deserialize, Serialize};
use three_d_processing_core::{Point3, PointCloud};
use three_d_processing_mesh::{Mesh, Triangle};
use video_analysis_core::{DetectError, Result};

pub fn read_mesh(path: impl AsRef<Path>) -> Result<Mesh> {
    match extension(path.as_ref()) {
        Some("obj") => read_obj_mesh(path),
        Some("ply") => read_ply_mesh(path),
        Some("gltf") => read_gltf_mesh(path),
        _ => Err(invalid_argument(format!(
            "unsupported mesh extension for `{}`",
            path.as_ref().display()
        ))),
    }
}

pub fn write_mesh(path: impl AsRef<Path>, mesh: &Mesh) -> Result<()> {
    match extension(path.as_ref()) {
        Some("obj") => write_obj_mesh(path, mesh),
        Some("ply") => write_ply_mesh(path, mesh),
        Some("gltf") => write_gltf_mesh(path, mesh),
        _ => Err(invalid_argument(format!(
            "unsupported mesh extension for `{}`",
            path.as_ref().display()
        ))),
    }
}

pub fn read_obj_mesh(path: impl AsRef<Path>) -> Result<Mesh> {
    let file = fs::File::open(path.as_ref())?;
    let reader = BufReader::new(file);
    let mut vertices = Vec::new();
    let mut triangles = Vec::new();
    for (line_index, line) in reader.lines().enumerate() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(rest) = line.strip_prefix("v ") {
            let parts = rest.split_whitespace().collect::<Vec<_>>();
            if parts.len() != 3 {
                return Err(invalid_argument(format!(
                    "OBJ vertex line {} must have 3 components",
                    line_index + 1
                )));
            }
            vertices.push(Point3::new(
                parse_f32(parts[0], "vertex x")?,
                parse_f32(parts[1], "vertex y")?,
                parse_f32(parts[2], "vertex z")?,
            ));
        } else if let Some(rest) = line.strip_prefix("f ") {
            let indices = rest
                .split_whitespace()
                .map(|part| {
                    let index = part.split('/').next().unwrap_or_default();
                    let index = index.parse::<usize>().map_err(|err| {
                        invalid_argument(format!("invalid OBJ face index `{index}`: {err}"))
                    })?;
                    index
                        .checked_sub(1)
                        .ok_or_else(|| invalid_argument("OBJ indices must be 1-based and non-zero"))
                })
                .collect::<Result<Vec<_>>>()?;
            if indices.len() < 3 {
                return Err(invalid_argument("OBJ face must contain at least 3 indices"));
            }
            for fan in 1..indices.len() - 1 {
                triangles.push(Triangle::new(indices[0], indices[fan], indices[fan + 1]));
            }
        }
    }
    Mesh::new(vertices, triangles)
}

pub fn write_obj_mesh(path: impl AsRef<Path>, mesh: &Mesh) -> Result<()> {
    mesh.validate()?;
    let mut output = String::new();
    for vertex in &mesh.vertices {
        output.push_str(&format!("v {} {} {}\n", vertex.x, vertex.y, vertex.z));
    }
    for triangle in &mesh.triangles {
        output.push_str(&format!(
            "f {} {} {}\n",
            triangle.vertices[0] + 1,
            triangle.vertices[1] + 1,
            triangle.vertices[2] + 1
        ));
    }
    fs::write(path, output)?;
    Ok(())
}

pub fn read_ply_mesh(path: impl AsRef<Path>) -> Result<Mesh> {
    let file = fs::File::open(path.as_ref())?;
    let mut reader = BufReader::new(file);
    let header = read_ply_header(&mut reader)?;
    let mut vertices = Vec::with_capacity(header.vertex_count);
    for _ in 0..header.vertex_count {
        let line = read_line(&mut reader)?;
        let parts = line.split_whitespace().collect::<Vec<_>>();
        if parts.len() < 3 {
            return Err(invalid_argument("PLY vertex line requires x y z"));
        }
        vertices.push(Point3::new(
            parse_f32(parts[0], "vertex x")?,
            parse_f32(parts[1], "vertex y")?,
            parse_f32(parts[2], "vertex z")?,
        ));
    }
    let mut triangles = Vec::with_capacity(header.face_count);
    for _ in 0..header.face_count {
        let line = read_line(&mut reader)?;
        let parts = line.split_whitespace().collect::<Vec<_>>();
        if parts.is_empty() {
            return Err(invalid_argument("PLY face line must not be empty"));
        }
        let count = parts[0]
            .parse::<usize>()
            .map_err(|err| invalid_argument(format!("invalid PLY face count: {err}")))?;
        if parts.len() != count + 1 || count < 3 {
            return Err(invalid_argument(
                "PLY face line must match the declared polygon size",
            ));
        }
        let indices = parts[1..]
            .iter()
            .map(|value| {
                value.parse::<usize>().map_err(|err| {
                    invalid_argument(format!("invalid PLY face index `{value}`: {err}"))
                })
            })
            .collect::<Result<Vec<_>>>()?;
        for fan in 1..indices.len() - 1 {
            triangles.push(Triangle::new(indices[0], indices[fan], indices[fan + 1]));
        }
    }
    Mesh::new(vertices, triangles)
}

pub fn write_ply_mesh(path: impl AsRef<Path>, mesh: &Mesh) -> Result<()> {
    mesh.validate()?;
    let mut output = String::new();
    output.push_str("ply\nformat ascii 1.0\n");
    output.push_str(&format!("element vertex {}\n", mesh.vertices.len()));
    output.push_str("property float x\nproperty float y\nproperty float z\n");
    output.push_str(&format!("element face {}\n", mesh.triangles.len()));
    output.push_str("property list uchar int vertex_indices\nend_header\n");
    for vertex in &mesh.vertices {
        output.push_str(&format!("{} {} {}\n", vertex.x, vertex.y, vertex.z));
    }
    for triangle in &mesh.triangles {
        output.push_str(&format!(
            "3 {} {} {}\n",
            triangle.vertices[0], triangle.vertices[1], triangle.vertices[2]
        ));
    }
    fs::write(path, output)?;
    Ok(())
}

pub fn read_ply_point_cloud(path: impl AsRef<Path>) -> Result<PointCloud> {
    let file = fs::File::open(path.as_ref())?;
    let mut reader = BufReader::new(file);
    let header = read_ply_header(&mut reader)?;
    let mut points = Vec::with_capacity(header.vertex_count);
    for _ in 0..header.vertex_count {
        let line = read_line(&mut reader)?;
        let parts = line.split_whitespace().collect::<Vec<_>>();
        if parts.len() < 3 {
            return Err(invalid_argument("PLY vertex line requires x y z"));
        }
        points.push(Point3::new(
            parse_f32(parts[0], "point x")?,
            parse_f32(parts[1], "point y")?,
            parse_f32(parts[2], "point z")?,
        ));
    }
    PointCloud::new(points)
}

pub fn write_ply_point_cloud(path: impl AsRef<Path>, cloud: &PointCloud) -> Result<()> {
    let mut output = String::new();
    output.push_str("ply\nformat ascii 1.0\n");
    output.push_str(&format!("element vertex {}\n", cloud.points().len()));
    output.push_str("property float x\nproperty float y\nproperty float z\nend_header\n");
    for point in cloud.points() {
        output.push_str(&format!("{} {} {}\n", point.x, point.y, point.z));
    }
    fs::write(path, output)?;
    Ok(())
}

pub fn read_gltf_mesh(path: impl AsRef<Path>) -> Result<Mesh> {
    let data = fs::read(path.as_ref())?;
    let document: MinimalGltf = serde_json::from_slice(&data)
        .map_err(|err| invalid_argument(format!("failed to parse glTF JSON: {err}")))?;
    let buffer = document
        .buffers
        .first()
        .ok_or_else(|| invalid_argument("glTF file must contain one buffer"))?;
    let encoded = buffer
        .uri
        .strip_prefix("data:application/octet-stream;base64,")
        .ok_or_else(|| invalid_argument("glTF buffer must use an embedded base64 URI"))?;
    let bytes = BASE64
        .decode(encoded)
        .map_err(|err| invalid_argument(format!("invalid base64 glTF buffer: {err}")))?;
    let primitive = document
        .meshes
        .first()
        .and_then(|mesh| mesh.primitives.first())
        .ok_or_else(|| invalid_argument("glTF file must contain one mesh primitive"))?;
    let position_accessor = primitive
        .attributes
        .position
        .ok_or_else(|| invalid_argument("glTF primitive must expose POSITION"))?;
    let positions = decode_vec3_accessor(&document, &bytes, position_accessor)?;
    let indices = if let Some(accessor) = primitive.indices {
        decode_index_accessor(&document, &bytes, accessor)?
    } else {
        (0..positions.len()).collect()
    };
    if indices.len() % 3 != 0 {
        return Err(invalid_argument(
            "glTF triangle index count must be divisible by 3",
        ));
    }
    let triangles = indices
        .chunks_exact(3)
        .map(|chunk| Triangle::new(chunk[0], chunk[1], chunk[2]))
        .collect::<Vec<_>>();
    Mesh::new(positions, triangles)
}

pub fn write_gltf_mesh(path: impl AsRef<Path>, mesh: &Mesh) -> Result<()> {
    mesh.validate()?;
    let mut bytes = Vec::new();
    let position_offset = 0_u32;
    for vertex in &mesh.vertices {
        bytes.extend_from_slice(&vertex.x.to_le_bytes());
        bytes.extend_from_slice(&vertex.y.to_le_bytes());
        bytes.extend_from_slice(&vertex.z.to_le_bytes());
    }
    let mut index_bytes = Vec::new();
    for triangle in &mesh.triangles {
        for index in triangle.vertices {
            index_bytes.extend_from_slice(&(index as u32).to_le_bytes());
        }
    }
    let index_offset = bytes.len() as u32;
    bytes.extend_from_slice(&index_bytes);

    let document = MinimalGltf {
        asset: GltfAsset {
            version: "2.0".to_string(),
        },
        buffers: vec![GltfBuffer {
            byte_length: bytes.len() as u32,
            uri: format!(
                "data:application/octet-stream;base64,{}",
                BASE64.encode(bytes)
            ),
        }],
        buffer_views: vec![
            GltfBufferView {
                buffer: 0,
                byte_offset: position_offset,
                byte_length: (mesh.vertices.len() * 12) as u32,
                target: Some(34962),
            },
            GltfBufferView {
                buffer: 0,
                byte_offset: index_offset,
                byte_length: index_bytes.len() as u32,
                target: Some(34963),
            },
        ],
        accessors: vec![
            GltfAccessor {
                buffer_view: 0,
                byte_offset: 0,
                component_type: 5126,
                count: mesh.vertices.len() as u32,
                accessor_type: "VEC3".to_string(),
            },
            GltfAccessor {
                buffer_view: 1,
                byte_offset: 0,
                component_type: 5125,
                count: (mesh.triangles.len() * 3) as u32,
                accessor_type: "SCALAR".to_string(),
            },
        ],
        meshes: vec![GltfMesh {
            primitives: vec![GltfPrimitive {
                attributes: GltfAttributes { position: Some(0) },
                indices: Some(1),
                mode: Some(4),
            }],
        }],
        scenes: vec![GltfScene { nodes: vec![0] }],
        nodes: vec![GltfNode { mesh: Some(0) }],
        scene: Some(0),
    };
    let encoded = serde_json::to_vec_pretty(&document)
        .map_err(|err| invalid_argument(format!("failed to encode glTF JSON: {err}")))?;
    fs::write(path, encoded)?;
    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct PlyHeader {
    vertex_count: usize,
    face_count: usize,
}

fn read_ply_header(reader: &mut BufReader<fs::File>) -> Result<PlyHeader> {
    let first = read_line(reader)?;
    if first.trim() != "ply" {
        return Err(invalid_argument("PLY file must start with `ply`"));
    }
    let mut vertex_count = None;
    let mut face_count = 0;
    loop {
        let line = read_line(reader)?;
        let trimmed = line.trim();
        if trimmed == "end_header" {
            break;
        }
        if let Some(rest) = trimmed.strip_prefix("element vertex ") {
            vertex_count = Some(rest.parse::<usize>().map_err(|err| {
                invalid_argument(format!("invalid PLY vertex count `{rest}`: {err}"))
            })?);
        } else if let Some(rest) = trimmed.strip_prefix("element face ") {
            face_count = rest.parse::<usize>().map_err(|err| {
                invalid_argument(format!("invalid PLY face count `{rest}`: {err}"))
            })?;
        }
    }
    Ok(PlyHeader {
        vertex_count: vertex_count
            .ok_or_else(|| invalid_argument("PLY header must declare vertex count"))?,
        face_count,
    })
}

fn read_line(reader: &mut BufReader<fs::File>) -> Result<String> {
    let mut line = String::new();
    let bytes = reader.read_line(&mut line)?;
    if bytes == 0 {
        return Err(invalid_argument("unexpected end of file"));
    }
    Ok(line)
}

fn parse_f32(value: &str, label: &str) -> Result<f32> {
    value
        .parse::<f32>()
        .map_err(|err| invalid_argument(format!("invalid {label} `{value}`: {err}")))
}

fn extension(path: &Path) -> Option<&str> {
    path.extension().and_then(|value| value.to_str())
}

fn invalid_argument(message: impl Into<String>) -> DetectError {
    DetectError::InvalidArgument(message.into())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MinimalGltf {
    asset: GltfAsset,
    buffers: Vec<GltfBuffer>,
    #[serde(rename = "bufferViews")]
    buffer_views: Vec<GltfBufferView>,
    accessors: Vec<GltfAccessor>,
    meshes: Vec<GltfMesh>,
    scenes: Vec<GltfScene>,
    nodes: Vec<GltfNode>,
    scene: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GltfAsset {
    version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GltfBuffer {
    #[serde(rename = "byteLength")]
    byte_length: u32,
    uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GltfBufferView {
    buffer: usize,
    #[serde(rename = "byteOffset")]
    byte_offset: u32,
    #[serde(rename = "byteLength")]
    byte_length: u32,
    target: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GltfAccessor {
    #[serde(rename = "bufferView")]
    buffer_view: usize,
    #[serde(rename = "byteOffset")]
    byte_offset: u32,
    #[serde(rename = "componentType")]
    component_type: u32,
    count: u32,
    #[serde(rename = "type")]
    accessor_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GltfMesh {
    primitives: Vec<GltfPrimitive>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GltfPrimitive {
    attributes: GltfAttributes,
    indices: Option<usize>,
    mode: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GltfAttributes {
    #[serde(rename = "POSITION")]
    position: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GltfScene {
    nodes: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GltfNode {
    mesh: Option<usize>,
}

fn decode_vec3_accessor(
    document: &MinimalGltf,
    bytes: &[u8],
    accessor_index: usize,
) -> Result<Vec<Point3>> {
    let accessor = document
        .accessors
        .get(accessor_index)
        .ok_or_else(|| invalid_argument("glTF POSITION accessor is missing"))?;
    if accessor.component_type != 5126 || accessor.accessor_type != "VEC3" {
        return Err(invalid_argument(
            "glTF POSITION accessor must use float VEC3 values",
        ));
    }
    let view = document
        .buffer_views
        .get(accessor.buffer_view)
        .ok_or_else(|| invalid_argument("glTF POSITION bufferView is missing"))?;
    let start = (view.byte_offset + accessor.byte_offset) as usize;
    let end = start + accessor.count as usize * 12;
    let slice = bytes
        .get(start..end)
        .ok_or_else(|| invalid_argument("glTF POSITION accessor exceeds buffer length"))?;
    let mut points = Vec::with_capacity(accessor.count as usize);
    for chunk in slice.chunks_exact(12) {
        points.push(Point3::new(
            f32::from_le_bytes(chunk[0..4].try_into().unwrap()),
            f32::from_le_bytes(chunk[4..8].try_into().unwrap()),
            f32::from_le_bytes(chunk[8..12].try_into().unwrap()),
        ));
    }
    Ok(points)
}

fn decode_index_accessor(
    document: &MinimalGltf,
    bytes: &[u8],
    accessor_index: usize,
) -> Result<Vec<usize>> {
    let accessor = document
        .accessors
        .get(accessor_index)
        .ok_or_else(|| invalid_argument("glTF index accessor is missing"))?;
    if accessor.component_type != 5125 || accessor.accessor_type != "SCALAR" {
        return Err(invalid_argument(
            "glTF index accessor must use unsigned int SCALAR values",
        ));
    }
    let view = document
        .buffer_views
        .get(accessor.buffer_view)
        .ok_or_else(|| invalid_argument("glTF index bufferView is missing"))?;
    let start = (view.byte_offset + accessor.byte_offset) as usize;
    let end = start + accessor.count as usize * 4;
    let slice = bytes
        .get(start..end)
        .ok_or_else(|| invalid_argument("glTF index accessor exceeds buffer length"))?;
    Ok(slice
        .chunks_exact(4)
        .map(|chunk| u32::from_le_bytes(chunk.try_into().unwrap()) as usize)
        .collect())
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    fn mesh() -> Mesh {
        Mesh::new(
            [
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
            ],
            [Triangle::new(0, 1, 2)],
        )
        .unwrap()
    }

    #[test]
    fn obj_ply_and_gltf_round_trip() {
        let dir = tempdir().unwrap();
        let mesh = mesh();

        let obj = dir.path().join("mesh.obj");
        write_obj_mesh(&obj, &mesh).unwrap();
        assert_eq!(read_obj_mesh(&obj).unwrap(), mesh);

        let ply = dir.path().join("mesh.ply");
        write_ply_mesh(&ply, &mesh).unwrap();
        assert_eq!(read_ply_mesh(&ply).unwrap(), mesh);

        let gltf = dir.path().join("mesh.gltf");
        write_gltf_mesh(&gltf, &mesh).unwrap();
        assert_eq!(read_gltf_mesh(&gltf).unwrap(), mesh);
    }

    #[test]
    fn point_cloud_ply_round_trip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("cloud.ply");
        let cloud =
            PointCloud::new([Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 2.0, 3.0)]).unwrap();
        write_ply_point_cloud(&path, &cloud).unwrap();
        assert_eq!(read_ply_point_cloud(&path).unwrap(), cloud);
    }
}
