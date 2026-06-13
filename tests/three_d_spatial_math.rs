use runtime_core::{OperationId, SurfaceRequest};
use video_analysis as va;

#[test]
fn core_spatial_math_covers_rotations_transforms_and_cameras(
) -> Result<(), Box<dyn std::error::Error>> {
    let rotation = va::three_d_core::Quaternion::from_axis_angle(
        va::three_d_core::Vector3::new(0.0, 1.0, 0.0),
        std::f32::consts::FRAC_PI_2,
    )?;
    let matrix = rotation.to_rotation_matrix()?;
    let roundtrip = va::three_d_core::Quaternion::from_rotation_matrix(matrix)?;
    assert!((roundtrip.norm() - 1.0).abs() < 1.0e-5);

    let transform = va::three_d_core::TrsTransform3::new(
        va::three_d_core::Vector3::new(1.0, 2.0, 3.0),
        rotation,
        va::three_d_core::Vector3::new(2.0, 2.0, 2.0),
    )?;
    let transformed = transform.apply_point(va::three_d_core::Point3::new(1.0, 0.0, 0.0))?;
    let recovered = transform.to_affine()?.inverse()?.apply_point(transformed)?;
    assert!((recovered.x - 1.0).abs() < 1.0e-4);
    assert!(recovered.y.abs() < 1.0e-4);
    assert!(recovered.z.abs() < 1.0e-4);

    let intrinsics = va::three_d_core::PinholeIntrinsics::new(32, 32, 30.0, 30.0, 15.0, 15.0)?;
    let pose = va::three_d_core::CameraPose3::identity();
    let projected = pose.project_point(intrinsics, va::three_d_core::Point3::new(0.0, 0.0, 3.0))?;
    assert_eq!(projected, Some([15.0, 15.0]));
    let ray = pose.pixel_ray(intrinsics, [15.0, 15.0], 0.0, 100.0)?;
    assert!((ray.direction.z - 1.0).abs() < 1.0e-5);

    let colmap = pose.to_colmap_world_to_camera()?;
    let pose_from_colmap = va::three_d_core::CameraPose3::from_colmap_world_to_camera(
        colmap.qw, colmap.qx, colmap.qy, colmap.qz, colmap.tx, colmap.ty, colmap.tz,
    )?;
    assert!((pose_from_colmap.forward.z - 1.0).abs() < 1.0e-5);

    Ok(())
}

#[test]
fn three_d_surface_exposes_curated_transform_camera_and_debug_operations(
) -> Result<(), Box<dyn std::error::Error>> {
    let surface = va::three_d_core::surface::package_surface();
    let operations = surface
        .operations
        .iter()
        .map(|operation| operation.id.as_str())
        .collect::<Vec<_>>();
    for expected in [
        "threeD.transform.compose",
        "threeD.transform.inverse",
        "threeD.transform.apply",
        "threeD.camera.project",
        "threeD.camera.pixelRay",
        "threeD.camera.viewMatrix",
        "threeD.camera.projectionMatrix",
        "threeD.convert.colmapPose",
        "threeD.convert.gltfMatrix",
        "threeD.debug.matrixInspect",
        "threeD.debug.rotationInspect",
        "threeD.debug.transformDiagnostics",
    ] {
        assert!(
            operations.contains(&expected),
            "missing operation {expected}"
        );
    }

    let projection = va::three_d_core::surface::run_surface_operation(SurfaceRequest {
        operation: OperationId::new("threeD.camera.project"),
        input: serde_json::json!({
            "pose": {"position": [0, 0, 0], "right": [1, 0, 0], "up": [0, 1, 0], "forward": [0, 0, 1]},
            "intrinsics": {"width": 32, "height": 32, "fx": 30, "fy": 30, "cx": 15, "cy": 15},
            "point": [0, 0, 3]
        }),
    })?;
    assert_eq!(projection.value["pixel"], serde_json::json!([15.0, 15.0]));

    let diagnostics = va::three_d_core::surface::run_surface_operation(SurfaceRequest {
        operation: OperationId::new("threeD.debug.rotationInspect"),
        input: serde_json::json!({"quaternion": [0, 0, 0, 1]}),
    })?;
    assert_eq!(diagnostics.value["rotationMatrix"][0][0], 1.0);

    let view = va::three_d_core::surface::run_surface_operation(SurfaceRequest {
        operation: OperationId::new("threeD.camera.viewMatrix"),
        input: serde_json::json!({
            "pose": {"position": [0, 0, 0], "right": [1, 0, 0], "up": [0, 1, 0], "forward": [0, 0, 1]},
            "target": "webgl",
            "precision": "f64"
        }),
    })?;
    assert_eq!(view.value["precision"], "f64");
    assert_eq!(view.value["matrix"][2][2], -1.0);

    let projection_matrix = va::three_d_core::surface::run_surface_operation(SurfaceRequest {
        operation: OperationId::new("threeD.camera.projectionMatrix"),
        input: serde_json::json!({
            "intrinsics": {"width": 32, "height": 32, "fx": 30, "fy": 30, "cx": 15, "cy": 15},
            "near": 0.1,
            "far": 100,
            "target": "webgl"
        }),
    })?;
    assert!(
        projection_matrix.value["matrix"][0][0]
            .as_f64()
            .expect("matrix number")
            > 0.0
    );

    let bad_scale = va::three_d_core::surface::run_surface_operation(SurfaceRequest {
        operation: OperationId::new("threeD.transform.apply"),
        input: serde_json::json!({
            "transform": {"translation": [0, 0, 0], "rotation": [0, 0, 0, 1], "scale": [1, 0, 1]},
            "point": [0, 0, 0]
        }),
    })
    .expect_err("zero scale");
    assert!(bad_scale.contains("scale"));

    let bad_basis = va::three_d_core::surface::run_surface_operation(SurfaceRequest {
        operation: OperationId::new("threeD.camera.viewMatrix"),
        input: serde_json::json!({
            "pose": {"position": [0, 0, 0], "right": [1, 0, 0], "up": [1, 0, 0], "forward": [0, 0, 1]},
            "target": "workspace"
        }),
    })
    .expect_err("non-orthogonal basis");
    assert!(bad_basis.contains("orthogonal"));

    let bad_colmap = va::three_d_core::surface::run_surface_operation(SurfaceRequest {
        operation: OperationId::new("threeD.convert.colmapPose"),
        input: serde_json::json!({"qw": 0, "qx": 0, "qy": 0, "qz": 0, "tx": 0, "ty": 0, "tz": 0}),
    })
    .expect_err("zero quaternion");
    assert!(bad_colmap.contains("quaternion"));

    Ok(())
}

