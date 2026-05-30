use serde::{Deserialize, Serialize};
use video_analysis_core::Result;

use crate::{
    closest_point_on_bounds, distance_point_bounds, intersect_ray_bounds, invalid_argument,
    sphere_intersects_bounds, validate_points, Point3, Ray3, RayBoundsIntersection, Sphere3,
    Vector3,
};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
/// Data type for bounds3.
pub struct Bounds3 {
    /// The min value.
    pub min: Point3,
    /// The max value.
    pub max: Point3,
}

impl Bounds3 {
    /// Creates a new value.
    pub fn new(min: Point3, max: Point3) -> Result<Self> {
        let bounds = Self { min, max };
        bounds.validate()?;
        Ok(bounds)
    }

    /// Builds this value from points.
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

    /// Validates this value.
    pub fn validate(self) -> Result<()> {
        validate_points(&[self.min, self.max])?;
        if self.min.x > self.max.x || self.min.y > self.max.y || self.min.z > self.max.z {
            return Err(invalid_argument("bounds min must not exceed max"));
        }
        Ok(())
    }

    /// Returns size.
    pub fn size(self) -> Vector3 {
        self.max - self.min
    }

    /// Returns center.
    pub fn center(self) -> Point3 {
        self.min + (self.size() * 0.5)
    }

    /// Returns whether this value contains the point.
    pub fn contains_point(self, point: Point3) -> Result<bool> {
        self.validate()?;
        validate_points(&[point])?;
        Ok(point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
            && point.z >= self.min.z
            && point.z <= self.max.z)
    }

    /// Returns whether this value intersects another bounds.
    pub fn intersects(self, other: Self) -> Result<bool> {
        self.validate()?;
        other.validate()?;
        Ok(self.min.x < other.max.x
            && other.min.x < self.max.x
            && self.min.y < other.max.y
            && other.min.y < self.max.y
            && self.min.z < other.max.z
            && other.min.z < self.max.z)
    }

    /// Returns the closest point in this bounds to a point.
    pub fn closest_point(self, point: Point3) -> Result<Point3> {
        closest_point_on_bounds(self, point)
    }

    /// Returns the distance from this bounds to a point.
    pub fn distance_to_point(self, point: Point3) -> Result<f32> {
        distance_point_bounds(point, self)
    }

    /// Returns whether this bounds intersects a sphere.
    pub fn intersects_sphere(self, sphere: Sphere3) -> Result<bool> {
        sphere_intersects_bounds(sphere, self)
    }

    /// Returns the forward ray-bounds intersection interval, if any.
    pub fn intersect_ray(self, ray: Ray3) -> Result<Option<RayBoundsIntersection>> {
        intersect_ray_bounds(ray, self)
    }

    /// Returns intersection.
    pub fn intersection(self, other: Self) -> Result<Option<Self>> {
        if !self.intersects(other)? {
            return Ok(None);
        }
        Self::new(
            Point3::new(
                self.min.x.max(other.min.x),
                self.min.y.max(other.min.y),
                self.min.z.max(other.min.z),
            ),
            Point3::new(
                self.max.x.min(other.max.x),
                self.max.y.min(other.max.y),
                self.max.z.min(other.max.z),
            ),
        )
        .map(Some)
    }

    /// Returns union.
    pub fn union(self, other: Self) -> Result<Self> {
        self.validate()?;
        other.validate()?;
        Self::new(
            Point3::new(
                self.min.x.min(other.min.x),
                self.min.y.min(other.min.y),
                self.min.z.min(other.min.z),
            ),
            Point3::new(
                self.max.x.max(other.max.x),
                self.max.y.max(other.max.y),
                self.max.z.max(other.max.z),
            ),
        )
    }

    /// Returns volume.
    pub fn volume(self) -> Result<f32> {
        self.validate()?;
        let size = self.size();
        Ok(size.x * size.y * size.z)
    }
}
