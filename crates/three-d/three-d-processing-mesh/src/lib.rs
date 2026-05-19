#![doc = include_str!("../README.md")]

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};
use three_d_processing_core::{
    closest_point_on_segment, Bounds3, LineSegment3, Point3, Ray3, RigidTransform3, Transform3,
    Vector3,
};
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
/// Data type for mesh diagnostics.
pub struct MeshDiagnostics {
    /// Triangle indices whose area is effectively zero.
    pub degenerate_triangles: Vec<usize>,
    /// Pairs of vertex indices with identical positions.
    pub duplicate_vertices: Vec<[usize; 2]>,
    /// Edges referenced by more than two triangles.
    pub non_manifold_edges: Vec<Edge>,
    /// Edges referenced by exactly one triangle.
    pub boundary_edges: Vec<Edge>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
/// Data type for a ray-mesh intersection.
pub struct MeshRayIntersection {
    /// Triangle index hit by the ray.
    pub triangle_index: usize,
    /// Distance along the ray.
    pub distance: f32,
    /// Intersection point.
    pub point: Point3,
    /// Barycentric coordinates at the hit point.
    pub barycentric: [f32; 3],
}

impl MeshDiagnostics {
    /// Returns whether any issue was found.
    pub fn has_issues(&self) -> bool {
        !self.degenerate_triangles.is_empty()
            || !self.duplicate_vertices.is_empty()
            || !self.non_manifold_edges.is_empty()
            || !self.boundary_edges.is_empty()
    }
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