#[test]
fn adapters_bridge_existing_3d_crates_to_core_math() -> Result<(), Box<dyn std::error::Error>> {
    let radiance_intrinsics =
        va::radiance_fields::CameraIntrinsics::new(32, 32, 30.0, 30.0, 15.0, 15.0)?;
    let core_intrinsics = radiance_intrinsics.to_core_pinhole()?;
    assert_eq!(core_intrinsics.fx, 30.0);

    let radiance_pose = va::radiance_fields::CameraPose::look_at(
        va::radiance_fields::Vec3::new(0.0, 0.0, 0.0),
        va::radiance_fields::Vec3::new(0.0, 0.0, 3.0),
        va::radiance_fields::Vec3::new(0.0, 1.0, 0.0),
    )?;
    let core_pose = radiance_pose.to_core_pose()?;
    assert_eq!(core_pose.forward.to_array(), [0.0, 0.0, 1.0]);
    let radiance_roundtrip = va::radiance_fields::CameraPose::from_core_pose(core_pose)?;
    assert_eq!(radiance_roundtrip.forward, radiance_pose.forward);

    let scene_transform = va::three_d_scene::NodeTransform::new(
        va::three_d_core::Vector3::new(1.0, 0.0, 0.0),
        va::three_d_core::Quaternion::IDENTITY,
        va::three_d_core::Vector3::new(1.0, 1.0, 1.0),
    )?;
    assert_eq!(
        scene_transform
            .to_core_trs()?
            .apply_point(va::three_d_core::Point3::new(0.0, 0.0, 0.0))?,
        va::three_d_core::Point3::new(1.0, 0.0, 0.0)
    );

    let splat_quaternion = va::gaussian_splatting::Quaternion::from_core_quaternion(
        va::three_d_core::Quaternion::IDENTITY,
    )?;
    assert_eq!(
        splat_quaternion.to_core_quaternion()?,
        va::three_d_core::Quaternion::IDENTITY
    );

    Ok(())
}

#[test]
fn video_camera_adapters_match_core_projection_and_colmap() -> Result<(), Box<dyn std::error::Error>>
{
    let radiance_pose = va::radiance_fields::CameraPose::from_colmap_world_to_camera(
        1.0, 0.0, 0.0, 0.0, 0.25, -0.5, 1.5,
    )?;
    let core_pose = va::three_d_core::CameraPose3::from_colmap_world_to_camera(
        1.0, 0.0, 0.0, 0.0, 0.25, -0.5, 1.5,
    )?;
    assert_eq!(radiance_pose.to_core_pose()?.position, core_pose.position);
    assert_eq!(radiance_pose.to_core_pose()?.forward, core_pose.forward);

    let intrinsics = va::radiance_fields::CameraIntrinsics::new(64, 48, 40.0, 41.0, 31.5, 23.5)?;
    let point = va::radiance_fields::Vec3::new(0.25, -0.1, 3.0);
    let reconstruction_pixel =
        va::reconstruction::project_point(radiance_pose, intrinsics, point)?.unwrap();
    let core_pixel = radiance_pose
        .to_core_pose()?
        .project_point(intrinsics.to_core_pinhole()?, point.to_core_point())?
        .unwrap();
    assert!((reconstruction_pixel.x - core_pixel[0]).abs() < 1.0e-5);
    assert!((reconstruction_pixel.y - core_pixel[1]).abs() < 1.0e-5);

    Ok(())
}

#[test]
fn gaussian_covariance_uses_core_rotation_matrix_math() -> Result<(), Box<dyn std::error::Error>> {
    let scale = va::radiance_fields::Vec3::new(2.0, 3.0, 4.0);
    let rotation = va::gaussian_splatting::Quaternion::from_core_quaternion(
        va::three_d_core::Quaternion::from_axis_angle(
            va::three_d_core::Vector3::new(0.0, 0.0, 1.0),
            0.4,
        )?,
    )?;
    let local = va::gaussian_splatting::Covariance3::from_scale_rotation(scale, rotation)?;
    let r = rotation.to_core_quaternion()?.to_rotation_matrix()?.rows;
    let variances = [scale.x * scale.x, scale.y * scale.y, scale.z * scale.z];
    let covariance = |row: usize, col: usize| -> f32 {
        r[row][0] * variances[0] * r[col][0]
            + r[row][1] * variances[1] * r[col][1]
            + r[row][2] * variances[2] * r[col][2]
    };
    assert!((local.xx - covariance(0, 0)).abs() < 1.0e-5);
    assert!((local.xy - covariance(0, 1)).abs() < 1.0e-5);
    assert!((local.zz - covariance(2, 2)).abs() < 1.0e-5);
    Ok(())
}
