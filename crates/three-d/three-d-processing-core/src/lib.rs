#![doc = include_str!("../README.md")]

use std::collections::BTreeMap;
use std::ops::{Add, AddAssign, Div, Mul, Sub};

use serde::{Deserialize, Serialize};
use video_analysis_core::{DetectError, Result};

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
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

    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }

    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    pub fn distance(self, rhs: Self) -> f32 {
        (self - rhs).length()
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

impl Mul<Vector3> for f32 {
    type Output = Vector3;

    fn mul(self, rhs: Vector3) -> Self::Output {
        rhs * self
    }
}

impl Div<f32> for Vector3 {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs, self.z / rhs)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
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

    pub fn distance(self, rhs: Self) -> f32 {
        (self - rhs).length()
    }

    pub fn midpoint(self, rhs: Self) -> Self {
        self + ((rhs - self) * 0.5)
    }
}

impl Add<Vector3> for Point3 {
    type Output = Self;

    fn add(self, rhs: Vector3) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl Sub<Vector3> for Point3 {
    type Output = Self;

    fn sub(self, rhs: Vector3) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl Sub<Point3> for Point3 {
    type Output = Vector3;

    fn sub(self, rhs: Point3) -> Self::Output {
        Vector3::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Quaternion {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Quaternion {
    pub const IDENTITY: Self = Self::new(0.0, 0.0, 0.0, 1.0);

    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite() && self.w.is_finite()
    }

    pub fn from_axis_angle(axis: Vector3, angle_radians: f32) -> Result<Self> {
        validate_finite_vector(axis, "axis")?;
        if !angle_radians.is_finite() {
            return Err(invalid_argument("angle must be finite"));
        }
        let axis = axis.normalize()?;
        let half = angle_radians * 0.5;
        let sin = half.sin();
        Ok(Self::new(
            axis.x * sin,
            axis.y * sin,
            axis.z * sin,
            half.cos(),
        ))
    }

    pub fn dot(self, rhs: Self) -> f32 {
        self.x.mul_add(
            rhs.x,
            self.y.mul_add(rhs.y, self.z.mul_add(rhs.z, self.w * rhs.w)),
        )
    }

    pub fn norm(self) -> f32 {
        self.dot(self).sqrt()
    }

    pub fn normalize(self) -> Result<Self> {
        if !self.is_finite() {
            return Err(invalid_argument("quaternion components must be finite"));
        }
        let norm = self.norm();
        if norm <= f32::EPSILON {
            return Err(invalid_argument(
                "quaternion norm must be greater than zero",
            ));
        }
        Ok(Self::new(
            self.x / norm,
            self.y / norm,
            self.z / norm,
            self.w / norm,
        ))
    }

    pub fn conjugate(self) -> Self {
        Self::new(-self.x, -self.y, -self.z, self.w)
    }

    pub fn rotate_vector(self, vector: Vector3) -> Result<Vector3> {
        let q = self.normalize()?;
        validate_finite_vector(vector, "vector")?;
        let u = Vector3::new(q.x, q.y, q.z);
        let uv = u.cross(vector);
        let uuv = u.cross(uv);
        Ok(vector + ((2.0 * q.w) * uv) + (2.0 * uuv))
    }

    pub fn mul_quaternion(self, rhs: Self) -> Result<Self> {
        let lhs = self.normalize()?;
        let rhs = rhs.normalize()?;
        Ok(Self::new(
            lhs.w
                .mul_add(rhs.x, lhs.x.mul_add(rhs.w, lhs.y * rhs.z - lhs.z * rhs.y)),
            lhs.w
                .mul_add(rhs.y, -lhs.x * rhs.z + lhs.y.mul_add(rhs.w, lhs.z * rhs.x)),
            lhs.w
                .mul_add(rhs.z, lhs.x * rhs.y - lhs.y * rhs.x + lhs.z * rhs.w),
            lhs.w
                .mul_add(rhs.w, -(lhs.x * rhs.x + lhs.y * rhs.y + lhs.z * rhs.z)),
        ))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RigidTransform3 {
    pub rotation: Quaternion,
    pub translation: Vector3,
}

impl RigidTransform3 {
    pub const IDENTITY: Self = Self {
        rotation: Quaternion::IDENTITY,
        translation: Vector3::ZERO,
    };

    pub fn new(rotation: Quaternion, translation: Vector3) -> Result<Self> {
        let rotation = rotation.normalize()?;
        validate_finite_vector(translation, "translation")?;
        Ok(Self {
            rotation,
            translation,
        })
    }

    pub fn apply_point(self, point: Point3) -> Result<Point3> {
        let rotated = self
            .rotation
            .rotate_vector(point - Point3::new(0.0, 0.0, 0.0))?;
        Ok(Point3::new(
            rotated.x + self.translation.x,
            rotated.y + self.translation.y,
            rotated.z + self.translation.z,
        ))
    }

    pub fn apply_vector(self, vector: Vector3) -> Result<Vector3> {
        self.rotation.rotate_vector(vector)
    }

    pub fn inverse(self) -> Result<Self> {
        let rotation = self.rotation.normalize()?.conjugate();
        let translation = rotation.rotate_vector(self.translation * -1.0)?;
        Self::new(rotation, translation)
    }

    pub fn compose(self, rhs: Self) -> Result<Self> {
        let rotation = self.rotation.mul_quaternion(rhs.rotation)?;
        let translation = self.apply_vector(rhs.translation)? + self.translation;
        Self::new(rotation, translation)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LineSegment3 {
    pub start: Point3,
    pub end: Point3,
}

impl LineSegment3 {
    pub fn new(start: Point3, end: Point3) -> Result<Self> {
        validate_points(&[start, end])?;
        if start == end {
            return Err(invalid_argument(
                "line segment start and end must not be identical",
            ));
        }
        Ok(Self { start, end })
    }

    pub fn length(self) -> f32 {
        self.start.distance(self.end)
    }

    pub fn midpoint(self) -> Point3 {
        self.start.midpoint(self.end)
    }

    pub fn direction(self) -> Result<Vector3> {
        (self.end - self.start).normalize()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

    pub fn transformed_rigid(&self, transform: RigidTransform3) -> Result<Self> {
        PointCloud::new(transform_rigid(&self.points, transform)?)
    }

    pub fn voxel_downsample(&self, voxel_size: f32) -> Result<Self> {
        PointCloud::new(voxel_downsample(&self.points, voxel_size)?)
    }

    pub fn center_and_scale(&self, target_extent: f32) -> Result<Option<Self>> {
        center_and_scale(&self.points, target_extent)
            .map(|value| value.map(|points| Self { points }))
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

pub fn point_distance(a: Point3, b: Point3) -> Result<f32> {
    validate_points(&[a, b])?;
    Ok(a.distance(b))
}

pub fn transform_rigid(points: &[Point3], transform: RigidTransform3) -> Result<Vec<Point3>> {
    validate_points(points)?;
    points
        .iter()
        .copied()
        .map(|point| transform.apply_point(point))
        .collect()
}

pub fn voxel_downsample(points: &[Point3], voxel_size: f32) -> Result<Vec<Point3>> {
    validate_points(points)?;
    if !voxel_size.is_finite() || voxel_size <= 0.0 {
        return Err(invalid_argument(
            "voxel size must be finite and greater than zero",
        ));
    }
    let mut buckets: BTreeMap<(i32, i32, i32), (Vector3, usize)> = BTreeMap::new();
    for point in points {
        let key = (
            (point.x / voxel_size).floor() as i32,
            (point.y / voxel_size).floor() as i32,
            (point.z / voxel_size).floor() as i32,
        );
        let entry = buckets.entry(key).or_insert((Vector3::ZERO, 0));
        entry.0 += Vector3::new(point.x, point.y, point.z);
        entry.1 += 1;
    }
    Ok(buckets
        .into_values()
        .map(|(sum, count)| {
            let denom = count as f32;
            Point3::new(sum.x / denom, sum.y / denom, sum.z / denom)
        })
        .collect())
}

pub fn center_and_scale(points: &[Point3], target_extent: f32) -> Result<Option<Vec<Point3>>> {
    validate_points(points)?;
    if points.is_empty() {
        return Ok(None);
    }
    if !target_extent.is_finite() || target_extent <= 0.0 {
        return Err(invalid_argument(
            "target extent must be finite and greater than zero",
        ));
    }
    let bounds = Bounds3::from_points(points)?.expect("non-empty points");
    let center = bounds.center();
    let extent = bounds.size();
    let max_extent = extent.x.max(extent.y).max(extent.z);
    let scale = if max_extent <= f32::EPSILON {
        1.0
    } else {
        target_extent / max_extent
    };
    Ok(Some(
        points
            .iter()
            .map(|point| {
                let relative = *point - center;
                Point3::new(relative.x * scale, relative.y * scale, relative.z * scale)
            })
            .collect(),
    ))
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
    use std::f32::consts::FRAC_PI_2;

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

    #[test]
    fn quaternion_normalization_and_rigid_inverse_are_stable() {
        let rotation = Quaternion::from_axis_angle(Vector3::new(0.0, 0.0, 2.0), FRAC_PI_2)
            .unwrap()
            .normalize()
            .unwrap();
        let transform = RigidTransform3::new(rotation, Vector3::new(1.0, 2.0, 3.0)).unwrap();
        let point = Point3::new(1.0, 0.0, 0.0);
        let transformed = transform.apply_point(point).unwrap();
        let recovered = transform
            .inverse()
            .unwrap()
            .apply_point(transformed)
            .unwrap();
        assert!((recovered.x - point.x).abs() < 0.001);
        assert!((recovered.y - point.y).abs() < 0.001);
        assert!((recovered.z - point.z).abs() < 0.001);
    }

    #[test]
    fn voxel_downsampling_is_deterministic() {
        let points = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(0.1, 0.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
        ];
        let first = voxel_downsample(&points, 0.5).unwrap();
        let second = voxel_downsample(&points, 0.5).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.len(), 2);
    }

    #[test]
    fn center_and_scale_normalizes_extent() {
        let points = vec![Point3::new(1.0, 1.0, 1.0), Point3::new(3.0, 5.0, 1.0)];
        let normalized = center_and_scale(&points, 2.0).unwrap().unwrap();
        let bounds = Bounds3::from_points(&normalized).unwrap().unwrap();
        let extent = bounds.size();
        assert!((extent.y - 2.0).abs() < 0.001);
        assert_eq!(bounds.center(), Point3::new(0.0, 0.0, 0.0));
    }
}
