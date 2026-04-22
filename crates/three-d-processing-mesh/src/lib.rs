use three_d_processing_core::{Bounds3, Point3, Vector3};
use video_analysis_core::{DetectError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Triangle {
    pub vertices: [usize; 3],
}

impl Triangle {
    pub const fn new(a: usize, b: usize, c: usize) -> Self {
        Self {
            vertices: [a, b, c],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Mesh {
    pub vertices: Vec<Point3>,
    pub triangles: Vec<Triangle>,
}

impl Mesh {
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

    pub fn bounds(&self) -> Result<Option<Bounds3>> {
        Bounds3::from_points(&self.vertices)
    }

    pub fn surface_area(&self) -> Result<f32> {
        surface_area(self)
    }

    pub fn vertex_normals(&self) -> Result<Vec<Vector3>> {
        vertex_normals(self)
    }
}

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

pub fn surface_area(mesh: &Mesh) -> Result<f32> {
    let mut area = 0.0_f32;
    for triangle in &mesh.triangles {
        area += triangle_area(mesh, *triangle)?;
    }
    Ok(area)
}

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

fn invalid_argument(message: impl Into<String>) -> DetectError {
    DetectError::InvalidArgument(message.into())
}

#[cfg(test)]
mod tests {
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
    fn computes_triangle_area_and_normal() {
        let mesh = mesh();
        assert_eq!(mesh.surface_area().unwrap(), 0.5);
        assert_eq!(
            triangle_normal(&mesh, Triangle::new(0, 1, 2)).unwrap(),
            Vector3::new(0.0, 0.0, 1.0)
        );
    }
}
