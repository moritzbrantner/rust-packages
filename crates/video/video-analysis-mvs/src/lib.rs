#![doc = include_str!("../README.md")]

pub mod surface;
use three_d_processing_core::{Point3, PointCloud, Vector3};
use three_d_processing_mesh::Mesh;
use video_analysis_core::{DetectError, Result};
use video_analysis_radiance_fields::{CameraView, CameraViewSet};
use video_analysis_reconstruction::SparseReconstruction;

fn invalid_argument(message: impl Into<String>) -> DetectError {
    DetectError::InvalidArgument(message.into())
}

fn validate_f32(value: f32, name: &str) -> Result<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(invalid_argument(format!("{name} must be finite")))
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for dense reconstruction request.
pub struct MvsRequest {
    /// Sparse reconstruction that seeds dense reconstruction.
    pub sparse_reconstruction: SparseReconstruction,
    /// Camera views used for depth estimation.
    pub views: CameraViewSet,
    /// Optional maximum image size for native backends.
    pub max_image_size: Option<u32>,
    /// Minimum number of consistent views for fusion.
    pub min_consistent_views: usize,
}

impl MvsRequest {
    /// Creates a new value.
    pub fn new(sparse_reconstruction: SparseReconstruction, views: CameraViewSet) -> Result<Self> {
        let request = Self {
            sparse_reconstruction,
            views,
            max_image_size: None,
            min_consistent_views: 2,
        };
        request.validate()?;
        Ok(request)
    }

    /// Returns this value with max image size.
    pub fn max_image_size(mut self, value: u32) -> Result<Self> {
        if value == 0 {
            return Err(invalid_argument("max image size must be positive"));
        }
        self.max_image_size = Some(value);
        Ok(self)
    }

    /// Returns this value with min consistent views.
    pub fn min_consistent_views(mut self, value: usize) -> Result<Self> {
        if value == 0 {
            return Err(invalid_argument("min consistent views must be positive"));
        }
        self.min_consistent_views = value;
        Ok(self)
    }