    /// Returns area-weighted surface centroid.
    pub fn surface_centroid(&self) -> Result<Option<Point3>> {
        surface_centroid(self)
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

    /// Returns closest point on this mesh to a point.
    pub fn closest_point(&self, point: Point3) -> Result<Option<Point3>> {
        closest_point(self, point)
    }

    /// Returns forward ray intersections in distance order.
    pub fn ray_intersections(&self, ray: Ray3) -> Result<Vec<MeshRayIntersection>> {
        ray_intersections(self, ray)
    }

    /// Returns nearest forward ray intersection.
    pub fn ray_intersection(&self, ray: Ray3) -> Result<Option<MeshRayIntersection>> {
        Ok(self.ray_intersections(ray)?.into_iter().next())
    }

    /// Returns laplacian smooth.
    pub fn laplacian_smooth(&self, iterations: usize, lambda: f32) -> Result<Self> {
        laplacian_smooth(self, iterations, lambda)
    }

    /// Returns diagnostics.
    pub fn diagnostics(&self) -> Result<MeshDiagnostics> {
        mesh_diagnostics(self)
    }

    /// Returns without degenerate triangles.
    pub fn remove_degenerate_triangles(&self) -> Result<Self> {
        remove_degenerate_triangles(self)
    }

    /// Returns with duplicate vertices welded by distance.
    pub fn weld_vertices(&self, epsilon: f32) -> Result<Self> {
        weld_vertices(self, epsilon)
    }

    /// Returns with triangle winding reversed.
    pub fn flip_winding(&self) -> Result<Self> {
        flip_winding(self)
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

/// Returns triangle centroid.
pub fn triangle_centroid(mesh: &Mesh, triangle: Triangle) -> Result<Point3> {
    mesh.validate()?;
    validate_triangle_indices(mesh, triangle)?;
    let a = mesh.vertices[triangle.vertices[0]];
    let b = mesh.vertices[triangle.vertices[1]];
    let c = mesh.vertices[triangle.vertices[2]];
    Ok(Point3::new(
        (a.x + b.x + c.x) / 3.0,
        (a.y + b.y + c.y) / 3.0,
        (a.z + b.z + c.z) / 3.0,
    ))
}

/// Returns barycentric coordinates for a point against a triangle.
pub fn triangle_barycentric_coordinates(
    mesh: &Mesh,
    triangle: Triangle,
    point: Point3,
) -> Result<[f32; 3]> {
    mesh.validate()?;
    validate_triangle_indices(mesh, triangle)?;
    if !point.is_finite() {
        return Err(invalid_argument("point must be finite"));
    }
    let a = mesh.vertices[triangle.vertices[0]];
    let b = mesh.vertices[triangle.vertices[1]];
    let c = mesh.vertices[triangle.vertices[2]];
    let v0 = b - a;
    let v1 = c - a;
    let v2 = point - a;
    let d00 = v0.dot(v0);
    let d01 = v0.dot(v1);
    let d11 = v1.dot(v1);
    let d20 = v2.dot(v0);
    let d21 = v2.dot(v1);
    let denominator = d00.mul_add(d11, -(d01 * d01));
    if denominator.abs() <= f32::EPSILON {
        return Err(invalid_argument("triangle area must be greater than zero"));
    }
    let v = d11.mul_add(d20, -(d01 * d21)) / denominator;
    let w = d00.mul_add(d21, -(d01 * d20)) / denominator;
    Ok([1.0 - v - w, v, w])
}

/// Returns surface area.
pub fn surface_area(mesh: &Mesh) -> Result<f32> {
    mesh.triangles.iter().try_fold(0.0_f32, |area, triangle| {
        Ok(area + triangle_area(mesh, *triangle)?)
    })
}

/// Returns area-weighted surface centroid.
pub fn surface_centroid(mesh: &Mesh) -> Result<Option<Point3>> {
    mesh.validate()?;
    let mut weighted = Vector3::ZERO;
    let mut total_area = 0.0_f32;
    for triangle in &mesh.triangles {
        let area = triangle_area(mesh, *triangle)?;
        if area <= f32::EPSILON {
            continue;
        }
        let centroid = triangle_centroid(mesh, *triangle)?;
        weighted += Vector3::new(centroid.x, centroid.y, centroid.z) * area;
        total_area += area;
    }
    if total_area <= f32::EPSILON {
        return Ok(None);
    }
    Ok(Some(Point3::new(
        weighted.x / total_area,
        weighted.y / total_area,
        weighted.z / total_area,
    )))
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

/// Returns mesh diagnostics.
pub fn mesh_diagnostics(mesh: &Mesh) -> Result<MeshDiagnostics> {
    mesh.validate()?;
    let mut edge_counts: BTreeMap<Edge, usize> = BTreeMap::new();
    let mut degenerate_triangles = Vec::new();
    for (triangle_index, triangle) in mesh.triangles.iter().copied().enumerate() {
        if triangle_area(mesh, triangle)? <= f32::EPSILON {
            degenerate_triangles.push(triangle_index);
        }
        for edge in triangle.edges()? {
            *edge_counts.entry(edge).or_default() += 1;
        }
    }

    let mut duplicate_vertices = Vec::new();
    for left in 0..mesh.vertices.len() {
        for right in (left + 1)..mesh.vertices.len() {
            if mesh.vertices[left] == mesh.vertices[right] {
                duplicate_vertices.push([left, right]);
            }
        }
    }

    let non_manifold_edges = edge_counts
        .iter()
        .filter_map(|(edge, count)| (*count > 2).then_some(*edge))
        .collect();
    let boundary_edges = edge_counts
        .iter()
        .filter_map(|(edge, count)| (*count == 1).then_some(*edge))
        .collect();

    Ok(MeshDiagnostics {
        degenerate_triangles,
        duplicate_vertices,
        non_manifold_edges,
        boundary_edges,
    })
}

/// Returns without degenerate triangles.
pub fn remove_degenerate_triangles(mesh: &Mesh) -> Result<Mesh> {
    mesh.validate()?;
    Mesh::new(
        mesh.vertices.clone(),
        mesh.triangles
            .iter()
            .copied()
            .filter(|triangle| triangle_area(mesh, *triangle).unwrap_or(0.0) > f32::EPSILON)
            .collect::<Vec<_>>(),
    )
}

/// Returns with duplicate vertices welded by distance.
pub fn weld_vertices(mesh: &Mesh, epsilon: f32) -> Result<Mesh> {
    mesh.validate()?;
    if !epsilon.is_finite() || epsilon < 0.0 {
        return Err(invalid_argument(
            "weld epsilon must be finite and non-negative",
        ));
    }
    let mut vertices = Vec::new();
    let mut remap = Vec::with_capacity(mesh.vertices.len());
    for vertex in &mesh.vertices {
        let existing = vertices
            .iter()
            .position(|candidate: &Point3| candidate.distance(*vertex) <= epsilon);
        let index = existing.unwrap_or_else(|| {
            let next = vertices.len();
            vertices.push(*vertex);
            next
        });
        remap.push(index);
    }
    let triangles = mesh
        .triangles
        .iter()
        .filter_map(|triangle| {
            let remapped = [
                remap[triangle.vertices[0]],
                remap[triangle.vertices[1]],
                remap[triangle.vertices[2]],
            ];
            (remapped[0] != remapped[1] && remapped[1] != remapped[2] && remapped[0] != remapped[2])
                .then_some(Triangle { vertices: remapped })
        })
        .collect::<Vec<_>>();
    Mesh::new(vertices, triangles)
}

/// Returns with triangle winding reversed.
pub fn flip_winding(mesh: &Mesh) -> Result<Mesh> {
    mesh.validate()?;
    Mesh::new(
        mesh.vertices.clone(),
        mesh.triangles
            .iter()
            .map(|triangle| Triangle {
                vertices: [
                    triangle.vertices[0],
                    triangle.vertices[2],
                    triangle.vertices[1],
                ],
            })
            .collect::<Vec<_>>(),
    )
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

/// Returns closest point on triangle to a point.
pub fn triangle_closest_point(mesh: &Mesh, triangle: Triangle, point: Point3) -> Result<Point3> {
    mesh.validate()?;
    validate_triangle_indices(mesh, triangle)?;
    if !point.is_finite() {
        return Err(invalid_argument("point must be finite"));
    }
    let a = mesh.vertices[triangle.vertices[0]];
    let b = mesh.vertices[triangle.vertices[1]];
    let c = mesh.vertices[triangle.vertices[2]];
    let normal = (b - a).cross(c - a);
    if normal.length() > f32::EPSILON {
        let unit_normal = normal.normalize()?;
        let distance = (point - a).dot(unit_normal);
        let projected = point - unit_normal * distance;
        let barycentric = triangle_barycentric_coordinates(mesh, triangle, projected)?;
        if barycentric
            .iter()
            .all(|coordinate| *coordinate >= -f32::EPSILON)
        {
            return Ok(projected);
        }
    }

    let mut candidates = vec![a, b, c];
    for (start, end) in [(a, b), (b, c), (c, a)] {
        if start != end {
            candidates.push(closest_point_on_segment(
                LineSegment3::new(start, end)?,
                point,
            )?);
        }
    }
    candidates
        .into_iter()
        .min_by(|left, right| {
            left.distance(point)
                .partial_cmp(&right.distance(point))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .ok_or_else(|| invalid_argument("triangle must have at least one vertex"))
}

/// Returns closest point on mesh triangles to a point.
pub fn closest_point(mesh: &Mesh, point: Point3) -> Result<Option<Point3>> {
    mesh.validate()?;
    if !point.is_finite() {
        return Err(invalid_argument("point must be finite"));
    }
    if mesh.triangles.is_empty() {
        return Ok(mesh.vertices.iter().copied().min_by(|left, right| {
            left.distance(point)
                .partial_cmp(&right.distance(point))
                .unwrap_or(std::cmp::Ordering::Equal)
        }));
    }
    mesh.triangles
        .iter()
        .copied()
        .map(|triangle| triangle_closest_point(mesh, triangle, point))
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .min_by(|left, right| {
            left.distance(point)
                .partial_cmp(&right.distance(point))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(Some)
        .ok_or_else(|| invalid_argument("mesh must contain triangles or vertices"))
}

/// Returns forward ray intersections in distance order.
pub fn ray_intersections(mesh: &Mesh, ray: Ray3) -> Result<Vec<MeshRayIntersection>> {
    mesh.validate()?;
    if !ray.origin.is_finite() || !ray.direction.is_finite() {
        return Err(invalid_argument("ray components must be finite"));
    }
    let direction = ray.direction.normalize()?;
    let mut intersections = Vec::new();
    for (triangle_index, triangle) in mesh.triangles.iter().copied().enumerate() {
        if let Some(hit) =
            ray_triangle_intersection(mesh, triangle_index, triangle, ray.origin, direction)?
        {
            intersections.push(hit);
        }
    }
    intersections.sort_by(|left, right| {
        left.distance
            .partial_cmp(&right.distance)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(intersections)
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

fn validate_triangle_indices(mesh: &Mesh, triangle: Triangle) -> Result<()> {
    for index in triangle.vertices {
        if index >= mesh.vertices.len() {
            return Err(invalid_argument("triangle vertex index is out of bounds"));
        }
    }
    Ok(())
}

fn ray_triangle_intersection(
    mesh: &Mesh,
    triangle_index: usize,
    triangle: Triangle,
    origin: Point3,
    direction: Vector3,
) -> Result<Option<MeshRayIntersection>> {
    let a = mesh.vertices[triangle.vertices[0]];
    let b = mesh.vertices[triangle.vertices[1]];
    let c = mesh.vertices[triangle.vertices[2]];
    let edge1 = b - a;
    let edge2 = c - a;
    let p = direction.cross(edge2);
    let determinant = edge1.dot(p);
    if determinant.abs() <= f32::EPSILON {
        return Ok(None);
    }
    let inverse_determinant = 1.0 / determinant;
    let t = origin - a;
    let u = t.dot(p) * inverse_determinant;
    if !(-f32::EPSILON..=1.0 + f32::EPSILON).contains(&u) {
        return Ok(None);
    }
    let q = t.cross(edge1);
    let v = direction.dot(q) * inverse_determinant;
    if v < -f32::EPSILON || u + v > 1.0 + f32::EPSILON {
        return Ok(None);
    }
    let distance = edge2.dot(q) * inverse_determinant;
    if distance < 0.0 {
        return Ok(None);
    }
    let point = origin + direction * distance;
    Ok(Some(MeshRayIntersection {
        triangle_index,
        distance,
        point,
        barycentric: [1.0 - u - v, u, v],
    }))
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

    #[test]
    fn diagnostics_report_degenerate_duplicates_and_boundaries() {
        let mesh = Mesh::new(
            [
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(2.0, 0.0, 0.0),
                Point3::new(0.0, 0.0, 0.0),
            ],
            [Triangle::new(0, 1, 2)],
        )
        .unwrap();
        let diagnostics = mesh.diagnostics().unwrap();
        assert_eq!(diagnostics.degenerate_triangles, vec![0]);
        assert_eq!(diagnostics.duplicate_vertices, vec![[0, 3]]);
        assert_eq!(diagnostics.boundary_edges.len(), 3);
        assert!(diagnostics.has_issues());
    }

    #[test]
    fn repair_helpers_remove_degenerates_weld_vertices_and_flip_winding() {
        let mesh = Mesh::new(
            [
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(2.0, 0.0, 0.0),
            ],
            [Triangle::new(0, 1, 2), Triangle::new(0, 3, 4)],
        )
        .unwrap();
        assert_eq!(
            mesh.remove_degenerate_triangles().unwrap().triangles.len(),
            1
        );
        let welded = mesh.weld_vertices(0.0).unwrap();
        assert_eq!(welded.vertices.len(), 4);
        assert_eq!(welded.triangles.len(), 1);

        let flipped = single_triangle().flip_winding().unwrap();
        assert_eq!(flipped.triangles[0], Triangle::new(0, 2, 1));
    }

    #[test]
    fn computes_triangle_centroid_barycentrics_and_surface_centroid() {
        let mesh = single_triangle();
        let triangle = Triangle::new(0, 1, 2);
        assert_eq!(
            triangle_centroid(&mesh, triangle).unwrap(),
            Point3::new(1.0 / 3.0, 1.0 / 3.0, 0.0)
        );
        let barycentric =
            triangle_barycentric_coordinates(&mesh, triangle, Point3::new(0.25, 0.25, 0.0))
                .unwrap();
        assert!((barycentric[0] - 0.5).abs() < 0.001);
        assert!((barycentric[1] - 0.25).abs() < 0.001);
        assert!((barycentric[2] - 0.25).abs() < 0.001);
        assert_eq!(
            mesh.surface_centroid().unwrap(),
            Some(Point3::new(1.0 / 3.0, 1.0 / 3.0, 0.0))
        );
    }

    #[test]
    fn closest_point_projects_to_triangle_or_edges() {
        let mesh = single_triangle();
        assert_eq!(
            triangle_closest_point(&mesh, Triangle::new(0, 1, 2), Point3::new(0.25, 0.25, 2.0))
                .unwrap(),
            Point3::new(0.25, 0.25, 0.0)
        );
        assert_eq!(
            mesh.closest_point(Point3::new(2.0, 0.25, 0.0)).unwrap(),
            Some(Point3::new(1.0, 0.0, 0.0))
        );
    }

    #[test]
    fn ray_intersections_are_sorted_and_include_barycentrics() {
        let mesh = single_triangle();
        let ray = Ray3::new(Point3::new(0.25, 0.25, 1.0), Vector3::new(0.0, 0.0, -1.0)).unwrap();
        let hits = mesh.ray_intersections(ray).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].triangle_index, 0);
        assert!((hits[0].distance - 1.0).abs() < 0.001);
        assert_eq!(hits[0].point, Point3::new(0.25, 0.25, 0.0));
        assert!((hits[0].barycentric[0] - 0.5).abs() < 0.001);
    }
}
