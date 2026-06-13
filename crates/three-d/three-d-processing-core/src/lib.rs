#![doc = include_str!("../README.md")]

mod bounds;
mod broad_phase;
mod geometry;
mod math;
mod point_cloud;
mod spatial_math;
pub mod surface;
mod transform;
mod validation;

pub use bounds::*;
pub use broad_phase::*;
pub use geometry::*;
pub use math::*;
pub use point_cloud::*;
pub use spatial_math::*;
pub use transform::*;

#[cfg(test)]
pub(crate) use broad_phase::select_strategy_3d;
pub(crate) use validation::{invalid_argument, validate_finite_vector, validate_points};

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

    #[test]
    fn transform_helpers_round_trip_points() {
        let transform = Transform3::translation(Vector3::new(2.0, 0.0, 0.0))
            .compose(Transform3::scaling(3.0).unwrap())
            .unwrap();
        let point = Point3::new(1.0, 2.0, 3.0);
        let transformed = transform.apply_point(point);
        let recovered = transform.inverse().unwrap().apply_point(transformed);
        assert!((recovered.x - point.x).abs() < 0.001);
        assert!((recovered.y - point.y).abs() < 0.001);
        assert!((recovered.z - point.z).abs() < 0.001);
    }

    #[test]
    fn quaternion_slerp_and_spatial_primitives_are_stable() {
        let identity = Quaternion::IDENTITY;
        let half_turn =
            Quaternion::from_axis_angle(Vector3::new(0.0, 0.0, 1.0), std::f32::consts::PI).unwrap();
        let midpoint = identity.slerp(half_turn, 0.5).unwrap();
        assert!((midpoint.norm() - 1.0).abs() < 0.001);

        let ray = Ray3::new(Point3::new(0.0, 0.0, 0.0), Vector3::new(0.0, 2.0, 0.0)).unwrap();
        assert_eq!(ray.at(2.0).unwrap(), Point3::new(0.0, 2.0, 0.0));

        let plane =
            Plane3::from_point_normal(Point3::new(0.0, 1.0, 0.0), Vector3::new(0.0, 1.0, 0.0))
                .unwrap();
        assert!((plane.signed_distance(Point3::new(0.0, 3.0, 0.0)).unwrap() - 2.0).abs() < 0.001);

        let sphere = Sphere3::new(Point3::new(0.0, 0.0, 0.0), 1.0).unwrap();
        assert!(sphere.contains_point(Point3::new(0.5, 0.0, 0.0)).unwrap());
    }

    #[test]
    fn point_cloud_reports_nearest_point() {
        let cloud =
            PointCloud::new([Point3::new(-1.0, 0.0, 0.0), Point3::new(2.0, 0.0, 0.0)]).unwrap();
        assert_eq!(
            cloud.nearest_point(Point3::new(1.5, 0.0, 0.0)).unwrap(),
            Some(Point3::new(2.0, 0.0, 0.0))
        );
    }

    #[test]
    fn closest_point_helpers_clamp_to_segment_and_ray() {
        let segment =
            LineSegment3::new(Point3::new(0.0, 0.0, 0.0), Point3::new(2.0, 0.0, 0.0)).unwrap();
        assert_eq!(
            segment.closest_point(Point3::new(1.0, 2.0, 0.0)).unwrap(),
            Point3::new(1.0, 0.0, 0.0)
        );
        assert_eq!(
            segment.closest_point(Point3::new(4.0, 0.0, 0.0)).unwrap(),
            Point3::new(2.0, 0.0, 0.0)
        );

        let ray = Ray3::new(Point3::new(0.0, 0.0, 0.0), Vector3::new(1.0, 0.0, 0.0)).unwrap();
        assert_eq!(
            ray.closest_point(Point3::new(-1.0, 2.0, 0.0)).unwrap(),
            Point3::new(0.0, 0.0, 0.0)
        );
    }

    #[test]
    fn plane_projection_and_ray_intersection_are_stable() {
        let plane =
            Plane3::from_point_normal(Point3::new(0.0, 2.0, 0.0), Vector3::new(0.0, 1.0, 0.0))
                .unwrap();
        assert_eq!(
            plane.project_point(Point3::new(1.0, 5.0, 1.0)).unwrap(),
            Point3::new(1.0, 2.0, 1.0)
        );

        let ray = Ray3::new(Point3::new(0.0, 5.0, 0.0), Vector3::new(0.0, -1.0, 0.0)).unwrap();
        assert_eq!(
            plane.intersect_ray(ray).unwrap(),
            Some(Point3::new(0.0, 2.0, 0.0))
        );
    }

    #[test]
    fn bounds_collision_helpers_cover_points_spheres_and_rays() {
        let bounds =
            Bounds3::new(Point3::new(-1.0, -1.0, -1.0), Point3::new(1.0, 1.0, 1.0)).unwrap();

        assert_eq!(
            bounds.closest_point(Point3::new(2.0, 0.5, -3.0)).unwrap(),
            Point3::new(1.0, 0.5, -1.0)
        );
        assert!(
            (bounds
                .distance_to_point(Point3::new(3.0, 0.0, 0.0))
                .unwrap()
                - 2.0)
                .abs()
                < 0.001
        );
        assert!(bounds
            .intersects_sphere(Sphere3::new(Point3::new(1.5, 0.0, 0.0), 0.5).unwrap())
            .unwrap());

        let ray = Ray3::new(Point3::new(0.0, 0.0, -3.0), Vector3::new(0.0, 0.0, 1.0)).unwrap();
        let hit = bounds.intersect_ray(ray).unwrap().unwrap();
        assert!((hit.entry_distance - 2.0).abs() < 0.001);
        assert!((hit.exit_distance - 4.0).abs() < 0.001);
        assert_eq!(hit.entry_point, Point3::new(0.0, 0.0, -1.0));

        let inside_ray =
            Ray3::new(Point3::new(0.0, 0.0, 0.0), Vector3::new(1.0, 0.0, 0.0)).unwrap();
        let inside_hit = intersect_ray_bounds(inside_ray, bounds).unwrap().unwrap();
        assert_eq!(inside_hit.entry_distance, 0.0);
        assert_eq!(inside_hit.exit_point, Point3::new(1.0, 0.0, 0.0));
    }

    #[test]
    fn bounds_collision_helpers_report_misses_and_tangents() {
        let bounds =
            Bounds3::new(Point3::new(-1.0, -1.0, -1.0), Point3::new(1.0, 1.0, 1.0)).unwrap();

        let parallel_miss =
            Ray3::new(Point3::new(2.0, 0.0, -3.0), Vector3::new(0.0, 0.0, 1.0)).unwrap();
        assert_eq!(bounds.intersect_ray(parallel_miss).unwrap(), None);

        let behind_ray =
            Ray3::new(Point3::new(0.0, 0.0, 3.0), Vector3::new(0.0, 0.0, 1.0)).unwrap();
        assert_eq!(intersect_ray_bounds(behind_ray, bounds).unwrap(), None);

        let tangent_ray =
            Ray3::new(Point3::new(1.0, 0.0, -3.0), Vector3::new(0.0, 0.0, 1.0)).unwrap();
        let tangent_hit = bounds.intersect_ray(tangent_ray).unwrap().unwrap();
        assert_eq!(tangent_hit.entry_point, Point3::new(1.0, 0.0, -1.0));
        assert_eq!(tangent_hit.exit_point, Point3::new(1.0, 0.0, 1.0));

        assert!(!bounds
            .intersects_sphere(Sphere3::new(Point3::new(2.1, 0.0, 0.0), 1.0).unwrap())
            .unwrap());
        assert!(sphere_intersects_bounds(
            Sphere3::new(Point3::new(2.0, 0.0, 0.0), 1.0).unwrap(),
            bounds
        )
        .unwrap());
    }

    #[test]
    fn sphere_algorithms_report_surface_volume_and_intersections() {
        let sphere = Sphere3::new(Point3::new(0.0, 0.0, 0.0), 2.0).unwrap();
        assert!((sphere.surface_area() - (16.0 * std::f32::consts::PI)).abs() < 0.001);
        assert!((sphere.volume() - ((32.0 / 3.0) * std::f32::consts::PI)).abs() < 0.001);
        assert!((sphere.signed_distance(Point3::new(3.0, 0.0, 0.0)).unwrap() - 1.0).abs() < 0.001);
        assert_eq!(
            sphere.closest_point(Point3::new(3.0, 0.0, 0.0)).unwrap(),
            Point3::new(2.0, 0.0, 0.0)
        );

        let ray = Ray3::new(Point3::new(-3.0, 0.0, 0.0), Vector3::new(1.0, 0.0, 0.0)).unwrap();
        let intersections = sphere.intersect_ray(ray).unwrap();
        assert_eq!(
            intersections,
            vec![Point3::new(-2.0, 0.0, 0.0), Point3::new(2.0, 0.0, 0.0)]
        );
    }

    #[test]
    fn sphere_collision_reports_contact_data() {
        let left = Sphere3::new(Point3::new(0.0, 0.0, 0.0), 1.0).unwrap();
        let right = Sphere3::new(Point3::new(1.5, 0.0, 0.0), 1.0).unwrap();
        let collision = left.collision_with_sphere(right).unwrap().unwrap();

        assert_eq!(collision.normal, Vector3::new(1.0, 0.0, 0.0));
        assert_eq!(collision.point, Point3::new(0.75, 0.0, 0.0));
        assert!((collision.penetration_depth - 0.5).abs() < 0.001);
        assert!(left.intersects_sphere(right).unwrap());
        assert!(!left
            .intersects_sphere(Sphere3::new(Point3::new(3.0, 0.0, 0.0), 1.0).unwrap())
            .unwrap());
    }

    #[test]
    fn sphere_collision_handles_tangent_and_concentric_cases() {
        let left = Sphere3::new(Point3::new(0.0, 0.0, 0.0), 1.0).unwrap();
        let tangent = Sphere3::new(Point3::new(2.0, 0.0, 0.0), 1.0).unwrap();
        let tangent_collision = collision_sphere_sphere(left, tangent).unwrap().unwrap();
        assert_eq!(tangent_collision.penetration_depth, 0.0);
        assert_eq!(tangent_collision.point, Point3::new(1.0, 0.0, 0.0));

        let separate = Sphere3::new(Point3::new(2.01, 0.0, 0.0), 1.0).unwrap();
        assert_eq!(collision_sphere_sphere(left, separate).unwrap(), None);

        let concentric = Sphere3::new(Point3::new(0.0, 0.0, 0.0), 0.5).unwrap();
        let concentric_collision = left.collision_with_sphere(concentric).unwrap().unwrap();
        assert_eq!(concentric_collision.normal, Vector3::new(1.0, 0.0, 0.0));
        assert!((concentric_collision.penetration_depth - 1.5).abs() < 0.001);
    }

    #[test]
    fn bounds3_validates_and_reports_basic_operations() {
        assert!(Bounds3::new(Point3::new(f32::NAN, 0.0, 0.0), Point3::new(1.0, 1.0, 1.0)).is_err());
        assert!(Bounds3::new(Point3::new(2.0, 0.0, 0.0), Point3::new(1.0, 1.0, 1.0)).is_err());

        let a = Bounds3::new(Point3::new(-1.0, -1.0, -1.0), Point3::new(2.0, 2.0, 2.0)).unwrap();
        let b = Bounds3::new(Point3::new(1.0, 1.0, 1.0), Point3::new(3.0, 3.0, 3.0)).unwrap();
        let touching =
            Bounds3::new(Point3::new(2.0, 0.0, 0.0), Point3::new(4.0, 1.0, 1.0)).unwrap();

        assert!(a.contains_point(Point3::new(0.0, 0.0, 0.0)).unwrap());
        assert!(a.intersects(b).unwrap());
        assert!(!a.intersects(touching).unwrap());
        assert_eq!(
            a.intersection(b).unwrap(),
            Some(Bounds3::new(Point3::new(1.0, 1.0, 1.0), Point3::new(2.0, 2.0, 2.0)).unwrap())
        );
        assert_eq!(
            a.union(b).unwrap(),
            Bounds3::new(Point3::new(-1.0, -1.0, -1.0), Point3::new(3.0, 3.0, 3.0)).unwrap()
        );
        assert_eq!(a.volume().unwrap(), 27.0);
    }

    #[test]
    fn broad_phase_3d_strategies_match_for_mixed_bounds() {
        let bounds = [
            Bounds3::new(Point3::new(0.0, 0.0, 0.0), Point3::new(2.0, 2.0, 2.0)).unwrap(),
            Bounds3::new(Point3::new(1.0, 1.0, 1.0), Point3::new(3.0, 3.0, 3.0)).unwrap(),
            Bounds3::new(Point3::new(10.0, 0.0, 0.0), Point3::new(12.0, 2.0, 2.0)).unwrap(),
            Bounds3::new(Point3::new(10.5, 0.0, 0.0), Point3::new(11.0, 1.0, 1.0)).unwrap(),
        ];
        let brute = broad_phase_pairs_3d(
            &bounds,
            BroadPhase3Config {
                strategy: BroadPhase3Strategy::BruteForce,
                ..BroadPhase3Config::default()
            },
        )
        .unwrap();
        let sweep = broad_phase_pairs_3d(
            &bounds,
            BroadPhase3Config {
                strategy: BroadPhase3Strategy::SweepAndPrune,
                ..BroadPhase3Config::default()
            },
        )
        .unwrap();
        let grid = broad_phase_pairs_3d(
            &bounds,
            BroadPhase3Config {
                strategy: BroadPhase3Strategy::SpatialHashGrid,
                cell_size: SpatialCellSize3::Fixed { size: 1.0 },
                ..BroadPhase3Config::default()
            },
        )
        .unwrap();

        assert_eq!(brute, sweep);
        assert_eq!(brute, grid);
        assert_eq!(
            brute,
            vec![
                CollisionPair {
                    left_index: 0,
                    right_index: 1
                },
                CollisionPair {
                    left_index: 2,
                    right_index: 3
                }
            ]
        );
    }

    #[test]
    fn broad_phase_3d_auto_uses_sweep_for_large_cell_spans() {
        let bounds = [
            Bounds3::new(Point3::new(0.0, 0.0, 0.0), Point3::new(100.0, 100.0, 100.0)).unwrap(),
            Bounds3::new(Point3::new(1.0, 1.0, 1.0), Point3::new(2.0, 2.0, 2.0)).unwrap(),
        ];
        let strategy = select_strategy_3d(
            &bounds,
            BroadPhase3Config {
                strategy: BroadPhase3Strategy::Auto,
                brute_force_threshold: 0,
                max_cells_per_item: 4,
                cell_size: SpatialCellSize3::Fixed { size: 1.0 },
            },
        )
        .unwrap();

        assert_eq!(strategy, BroadPhase3Strategy::SweepAndPrune);
    }

    #[test]
    fn spatial_hash_grid_3d_reports_cross_set_pairs() {
        let left = [
            Bounds3::new(Point3::new(0.0, 0.0, 0.0), Point3::new(2.0, 2.0, 2.0)).unwrap(),
            Bounds3::new(Point3::new(10.0, 0.0, 0.0), Point3::new(12.0, 2.0, 2.0)).unwrap(),
        ];
        let right = [
            Bounds3::new(Point3::new(1.0, 1.0, 1.0), Point3::new(3.0, 3.0, 3.0)).unwrap(),
            Bounds3::new(Point3::new(30.0, 0.0, 0.0), Point3::new(31.0, 1.0, 1.0)).unwrap(),
        ];
        let mut grid = SpatialHashGrid3::new(BroadPhase3Config {
            strategy: BroadPhase3Strategy::SpatialHashGrid,
            cell_size: SpatialCellSize3::Fixed { size: 1.0 },
            ..BroadPhase3Config::default()
        })
        .unwrap();
        let pairs = grid.candidate_pairs_between(&left, &right).unwrap();

        assert_eq!(
            pairs,
            &[CollisionPair {
                left_index: 0,
                right_index: 0
            }]
        );
        assert_eq!(grid.stats().object_count, 4);
        assert_eq!(grid.stats().candidate_pair_count, 1);
    }
}