    /// Validates this value.
    pub fn validate(&self) -> Result<()> {
        self.views.validate()?;
        if let Some(max_image_size) = self.max_image_size {
            if max_image_size == 0 {
                return Err(invalid_argument("max image size must be positive"));
            }
        }
        if self.min_consistent_views == 0 {
            return Err(invalid_argument("min consistent views must be positive"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for a depth map.
pub struct DepthMap {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Depth values in row-major order.
    pub depths: Vec<f32>,
    /// Source view identifier.
    pub view_id: u32,
}

impl DepthMap {
    /// Creates a new value.
    pub fn new(width: u32, height: u32, depths: impl Into<Vec<f32>>, view_id: u32) -> Result<Self> {
        let map = Self {
            width,
            height,
            depths: depths.into(),
            view_id,
        };
        map.validate()?;
        Ok(map)
    }

    /// Validates this value.
    pub fn validate(&self) -> Result<()> {
        if self.width == 0 || self.height == 0 {
            return Err(invalid_argument("depth map dimensions must be positive"));
        }
        let expected = self.width as usize * self.height as usize;
        if self.depths.len() != expected {
            return Err(invalid_argument(format!(
                "depth map expected {expected} values, got {}",
                self.depths.len()
            )));
        }
        for depth in &self.depths {
            validate_f32(*depth, "depth")?;
            if *depth < 0.0 {
                return Err(invalid_argument("depth values must be non-negative"));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for a normal map.
pub struct NormalMap {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Normal values in row-major order.
    pub normals: Vec<Vector3>,
    /// Source view identifier.
    pub view_id: u32,
}

impl NormalMap {
    /// Creates a new value.
    pub fn new(
        width: u32,
        height: u32,
        normals: impl Into<Vec<Vector3>>,
        view_id: u32,
    ) -> Result<Self> {
        let map = Self {
            width,
            height,
            normals: normals.into(),
            view_id,
        };
        map.validate()?;
        Ok(map)
    }

    /// Validates this value.
    pub fn validate(&self) -> Result<()> {
        if self.width == 0 || self.height == 0 {
            return Err(invalid_argument("normal map dimensions must be positive"));
        }
        let expected = self.width as usize * self.height as usize;
        if self.normals.len() != expected {
            return Err(invalid_argument(format!(
                "normal map expected {expected} values, got {}",
                self.normals.len()
            )));
        }
        if self.normals.iter().any(|normal| !normal.is_finite()) {
            return Err(invalid_argument("normal map values must be finite"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for dense reconstruction.
pub struct DenseReconstruction {
    /// Fused point cloud.
    pub point_cloud: PointCloud,
    /// Optional fused mesh.
    pub mesh: Option<Mesh>,
    /// Depth maps used to build this result.
    pub depth_maps: Vec<DepthMap>,
    /// Normal maps used to build this result.
    pub normal_maps: Vec<NormalMap>,
}

impl DenseReconstruction {
    /// Creates a new value.
    pub fn new(point_cloud: PointCloud) -> Self {
        Self {
            point_cloud,
            mesh: None,
            depth_maps: Vec::new(),
            normal_maps: Vec::new(),
        }
    }

    /// Returns this value with mesh.
    pub fn mesh(mut self, mesh: Mesh) -> Result<Self> {
        mesh.validate()?;
        self.mesh = Some(mesh);
        Ok(self)
    }

    /// Returns this value with maps.
    pub fn maps(
        mut self,
        depth_maps: impl Into<Vec<DepthMap>>,
        normal_maps: impl Into<Vec<NormalMap>>,
    ) -> Result<Self> {
        self.depth_maps = depth_maps.into();
        self.normal_maps = normal_maps.into();
        self.validate()?;
        Ok(self)
    }

    /// Validates this value.
    pub fn validate(&self) -> Result<()> {
        self.point_cloud.bounds()?;
        if let Some(mesh) = &self.mesh {
            mesh.validate()?;
        }
        for map in &self.depth_maps {
            map.validate()?;
        }
        for map in &self.normal_maps {
            map.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for dense reconstruction report.
pub struct DenseReconstructionReport {
    /// Backend name associated with this report.
    pub backend: String,
    /// Number of input views.
    pub view_count: usize,
    /// Number of depth maps.
    pub depth_map_count: usize,
    /// Number of fused points.
    pub point_count: usize,
    /// Number of mesh triangles, if a mesh was emitted.
    pub mesh_triangle_count: Option<usize>,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for MVS output.
pub struct MvsOutput {
    /// Dense reconstruction.
    pub dense: DenseReconstruction,
    /// Report.
    pub report: DenseReconstructionReport,
}

/// Trait for depth estimator backends.
pub trait DepthEstimator {
    /// Estimates a depth map for one view.
    fn estimate_depth(&mut self, view: &CameraView, request: &MvsRequest) -> Result<DepthMap>;
}

/// Trait for depth fusion backends.
pub trait DepthFusion {
    /// Fuses depth maps into a dense reconstruction.
    fn fuse_depth_maps(
        &mut self,
        request: &MvsRequest,
        depth_maps: &[DepthMap],
    ) -> Result<DenseReconstruction>;
}

/// Trait for dense reconstructor backends.
pub trait DenseReconstructor {
    /// Returns backend name.
    fn name(&self) -> &'static str;

    /// Reconstructs dense geometry.
    fn reconstruct_dense(&mut self, request: &MvsRequest) -> Result<MvsOutput>;
}

#[derive(Debug)]
/// Data type for MVS pipeline.
pub struct MvsPipeline<B> {
    backend: B,
}

impl<B: DenseReconstructor> MvsPipeline<B> {
    /// Creates a new value.
    pub fn new(backend: B) -> Self {
        Self { backend }
    }

    /// Runs the pipeline.
    pub fn run(&mut self, request: &MvsRequest) -> Result<MvsOutput> {
        request.validate()?;
        self.backend.reconstruct_dense(request)
    }
}

#[derive(Debug, Clone, Default)]
/// Dense backend that normalizes sparse points into a dense output placeholder.
pub struct SparsePointCloudDenseReconstructor;

impl DenseReconstructor for SparsePointCloudDenseReconstructor {
    fn name(&self) -> &'static str {
        "sparse-point-cloud-dense-reconstructor"
    }

    fn reconstruct_dense(&mut self, request: &MvsRequest) -> Result<MvsOutput> {
        request.validate()?;
        let points = request
            .sparse_reconstruction
            .points()
            .values()
            .map(|point| Point3::new(point.position.x, point.position.y, point.position.z))
            .collect::<Vec<_>>();
        let point_cloud = PointCloud::new(points)?;
        let dense = DenseReconstruction::new(point_cloud);
        let report = dense_report(self.name(), request.views.views.len(), &dense);
        Ok(MvsOutput { dense, report })
    }
}

/// Returns dense reconstruction report.
pub fn dense_report(
    backend: impl Into<String>,
    view_count: usize,
    dense: &DenseReconstruction,
) -> DenseReconstructionReport {
    DenseReconstructionReport {
        backend: backend.into(),
        view_count,
        depth_map_count: dense.depth_maps.len(),
        point_count: dense.point_cloud.points().len(),
        mesh_triangle_count: dense.mesh.as_ref().map(|mesh| mesh.triangles.len()),
    }
}

#[cfg(test)]
mod tests {
    use video_analysis_radiance_fields::{CameraIntrinsics, CameraPose, CameraView};

    use super::*;

    #[test]
    fn depth_maps_validate_shape_and_values() {
        let map = DepthMap::new(2, 1, [1.0, 2.0], 7).unwrap();
        assert_eq!(map.depths.len(), 2);
        assert!(DepthMap::new(2, 1, [1.0], 7).is_err());
    }

    #[test]
    fn dense_report_counts_points_and_maps() {
        let dense = DenseReconstruction::new(
            PointCloud::new([Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0)]).unwrap(),
        )
        .maps([DepthMap::new(1, 1, [2.0], 1).unwrap()], [])
        .unwrap();
        let report = dense_report("test", 3, &dense);
        assert_eq!(report.view_count, 3);
        assert_eq!(report.depth_map_count, 1);
        assert_eq!(report.point_count, 2);
    }

    #[test]
    fn sparse_point_cloud_backend_normalizes_output() {
        let intrinsics = CameraIntrinsics::pinhole(16, 16, 1.0).unwrap();
        let views = CameraViewSet {
            views: vec![CameraView {
                id: 1,
                name: "a.png".to_string(),
                intrinsics,
                distortion: None,
                pose: CameraPose::identity(),
            }],
        };
        let request = MvsRequest::new(SparseReconstruction::new(), views).unwrap();
        let mut pipeline = MvsPipeline::new(SparsePointCloudDenseReconstructor);
        let output = pipeline.run(&request).unwrap();
        assert_eq!(output.report.point_count, 0);
    }
}
