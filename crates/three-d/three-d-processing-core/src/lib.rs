#![doc = include_str!("../README.md")]

use std::ops::{Add, AddAssign, Div, Mul, Sub};

use video_analysis_core::{DetectError, Result};

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vector3 {
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0);

    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }

    pub fn dot(self, rhs: Self) -> f32 {
        self.x.mul_add(rhs.x, self.y.mul_add(rhs.y, self.z * rhs.z))
    }

    pub fn cross(self, rhs: Self) -> Self {
        Self::new(
            self.y.mul_add(rhs.z, -(self.z * rhs.y)),
            self.z.mul_add(rhs.x, -(self.x * rhs.z)),
            self.x.mul_add(rhs.y, -(self.y * rhs.x)),
        )
    }

    pub fn length(self) -> f32 {
        self.dot(self).sqrt()
    }

    pub fn normalize(self) -> Result<Self> {
        validate_finite_vector(self, "vector")?;
        let length = self.length();
        if length <= f32::EPSILON {
            return Err(invalid_argument("vector length must be greater than zero"));
        }
        Ok(self / length)
    }
}

impl Add for Vector3 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl AddAssign for Vector3 {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
        self.z += rhs.z;
    }
}

impl Sub for Vector3 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl Mul<f32> for Vector3 {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

impl Div<f32> for Vector3 {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs, self.z / rhs)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Point3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Point3 {
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }
}

impl Add<Vector3> for Point3 {
    type Output = Self;

    fn add(self, rhs: Vector3) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl Sub<Point3> for Point3 {
    type Output = Vector3;

    fn sub(self, rhs: Point3) -> Self::Output {
        Vector3::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds3 {
    pub min: Point3,
    pub max: Point3,
}

impl Bounds3 {
    pub fn from_points(points: &[Point3]) -> Result<Option<Self>> {
        validate_points(points)?;
        let Some(first) = points.first().copied() else {
            return Ok(None);
        };
        let mut min = first;
        let mut max = first;
        for point in points.iter().copied().skip(1) {
            min.x = min.x.min(point.x);
            min.y = min.y.min(point.y);
            min.z = min.z.min(point.z);
            max.x = max.x.max(point.x);
            max.y = max.y.max(point.y);
            max.z = max.z.max(point.z);
        }
        Ok(Some(Self { min, max }))
    }

    pub fn size(self) -> Vector3 {
        self.max - self.min
    }

    pub fn center(self) -> Point3 {
        self.min + (self.size() * 0.5)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform3 {
    pub translation: Vector3,
    pub scale: f32,
}

impl Transform3 {
    pub const IDENTITY: Self = Self {
        translation: Vector3::ZERO,
        scale: 1.0,
    };

    pub fn new(translation: Vector3, scale: f32) -> Result<Self> {
        if !translation.is_finite() || !scale.is_finite() || scale == 0.0 {
            return Err(invalid_argument(
                "transform translation must be finite and scale must be finite and non-zero",
            ));
        }
        Ok(Self { translation, scale })
    }

    pub fn apply_point(self, point: Point3) -> Point3 {
        Point3::new(
            point.x * self.scale + self.translation.x,
            point.y * self.scale + self.translation.y,
            point.z * self.scale + self.translation.z,
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PointCloud {
    points: Vec<Point3>,
}

impl PointCloud {
    pub fn new(points: impl Into<Vec<Point3>>) -> Result<Self> {
        let points = points.into();
        validate_points(&points)?;
        Ok(Self { points })
    }

    pub fn points(&self) -> &[Point3] {
        &self.points
    }

    pub fn bounds(&self) -> Result<Option<Bounds3>> {
        Bounds3::from_points(&self.points)
    }

    pub fn centroid(&self) -> Result<Option<Point3>> {
        centroid(&self.points)
    }

    pub fn transformed(&self, transform: Transform3) -> Result<Self> {
        PointCloud::new(
            self.points
                .iter()
                .copied()
                .map(|point| transform.apply_point(point))
                .collect::<Vec<_>>(),
        )
    }
}

pub fn centroid(points: &[Point3]) -> Result<Option<Point3>> {
    validate_points(points)?;
    if points.is_empty() {
        return Ok(None);
    }
    let mut sum = Vector3::ZERO;
    for point in points {
        sum += Vector3::new(point.x, point.y, point.z);
    }
    let count = points.len() as f32;
    Ok(Some(Point3::new(
        sum.x / count,
        sum.y / count,
        sum.z / count,
    )))
}

fn validate_points(points: &[Point3]) -> Result<()> {
    if points.iter().any(|point| !point.is_finite()) {
        return Err(invalid_argument("points must be finite"));
    }
    Ok(())
}

fn validate_finite_vector(vector: Vector3, name: &str) -> Result<()> {
    if vector.is_finite() {
        Ok(())
    } else {
        Err(invalid_argument(format!(
            "{name} components must be finite"
        )))
    }
}

fn invalid_argument(message: impl Into<String>) -> DetectError {
    DetectError::InvalidArgument(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_point_cloud_bounds_and_centroid() {
        let cloud =
            PointCloud::new([Point3::new(0.0, 0.0, 0.0), Point3::new(2.0, 4.0, 6.0)]).unwrap();
        assert_eq!(cloud.centroid().unwrap(), Some(Point3::new(1.0, 2.0, 3.0)));
        assert_eq!(
            cloud.bounds().unwrap().unwrap().size(),
            Vector3::new(2.0, 4.0, 6.0)
        );
    }
}
