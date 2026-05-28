use std::collections::BTreeMap;

use serde::Serialize;
use video_analysis_core::Result;
use video_analysis_radiance_fields::{CameraPose, Vec3};
use video_analysis_radiance_io::ColmapDataset;

use crate::ColmapBaseline;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// Sparse reconstruction summary for browser display.
pub struct ColmapSceneSummary {
    /// Number of COLMAP cameras.
    pub camera_count: usize,
    /// Number of registered images.
    pub registered_image_count: usize,
    /// Number of sparse points.
    pub sparse_point_count: usize,
    /// Sparse point track-length histogram.
    pub track_length_histogram: BTreeMap<usize, usize>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
/// Browser-friendly sparse COLMAP scene.
pub struct ColmapScene {
    /// Registered camera poses.
    pub cameras: Vec<ColmapSceneCamera>,
    /// Camera positions in registration order.
    pub camera_path: Vec<[f32; 3]>,
    /// Sparse 3D points.
    pub points: Vec<ColmapScenePoint>,
    /// Scene bounds.
    pub bounds: ColmapSceneBounds,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
/// Browser-friendly camera pose.
pub struct ColmapSceneCamera {
    /// COLMAP image identifier.
    pub id: u32,
    /// COLMAP image name.
    pub name: String,
    /// Camera center.
    pub position: [f32; 3],
    /// Camera forward vector.
    pub forward: [f32; 3],
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
/// Browser-friendly sparse point.
pub struct ColmapScenePoint {
    /// COLMAP point identifier.
    pub id: u64,
    /// Point position.
    pub position: [f32; 3],
    /// Point color in 8-bit RGB.
    pub color: [u8; 3],
    /// COLMAP reprojection error.
    pub error: f32,
    /// Track length.
    pub track_length: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
/// Browser-friendly scene bounds.
pub struct ColmapSceneBounds {
    /// Minimum xyz.
    pub min: [f32; 3],
    /// Maximum xyz.
    pub max: [f32; 3],
}

/// Builds browser-friendly scene data from a loaded COLMAP baseline.
pub fn build_colmap_scene(baseline: &ColmapBaseline) -> Result<ColmapScene> {
    build_colmap_scene_from_dataset(&baseline.dataset)
}

pub(crate) fn build_colmap_scene_from_dataset(dataset: &ColmapDataset) -> Result<ColmapScene> {
    let mut cameras = Vec::with_capacity(dataset.images.len());
    let mut camera_path = Vec::with_capacity(dataset.images.len());
    for image in &dataset.images {
        let pose = CameraPose::from_colmap_world_to_camera(
            image.qw, image.qx, image.qy, image.qz, image.tx, image.ty, image.tz,
        )?;
        let position = vec3_array(pose.position);
        camera_path.push(position);
        cameras.push(ColmapSceneCamera {
            id: image.id,
            name: image.name.clone(),
            position,
            forward: vec3_array(pose.forward),
        });
    }

    let points = dataset
        .points
        .iter()
        .map(|point| ColmapScenePoint {
            id: point.id,
            position: vec3_array(point.xyz),
            color: [
                color_channel(point.color.r),
                color_channel(point.color.g),
                color_channel(point.color.b),
            ],
            error: point.error,
            track_length: point.track.len(),
        })
        .collect::<Vec<_>>();

    let bounds = scene_bounds(&cameras, &points);
    Ok(ColmapScene {
        cameras,
        camera_path,
        points,
        bounds,
    })
}

pub(crate) fn scene_summary_from_dataset(dataset: &ColmapDataset) -> ColmapSceneSummary {
    let mut track_length_histogram = BTreeMap::new();
    for point in &dataset.points {
        *track_length_histogram.entry(point.track.len()).or_insert(0) += 1;
    }
    ColmapSceneSummary {
        camera_count: dataset.cameras.len(),
        registered_image_count: dataset.images.len(),
        sparse_point_count: dataset.points.len(),
        track_length_histogram,
    }
}

pub(crate) fn empty_colmap_scene() -> ColmapScene {
    ColmapScene {
        cameras: Vec::new(),
        camera_path: Vec::new(),
        points: Vec::new(),
        bounds: ColmapSceneBounds {
            min: [0.0, 0.0, 0.0],
            max: [1.0, 1.0, 1.0],
        },
    }
}

pub(crate) fn empty_colmap_scene_summary() -> ColmapSceneSummary {
    ColmapSceneSummary {
        camera_count: 0,
        registered_image_count: 0,
        sparse_point_count: 0,
        track_length_histogram: BTreeMap::new(),
    }
}

fn vec3_array(value: Vec3) -> [f32; 3] {
    [value.x, value.y, value.z]
}

fn color_channel(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn scene_bounds(cameras: &[ColmapSceneCamera], points: &[ColmapScenePoint]) -> ColmapSceneBounds {
    let mut values = cameras
        .iter()
        .map(|camera| camera.position)
        .chain(points.iter().map(|point| point.position));
    let Some(first) = values.next() else {
        return ColmapSceneBounds {
            min: [0.0, 0.0, 0.0],
            max: [1.0, 1.0, 1.0],
        };
    };
    let mut min = first;
    let mut max = first;
    for value in values {
        for axis in 0..3 {
            min[axis] = min[axis].min(value[axis]);
            max[axis] = max[axis].max(value[axis]);
        }
    }
    for axis in 0..3 {
        if (max[axis] - min[axis]).abs() < 1.0e-4 {
            min[axis] -= 0.5;
            max[axis] += 0.5;
        }
    }
    ColmapSceneBounds { min, max }
}
