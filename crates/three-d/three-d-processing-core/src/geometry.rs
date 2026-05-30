use serde::{Deserialize, Serialize};
use video_analysis_core::Result;

use crate::{invalid_argument, validate_finite_vector, validate_points, Bounds3, Point3, Vector3};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
/// Data type for line segment3.
pub struct LineSegment3 {
    /// The start value.
    pub start: Point3,
    /// The end value.
    pub end: Point3,
}

impl LineSegment3 {
    /// Creates a new value.
    pub fn new(start: Point3, end: Point3) -> Result<Self> {
        validate_points(&[start, end])?;
        if start == end {
            return Err(invalid_argument(
                "line segment start and end must not be identical",
            ));
        }
        Ok(Self { start, end })
    }

    /// Returns length.
    pub fn length(self) -> f32 {
        self.start.distance(self.end)
    }

    /// Returns midpoint.
    pub fn midpoint(self) -> Point3 {
        self.start.midpoint(self.end)
    }

    /// Returns direction.
    pub fn direction(self) -> Result<Vector3> {
        (self.end - self.start).normalize()
    }

    /// Returns closest point on this segment to a point.
    pub fn closest_point(self, point: Point3) -> Result<Point3> {
        closest_point_on_segment(self, point)
    }

    /// Returns distance from this segment to a point.
    pub fn distance_to_point(self, point: Point3) -> Result<f32> {
        Ok(self.closest_point(point)?.distance(point))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
/// Data type for ray3.
pub struct Ray3 {
    /// The origin value.
    pub origin: Point3,
    /// Unit direction.
    pub direction: Vector3,
}

impl Ray3 {
    /// Creates a new value.
    pub fn new(origin: Point3, direction: Vector3) -> Result<Self> {
        validate_points(&[origin])?;
        Ok(Self {
            origin,
            direction: direction.normalize()?,
        })
    }

    /// Returns point at distance.
    pub fn at(self, distance: f32) -> Result<Point3> {
        if !distance.is_finite() {
            return Err(invalid_argument("ray distance must be finite"));
        }
        Ok(self.origin + self.direction * distance)
    }

    /// Returns closest point on this ray to a point.
    pub fn closest_point(self, point: Point3) -> Result<Point3> {
        closest_point_on_ray(self, point)
    }

    /// Returns distance from this ray to a point.
    pub fn distance_to_point(self, point: Point3) -> Result<f32> {
        Ok(self.closest_point(point)?.distance(point))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
/// Data type for plane3.
pub struct Plane3 {
    /// Unit normal.
    pub normal: Vector3,
    /// Signed distance from origin.
    pub d: f32,
}

impl Plane3 {
    /// Creates a new value from a point and normal.
    pub fn from_point_normal(point: Point3, normal: Vector3) -> Result<Self> {
        validate_points(&[point])?;
        let normal = normal.normalize()?;
        let d = -normal.dot(Vector3::new(point.x, point.y, point.z));
        Ok(Self { normal, d })
    }

    /// Returns signed distance to point.
    pub fn signed_distance(self, point: Point3) -> Result<f32> {
        validate_points(&[point])?;
        Ok(self.normal.dot(Vector3::new(point.x, point.y, point.z)) + self.d)
    }

    /// Returns the orthogonal projection of a point onto this plane.
    pub fn project_point(self, point: Point3) -> Result<Point3> {
        project_point_to_plane(point, self)
    }

    /// Returns the forward ray-plane intersection point, if any.
    pub fn intersect_ray(self, ray: Ray3) -> Result<Option<Point3>> {
        intersect_ray_plane(ray, self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
/// Data type for sphere3.
pub struct Sphere3 {
    /// The center value.
    pub center: Point3,
    /// The radius value.
    pub radius: f32,
}

impl Sphere3 {
    /// Creates a new value.
    pub fn new(center: Point3, radius: f32) -> Result<Self> {
        validate_points(&[center])?;
        if !radius.is_finite() || radius <= 0.0 {
            return Err(invalid_argument(
                "sphere radius must be finite and greater than zero",
            ));
        }
        Ok(Self { center, radius })
    }

    /// Returns contains point.
    pub fn contains_point(self, point: Point3) -> Result<bool> {
        validate_points(&[point])?;
        Ok(self.center.distance(point) <= self.radius)
    }

    /// Returns whether this sphere intersects another sphere.
    pub fn intersects_sphere(self, other: Self) -> Result<bool> {
        sphere_intersects_sphere(self, other)
    }

    /// Returns the collision contact with another sphere, if any.
    pub fn collision_with_sphere(self, other: Self) -> Result<Option<SphereCollision>> {
        collision_sphere_sphere(self, other)
    }

    /// Returns whether this sphere intersects bounds.
    pub fn intersects_bounds(self, bounds: Bounds3) -> Result<bool> {
        sphere_intersects_bounds(self, bounds)
    }

    /// Returns surface area.
    pub fn surface_area(self) -> f32 {
        sphere_surface_area(self)
    }

    /// Returns volume.
    pub fn volume(self) -> f32 {
        sphere_volume(self)
    }

    /// Returns signed distance to the sphere surface.
    pub fn signed_distance(self, point: Point3) -> Result<f32> {
        validate_points(&[point])?;
        Ok(self.center.distance(point) - self.radius)
    }

    /// Returns closest point on the sphere surface.
    pub fn closest_point(self, point: Point3) -> Result<Point3> {
        closest_point_on_sphere(self, point)
    }

    /// Returns forward ray-sphere intersection points in distance order.
    pub fn intersect_ray(self, ray: Ray3) -> Result<Vec<Point3>> {
        intersect_ray_sphere(ray, self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Data type for a ray/bounds intersection interval.
pub struct RayBoundsIntersection {
    /// Distance to the first forward point inside the bounds.
    pub entry_distance: f32,
    /// Distance to the last forward point inside the bounds.
    pub exit_distance: f32,
    /// First forward point inside the bounds.
    pub entry_point: Point3,
    /// Last forward point inside the bounds.
    pub exit_point: Point3,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Data type for a sphere/sphere collision contact.
pub struct SphereCollision {
    /// Unit normal pointing from the left sphere toward the right sphere.
    pub normal: Vector3,
    /// Contact point halfway through the overlapping region.
    pub point: Point3,
    /// Overlap depth along the normal.
    pub penetration_depth: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct AxisInterval {
    near: f32,
    far: f32,
}

/// Returns closest point on segment to a point.
pub fn closest_point_on_segment(segment: LineSegment3, point: Point3) -> Result<Point3> {
    validate_points(&[segment.start, segment.end, point])?;
    let segment_vector = segment.end - segment.start;
    let length_squared = segment_vector.length_squared();
    if length_squared <= f32::EPSILON {
        return Err(invalid_argument(
            "line segment start and end must not be identical",
        ));
    }
    let t = ((point - segment.start).dot(segment_vector) / length_squared).clamp(0.0, 1.0);
    Ok(segment.start + segment_vector * t)
}

/// Returns closest point on ray to a point.
pub fn closest_point_on_ray(ray: Ray3, point: Point3) -> Result<Point3> {
    validate_ray(ray)?;
    validate_points(&[point])?;
    let direction = ray.direction.normalize()?;
    let distance = (point - ray.origin).dot(direction).max(0.0);
    Ok(ray.origin + direction * distance)
}

/// Returns point projected to plane.
pub fn project_point_to_plane(point: Point3, plane: Plane3) -> Result<Point3> {
    validate_points(&[point])?;
    validate_finite_vector(plane.normal, "plane normal")?;
    let normal = plane.normal.normalize()?;
    if !plane.d.is_finite() {
        return Err(invalid_argument("plane distance must be finite"));
    }
    let distance = normal.dot(Vector3::new(point.x, point.y, point.z)) + plane.d;
    Ok(point - normal * distance)
}

/// Returns the closest point in bounds to a point.
pub fn closest_point_on_bounds(bounds: Bounds3, point: Point3) -> Result<Point3> {
    bounds.validate()?;
    validate_points(&[point])?;
    Ok(Point3::new(
        point.x.clamp(bounds.min.x, bounds.max.x),
        point.y.clamp(bounds.min.y, bounds.max.y),
        point.z.clamp(bounds.min.z, bounds.max.z),
    ))
}

/// Returns the distance from a point to bounds.
pub fn distance_point_bounds(point: Point3, bounds: Bounds3) -> Result<f32> {
    Ok(point.distance(closest_point_on_bounds(bounds, point)?))
}

/// Returns forward ray-plane intersection point, if any.
pub fn intersect_ray_plane(ray: Ray3, plane: Plane3) -> Result<Option<Point3>> {
    validate_ray(ray)?;
    validate_finite_vector(plane.normal, "plane normal")?;
    if !plane.d.is_finite() {
        return Err(invalid_argument("plane distance must be finite"));
    }
    let direction = ray.direction.normalize()?;
    let normal = plane.normal.normalize()?;
    let denominator = normal.dot(direction);
    if denominator.abs() <= f32::EPSILON {
        return Ok(None);
    }
    let numerator = -(normal.dot(Vector3::new(ray.origin.x, ray.origin.y, ray.origin.z)) + plane.d);
    let distance = numerator / denominator;
    if distance < 0.0 {
        return Ok(None);
    }
    Ok(Some(ray.origin + direction * distance))
}

/// Returns the forward ray-bounds intersection interval, if any.
pub fn intersect_ray_bounds(ray: Ray3, bounds: Bounds3) -> Result<Option<RayBoundsIntersection>> {
    validate_ray(ray)?;
    bounds.validate()?;
    let direction = ray.direction.normalize()?;
    let mut near = f32::NEG_INFINITY;
    let mut far = f32::INFINITY;

    for (origin, direction, min, max) in [
        (ray.origin.x, direction.x, bounds.min.x, bounds.max.x),
        (ray.origin.y, direction.y, bounds.min.y, bounds.max.y),
        (ray.origin.z, direction.z, bounds.min.z, bounds.max.z),
    ] {
        let Some(axis_interval) = ray_axis_interval(origin, direction, min, max) else {
            return Ok(None);
        };
        near = near.max(axis_interval.near);
        far = far.min(axis_interval.far);
        if near > far {
            return Ok(None);
        }
    }

    if far < 0.0 {
        return Ok(None);
    }
    let entry_distance = near.max(0.0);
    Ok(Some(RayBoundsIntersection {
        entry_distance,
        exit_distance: far,
        entry_point: ray.origin + direction * entry_distance,
        exit_point: ray.origin + direction * far,
    }))
}

/// Returns sphere surface area.
pub fn sphere_surface_area(sphere: Sphere3) -> f32 {
    4.0 * std::f32::consts::PI * sphere.radius * sphere.radius
}

/// Returns sphere volume.
pub fn sphere_volume(sphere: Sphere3) -> f32 {
    (4.0 / 3.0) * std::f32::consts::PI * sphere.radius * sphere.radius * sphere.radius
}

/// Returns closest point on sphere surface.
pub fn closest_point_on_sphere(sphere: Sphere3, point: Point3) -> Result<Point3> {
    validate_sphere(sphere)?;
    validate_points(&[point])?;
    let offset = point - sphere.center;
    let direction = if offset.length() <= f32::EPSILON {
        Vector3::new(1.0, 0.0, 0.0)
    } else {
        offset.normalize()?
    };
    Ok(sphere.center + direction * sphere.radius)
}

/// Returns whether two spheres intersect.
pub fn sphere_intersects_sphere(left: Sphere3, right: Sphere3) -> Result<bool> {
    validate_sphere(left)?;
    validate_sphere(right)?;
    Ok(spheres_overlap(left, right))
}

/// Returns collision contact data for two intersecting spheres.
pub fn collision_sphere_sphere(left: Sphere3, right: Sphere3) -> Result<Option<SphereCollision>> {
    validate_sphere(left)?;
    validate_sphere(right)?;
    let offset = right.center - left.center;
    let distance = offset.length();
    let radius_sum = left.radius + right.radius;
    if distance > radius_sum {
        return Ok(None);
    }
    let normal = if distance <= f32::EPSILON {
        Vector3::new(1.0, 0.0, 0.0)
    } else {
        offset / distance
    };
    let penetration_depth = radius_sum - distance;
    let left_surface = left.center + normal * left.radius;
    let right_surface = right.center - normal * right.radius;
    Ok(Some(SphereCollision {
        normal,
        point: left_surface.midpoint(right_surface),
        penetration_depth,
    }))
}

/// Returns whether a sphere intersects bounds.
pub fn sphere_intersects_bounds(sphere: Sphere3, bounds: Bounds3) -> Result<bool> {
    validate_sphere(sphere)?;
    bounds.validate()?;
    let distance_squared =
        (closest_point_on_bounds(bounds, sphere.center)? - sphere.center).length_squared();
    Ok(distance_squared <= sphere.radius * sphere.radius)
}

/// Returns forward ray-sphere intersection points in distance order.
pub fn intersect_ray_sphere(ray: Ray3, sphere: Sphere3) -> Result<Vec<Point3>> {
    validate_ray(ray)?;
    validate_sphere(sphere)?;
    let direction = ray.direction.normalize()?;
    let origin_to_center = ray.origin - sphere.center;
    let b = 2.0 * origin_to_center.dot(direction);
    let c = origin_to_center.length_squared() - sphere.radius * sphere.radius;
    let discriminant = b.mul_add(b, -4.0 * c);
    if discriminant < 0.0 {
        return Ok(Vec::new());
    }
    let sqrt_discriminant = discriminant.sqrt();
    let mut distances = [
        (-b - sqrt_discriminant) * 0.5,
        (-b + sqrt_discriminant) * 0.5,
    ];
    distances.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let mut intersections = Vec::new();
    for distance in distances {
        if distance < 0.0 {
            continue;
        }
        if intersections.last().is_some_and(|point: &Point3| {
            point.distance(ray.origin + direction * distance) <= f32::EPSILON
        }) {
            continue;
        }
        intersections.push(ray.origin + direction * distance);
    }
    Ok(intersections)
}

#[inline]
fn ray_axis_interval(origin: f32, direction: f32, min: f32, max: f32) -> Option<AxisInterval> {
    if direction.abs() <= f32::EPSILON {
        return (origin >= min && origin <= max).then_some(AxisInterval {
            near: f32::NEG_INFINITY,
            far: f32::INFINITY,
        });
    }
    let inverse = 1.0 / direction;
    let first = (min - origin) * inverse;
    let second = (max - origin) * inverse;
    Some(AxisInterval {
        near: first.min(second),
        far: first.max(second),
    })
}

#[inline]
fn spheres_overlap(left: Sphere3, right: Sphere3) -> bool {
    let radius_sum = left.radius + right.radius;
    (right.center - left.center).length_squared() <= radius_sum * radius_sum
}

fn validate_ray(ray: Ray3) -> Result<()> {
    validate_points(&[ray.origin])?;
    validate_finite_vector(ray.direction, "ray direction")
}

fn validate_sphere(sphere: Sphere3) -> Result<()> {
    validate_points(&[sphere.center])?;
    if !sphere.radius.is_finite() || sphere.radius <= 0.0 {
        return Err(invalid_argument(
            "sphere radius must be finite and greater than zero",
        ));
    }
    Ok(())
}
