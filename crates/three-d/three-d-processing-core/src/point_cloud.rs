use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use video_analysis_core::Result;

use crate::{
    invalid_argument, validate_points, Bounds3, Point3, RigidTransform3, Transform3, Vector3,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Data type for point cloud.
pub struct PointCloud {
    points: Vec<Point3>,
}

impl PointCloud {
    /// Creates a new value.
    pub fn new(points: impl Into<Vec<Point3>>) -> Result<Self> {
        let points = points.into();
        validate_points(&points)?;
        Ok(Self { points })
    }

    /// Returns points.
    pub fn points(&self) -> &[Point3] {
        &self.points
    }

    /// Returns bounds.
    pub fn bounds(&self) -> Result<Option<Bounds3>> {
        Bounds3::from_points(&self.points)
    }

    /// Returns centroid.
    pub fn centroid(&self) -> Result<Option<Point3>> {
        centroid(&self.points)
    }

    /// Returns transformed.
    pub fn transformed(&self, transform: Transform3) -> Result<Self> {
        PointCloud::new(
            self.points
                .iter()
                .copied()
                .map(|point| transform.apply_point(point))
                .collect::<Vec<_>>(),
        )
    }

    /// Returns transformed rigid.
    pub fn transformed_rigid(&self, transform: RigidTransform3) -> Result<Self> {
        PointCloud::new(transform_rigid(&self.points, transform)?)
    }

    /// Returns voxel downsample.
    pub fn voxel_downsample(&self, voxel_size: f32) -> Result<Self> {
        PointCloud::new(voxel_downsample(&self.points, voxel_size)?)
    }

    /// Returns center and scale.
    pub fn center_and_scale(&self, target_extent: f32) -> Result<Option<Self>> {
        center_and_scale(&self.points, target_extent)
            .map(|value| value.map(|points| Self { points }))
    }

    /// Returns nearest point.
    pub fn nearest_point(&self, query: Point3) -> Result<Option<Point3>> {
        nearest_point(&self.points, query)
    }
}

/// Returns centroid.
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

/// Returns point distance.
pub fn point_distance(a: Point3, b: Point3) -> Result<f32> {
    validate_points(&[a, b])?;
    Ok(a.distance(b))
}

/// Returns nearest point.
pub fn nearest_point(points: &[Point3], query: Point3) -> Result<Option<Point3>> {
    validate_points(points)?;
    validate_points(&[query])?;
    Ok(points.iter().copied().min_by(|a, b| {
        a.distance(query)
            .partial_cmp(&b.distance(query))
            .unwrap_or(std::cmp::Ordering::Equal)
    }))
}

/// Returns transform rigid.
pub fn transform_rigid(points: &[Point3], transform: RigidTransform3) -> Result<Vec<Point3>> {
    validate_points(points)?;
    points
        .iter()
        .copied()
        .map(|point| transform.apply_point(point))
        .collect()
}

/// Returns voxel downsample.
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

/// Returns center and scale.
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
    let Some(bounds) = Bounds3::from_points(points)? else {
        return Ok(None);
    };
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
