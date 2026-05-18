#![doc = include_str!("../README.md")]

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};
use three_d_processing_core::{Bounds3, Point3, RigidTransform3, Transform3, Vector3};
use video_analysis_core::{DetectError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
/// Data type for edge.
pub struct Edge {
    /// The vertices value.
    pub vertices: [usize; 2],
}

impl Edge {
    /// Creates a new value.
    pub fn new(a: usize, b: usize) -> Result<Self> {
        if a == b {
            return Err(invalid_argument(
                "edge must reference two distinct vertices",
            ));
        }
        Ok(Self {
            vertices: if a < b { [a, b] } else { [b, a] },
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
/// Data type for triangle.
pub struct Triangle {
    /// The vertices value.
    pub vertices: [usize; 3],
}

impl Triangle {
    /// Creates a new value.
    pub const fn new(a: usize, b: usize, c: usize) -> Self {
        Self {
            vertices: [a, b, c],
        }
    }

    /// Returns edges.
    pub fn edges(self) -> Result<[Edge; 3]> {
        Ok([
            Edge::new(self.vertices[0], self.vertices[1])?,
            Edge::new(self.vertices[1], self.vertices[2])?,
            Edge::new(self.vertices[0], self.vertices[2])?,
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Data type for mesh topology.
pub struct MeshTopology {
    /// The edges value.
    pub edges: Vec<Edge>,
    /// The boundary edges value.
    pub boundary_edges: Vec<Edge>,
    /// The vertex neighbors value.
    pub vertex_neighbors: Vec<Vec<usize>>,
    /// The triangle neighbors value.
    pub triangle_neighbors: Vec<Vec<usize>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Data type for mesh.
pub struct Mesh {
    /// The vertices value.
    pub vertices: Vec<Point3>,
    /// The triangles value.
    pub triangles: Vec<Triangle>,
}

impl Mesh {
    /// Creates a new value.
    pub fn new(
        vertices: impl Into<Vec<Point3>>,
        triangles: impl Into<Vec<Triangle>>,
    ) -> Result<Self> {
        let mesh = Self {
            vertices: vertices.into(),
            triangles: triangles.into(),
        };
        mesh.validate()?;
        Ok(mesh)
    }

    /// Validates this value.
    pub fn validate(&self) -> Result<()> {
        if self.vertices.iter().any(|vertex| !vertex.is_finite()) {
            return Err(invalid_argument("mesh vertices must be finite"));
        }
        for triangle in &self.triangles {
            for index in triangle.vertices {
                if index >= self.vertices.len() {
                    return Err(invalid_argument("triangle vertex index is out of bounds"));
                }
            }
            if triangle.vertices[0] == triangle.vertices[1]
                || triangle.vertices[1] == triangle.vertices[2]
                || triangle.vertices[0] == triangle.vertices[2]
            {
                return Err(invalid_argument(
                    "triangle must reference three distinct vertices",
                ));
            }
        }
        Ok(())
    }

    /// Returns bounds.
    pub fn bounds(&self) -> Result<Option<Bounds3>> {
        Bounds3::from_points(&self.vertices)
    }

    /// Returns surface area.
    pub fn surface_area(&self) -> Result<f32> {
        surface_area(self)
    }

    /// Returns face normals.
    pub fn face_normals(&self) -> Result<Vec<Vector3>> {
        face_normals(self)
    }

    /// Returns vertex normals.
    pub fn vertex_normals(&self) -> Result<Vec<Vector3>> {
        vertex_normals(self)
    }

    /// Returns topology.
    pub fn topology(&self) -> Result<MeshTopology> {
        mesh_topology(self)
    }

    /// Returns connected components.
    pub fn connected_components(&self) -> Result<Vec<Mesh>> {
        connected_components(self)
    }

    /// Returns whether is manifold.
    pub fn is_manifold(&self) -> Result<bool> {
        is_manifold(self)
    }

    /// Returns whether is watertight.
    pub fn is_watertight(&self) -> Result<bool> {
        is_watertight(self)
    }

    /// Returns volume.
    pub fn volume(&self) -> Result<f32> {
        volume(self)
    }

    /// Returns transformed.
    pub fn transformed(&self, transform: Transform3) -> Result<Self> {
        Mesh::new(
            self.vertices
                .iter()
                .copied()
                .map(|vertex| transform.apply_point(vertex))
                .collect::<Vec<_>>(),
            self.triangles.clone(),
        )
    }

    /// Returns transformed rigid.
    pub fn transformed_rigid(&self, transform: RigidTransform3) -> Result<Self> {
        Mesh::new(
            self.vertices
                .iter()
                .copied()
                .map(|vertex| transform.apply_point(vertex))
                .collect::<Result<Vec<_>>>()?,
            self.triangles.clone(),
        )
    }

    /// Returns merged with.
    pub fn merged_with(&self, other: &Mesh) -> Result<Self> {
        merge_meshes([self, other])
    }

    /// Returns sample points uniform.
    pub fn sample_points_uniform(&self, sample_count: usize) -> Result<Vec<Point3>> {
        sample_points_uniform(self, sample_count)
    }

    /// Returns laplacian smooth.
    pub fn laplacian_smooth(&self, iterations: usize, lambda: f32) -> Result<Self> {
        laplacian_smooth(self, iterations, lambda)
    }
}

/// Returns triangle normal.
pub fn triangle_normal(mesh: &Mesh, triangle: Triangle) -> Result<Vector3> {
    mesh.validate()?;
    for index in triangle.vertices {
        if index >= mesh.vertices.len() {
            return Err(invalid_argument("triangle vertex index is out of bounds"));
        }
    }
    let a = mesh.vertices[triangle.vertices[0]];
    let b = mesh.vertices[triangle.vertices[1]];
    let c = mesh.vertices[triangle.vertices[2]];
    (b - a).cross(c - a).normalize()
}

/// Returns triangle area.
pub fn triangle_area(mesh: &Mesh, triangle: Triangle) -> Result<f32> {
    mesh.validate()?;
    for index in triangle.vertices {
        if index >= mesh.vertices.len() {
            return Err(invalid_argument("triangle vertex index is out of bounds"));
        }
    }
    let a = mesh.vertices[triangle.vertices[0]];
    let b = mesh.vertices[triangle.vertices[1]];
    let c = mesh.vertices[triangle.vertices[2]];
    Ok((b - a).cross(c - a).length() * 0.5)
}

/// Returns surface area.
pub fn surface_area(mesh: &Mesh) -> Result<f32> {
    mesh.triangles.iter().try_fold(0.0_f32, |area, triangle| {
        Ok(area + triangle_area(mesh, *triangle)?)
    })
}

/// Returns face normals.
pub fn face_normals(mesh: &Mesh) -> Result<Vec<Vector3>> {
    mesh.triangles
        .iter()
        .copied()
        .map(|triangle| triangle_normal(mesh, triangle))
        .collect()
}

/// Returns vertex normals.
pub fn vertex_normals(mesh: &Mesh) -> Result<Vec<Vector3>> {
    mesh.validate()?;
    let mut normals = vec![Vector3::ZERO; mesh.vertices.len()];
    for triangle in &mesh.triangles {
        let normal = triangle_normal(mesh, *triangle)?;
        for index in triangle.vertices {
            normals[index] += normal;
        }
    }
    for normal in &mut normals {
        if normal.length() > f32::EPSILON {
            *normal = normal.normalize()?;
        }
    }
    Ok(normals)
}

/// Returns mesh topology.
pub fn mesh_topology(mesh: &Mesh) -> Result<MeshTopology> {
    mesh.validate()?;
    let mut edge_map: BTreeMap<Edge, Vec<usize>> = BTreeMap::new();
    let mut vertex_neighbors: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); mesh.vertices.len()];
    let mut triangle_neighbors: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); mesh.triangles.len()];

    for (triangle_index, triangle) in mesh.triangles.iter().copied().enumerate() {
        let edges = triangle.edges()?;
        for edge in edges {
            edge_map.entry(edge).or_default().push(triangle_index);
            vertex_neighbors[edge.vertices[0]].insert(edge.vertices[1]);
            vertex_neighbors[edge.vertices[1]].insert(edge.vertices[0]);
        }
    }

    for triangles in edge_map.values() {
        for &left in triangles {
            for &right in triangles {
                if left != right {
                    triangle_neighbors[left].insert(right);
                }
            }
        }
    }

    let mut edges = edge_map.keys().copied().collect::<Vec<_>>();
    edges.sort();
    let boundary_edges = edge_map
        .iter()
        .filter_map(|(edge, triangles)| (triangles.len() == 1).then_some(*edge))
        .collect::<Vec<_>>();
    Ok(MeshTopology {
        edges,
        boundary_edges,
        vertex_neighbors: vertex_neighbors
            .into_iter()
            .map(|neighbors| neighbors.into_iter().collect())
            .collect(),
        triangle_neighbors: triangle_neighbors
            .into_iter()
            .map(|neighbors| neighbors.into_iter().collect())
            .collect(),
    })
}

/// Returns connected components.
pub fn connected_components(mesh: &Mesh) -> Result<Vec<Mesh>> {
    let topology = mesh_topology(mesh)?;
    let mut visited = vec![false; mesh.triangles.len()];
    let mut components = Vec::new();

    for start in 0..mesh.triangles.len() {
        if visited[start] {
            continue;
        }
        let mut queue = VecDeque::from([start]);
        let mut triangle_indices = Vec::new();
        visited[start] = true;
        while let Some(triangle_index) = queue.pop_front() {
            triangle_indices.push(triangle_index);
            for &neighbor in &topology.triangle_neighbors[triangle_index] {
                if !visited[neighbor] {
                    visited[neighbor] = true;
                    queue.push_back(neighbor);
                }
            }
        }

        let mut used_vertices = BTreeMap::new();
        let mut vertices = Vec::new();
        let mut triangles = Vec::new();
        for triangle_index in triangle_indices {
            let triangle = mesh.triangles[triangle_index];
            let mut remapped = [0_usize; 3];
            for (slot, index) in triangle.vertices.into_iter().enumerate() {
                let mapped = *used_vertices.entry(index).or_insert_with(|| {
                    let next = vertices.len();
                    vertices.push(mesh.vertices[index]);
                    next
                });
                remapped[slot] = mapped;
            }
            triangles.push(Triangle { vertices: remapped });
        }
        components.push(Mesh::new(vertices, triangles)?);
    }
    Ok(components)
}

/// Returns whether is manifold.
pub fn is_manifold(mesh: &Mesh) -> Result<bool> {
    mesh.validate()?;
    let mut edge_counts: BTreeMap<Edge, usize> = BTreeMap::new();
    for triangle in &mesh.triangles {
        for edge in triangle.edges()? {
            *edge_counts.entry(edge).or_default() += 1;
        }
    }
    Ok(edge_counts.values().all(|count| *count <= 2))
}

/// Returns whether is watertight.
pub fn is_watertight(mesh: &Mesh) -> Result<bool> {
    mesh.validate()?;
    let mut edge_counts: BTreeMap<Edge, usize> = BTreeMap::new();
    for triangle in &mesh.triangles {
        for edge in triangle.edges()? {
            *edge_counts.entry(edge).or_default() += 1;
        }
    }
    Ok(edge_counts.values().all(|count| *count == 2))
}

/// Returns volume.
pub fn volume(mesh: &Mesh) -> Result<f32> {
    mesh.validate()?;
    let mut signed_volume = 0.0_f32;
    for triangle in &mesh.triangles {
        let a = mesh.vertices[triangle.vertices[0]];
        let b = mesh.vertices[triangle.vertices[1]];
        let c = mesh.vertices[triangle.vertices[2]];
        let va = Vector3::new(a.x, a.y, a.z);
        let vb = Vector3::new(b.x, b.y, b.z);
        let vc = Vector3::new(c.x, c.y, c.z);
        signed_volume += va.dot(vb.cross(vc)) / 6.0;
    }
    Ok(signed_volume.abs())
}

/// Returns merge meshes.
pub fn merge_meshes<'a>(meshes: impl IntoIterator<Item = &'a Mesh>) -> Result<Mesh> {
    let mut vertices = Vec::new();
    let mut triangles = Vec::new();
    for mesh in meshes {
        mesh.validate()?;
        let base = vertices.len();
        vertices.extend(mesh.vertices.iter().copied());
        triangles.extend(mesh.triangles.iter().map(|triangle| Triangle {
            vertices: [
                triangle.vertices[0] + base,
                triangle.vertices[1] + base,
                triangle.vertices[2] + base,
            ],
        }));
    }
    Mesh::new(vertices, triangles)
}

/// Returns sample points uniform.
pub fn sample_points_uniform(mesh: &Mesh, sample_count: usize) -> Result<Vec<Point3>> {
    mesh.validate()?;
    if sample_count == 0 || mesh.triangles.is_empty() {
        return Ok(Vec::new());
    }
    let areas = mesh
        .triangles
        .iter()
        .copied()
        .map(|triangle| triangle_area(mesh, triangle))
        .collect::<Result<Vec<_>>>()?;
    let total_area: f32 = areas.iter().sum();
    if total_area <= f32::EPSILON {
        return Ok(mesh.vertices.iter().copied().take(sample_count).collect());
    }

    let mut cumulative = Vec::with_capacity(areas.len());
    let mut sum = 0.0_f32;
    for area in &areas {
        sum += *area / total_area;
        cumulative.push(sum);
    }

    let mut points = Vec::with_capacity(sample_count);
    for index in 0..sample_count {
        let t = (index as f32 + 0.5) / sample_count as f32;
        let triangle_index = cumulative
            .iter()
            .position(|value| *value >= t)
            .unwrap_or(cumulative.len() - 1);
        let triangle = mesh.triangles[triangle_index];
        let a = mesh.vertices[triangle.vertices[0]];
        let b = mesh.vertices[triangle.vertices[1]];
        let c = mesh.vertices[triangle.vertices[2]];
        let u = ((index as f32 + 0.5) / sample_count as f32).sqrt();
        let v = fract(index as f32 * 0.618_034);
        let w0 = 1.0 - u;
        let w1 = u * (1.0 - v);
        let w2 = u * v;
        points.push(Point3::new(
            a.x * w0 + b.x * w1 + c.x * w2,
            a.y * w0 + b.y * w1 + c.y * w2,
            a.z * w0 + b.z * w1 + c.z * w2,
        ));
    }
    Ok(points)
}

/// Returns laplacian smooth.
pub fn laplacian_smooth(mesh: &Mesh, iterations: usize, lambda: f32) -> Result<Mesh> {
    mesh.validate()?;
    if !lambda.is_finite() || !(0.0..=1.0).contains(&lambda) {
        return Err(invalid_argument(
            "laplacian lambda must be finite and in the range [0, 1]",
        ));
    }
    let topology = mesh_topology(mesh)?;
    let mut vertices = mesh.vertices.clone();
    for _ in 0..iterations {
        let previous = vertices.clone();
        for (vertex_index, neighbors) in topology.vertex_neighbors.iter().enumerate() {
            if neighbors.is_empty() {
                continue;
            }
            let mean = neighbors.iter().fold(Vector3::ZERO, |sum, neighbor| {
                let point = previous[*neighbor];
                sum + Vector3::new(point.x, point.y, point.z)
            }) / neighbors.len() as f32;
            let current = previous[vertex_index];
            let blended =
                Vector3::new(current.x, current.y, current.z) * (1.0 - lambda) + mean * lambda;
            vertices[vertex_index] = Point3::new(blended.x, blended.y, blended.z);
        }
    }
    Mesh::new(vertices, mesh.triangles.clone())
}

fn fract(value: f32) -> f32 {
    value - value.floor()
}

fn invalid_argument(message: impl Into<String>) -> DetectError {
    DetectError::InvalidArgument(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use three_d_processing_core::Quaternion;

    fn single_triangle() -> Mesh {
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

    fn tetrahedron() -> Mesh {
        Mesh::new(
            [
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
                Point3::new(0.0, 0.0, 1.0),
            ],
            [
                Triangle::new(0, 2, 1),
                Triangle::new(0, 1, 3),
                Triangle::new(1, 2, 3),
                Triangle::new(2, 0, 3),
            ],
        )
        .unwrap()
    }

    #[test]
    fn computes_triangle_area_and_normal() {
        let mesh = single_triangle();
        assert_eq!(mesh.surface_area().unwrap(), 0.5);
        assert_eq!(
            triangle_normal(&mesh, Triangle::new(0, 1, 2)).unwrap(),
            Vector3::new(0.0, 0.0, 1.0)
        );
    }

    #[test]
    fn tetrahedron_is_manifold_watertight_and_has_volume() {
        let mesh = tetrahedron();
        assert!(mesh.is_manifold().unwrap());
        assert!(mesh.is_watertight().unwrap());
        assert!((mesh.volume().unwrap() - (1.0 / 6.0)).abs() < 0.001);
    }

    #[test]
    fn connected_components_split_disconnected_meshes() {
        let first = single_triangle();
        let second = Mesh::new(
            [
                Point3::new(2.0, 0.0, 0.0),
                Point3::new(3.0, 0.0, 0.0),
                Point3::new(2.0, 1.0, 0.0),
            ],
            [Triangle::new(0, 1, 2)],
        )
        .unwrap();
        let merged = merge_meshes([&first, &second]).unwrap();
        let components = merged.connected_components().unwrap();
        assert_eq!(components.len(), 2);
    }

    #[test]
    fn laplacian_smoothing_preserves_vertex_count_and_sampling_is_deterministic() {
        let mesh = tetrahedron();
        let smoothed = mesh.laplacian_smooth(2, 0.25).unwrap();
        assert_eq!(smoothed.vertices.len(), mesh.vertices.len());
        let first = mesh.sample_points_uniform(8).unwrap();
        let second = mesh.sample_points_uniform(8).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn rigid_transform_preserves_triangle_count() {
        let mesh = tetrahedron();
        let rotation = Quaternion::from_axis_angle(Vector3::new(0.0, 0.0, 1.0), 0.5).unwrap();
        let transformed = mesh
            .transformed_rigid(RigidTransform3::new(rotation, Vector3::new(1.0, 0.0, 0.0)).unwrap())
            .unwrap();
        assert_eq!(transformed.triangles.len(), mesh.triangles.len());
    }
}
