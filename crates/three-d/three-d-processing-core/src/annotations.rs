//! Typed, product-agnostic spatial annotation contracts.
//!
//! This module deliberately does not define COLMAP, Gaussian-splatting, GIS, image, or video
//! product models. It provides a small interoperability vocabulary for coordinate frames,
//! geographic anchors, 3D selections, poses, camera poses, and correspondences to selectors owned
//! by other capabilities. Canonical spatial math remains owned by this crate; media, image, video,
//! and geo packages can compose these values at their own boundaries without becoming dependencies
//! of the 3D core.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use video_analysis_core::Result;

use crate::{
    invalid_argument, CameraPose3d, PinholeIntrinsicsd, Point3d, Quaterniond, RigidTransform3d,
    SimilarityTransform3d,
};

/// Current serialized schema version for [`SpatialBinding`].
pub const SPATIAL_ANNOTATION_SCHEMA_VERSION: u32 = 1;

/// Broad semantic role of a coordinate frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoordinateFrameKind {
    /// A local Cartesian frame such as a reconstruction or scene frame.
    Local,
    /// A geographic or geodetic frame.
    Geographic,
    /// A camera-local frame.
    Camera,
    /// An image or pixel-space frame.
    Image,
    /// A domain-specific frame whose semantics are carried by its id/convention metadata.
    Custom,
}

/// Unit used by coordinates in a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoordinateUnit {
    /// Meters.
    Meter,
    /// Centimeters.
    Centimeter,
    /// Millimeters.
    Millimeter,
    /// Image pixels.
    Pixel,
    /// Angular degrees.
    Degree,
    /// Dimensionless normalized coordinates.
    Unitless,
    /// Scale is intentionally unknown or reconstruction-relative.
    Arbitrary,
}

/// Stable reference to a coordinate frame plus enough metadata to interpret its coordinates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoordinateFrameRef {
    /// Stable frame identifier, for example `colmap-world` or `camera-17`.
    pub id: String,
    /// Semantic role of the frame.
    pub kind: CoordinateFrameKind,
    /// Optional coordinate unit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<CoordinateUnit>,
    /// Optional explicit axis convention, kept open so adapters can name external conventions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub axis_convention: Option<String>,
}

impl CoordinateFrameRef {
    /// Creates a coordinate-frame reference.
    pub fn new(id: impl Into<String>, kind: CoordinateFrameKind) -> Result<Self> {
        let frame = Self {
            id: id.into(),
            kind,
            unit: None,
            axis_convention: None,
        };
        frame.validate()?;
        Ok(frame)
    }

    /// Creates a local Cartesian frame.
    pub fn local(id: impl Into<String>) -> Result<Self> {
        Self::new(id, CoordinateFrameKind::Local)
    }

    /// Returns this frame with an explicit unit.
    pub fn unit(mut self, unit: CoordinateUnit) -> Self {
        self.unit = Some(unit);
        self
    }

    /// Returns this frame with an explicit axis-convention label.
    pub fn axis_convention(mut self, convention: impl Into<String>) -> Result<Self> {
        let convention = convention.into();
        validate_non_empty(&convention, "axis convention")?;
        self.axis_convention = Some(convention);
        Ok(self)
    }

    /// Validates required frame metadata.
    pub fn validate(&self) -> Result<()> {
        validate_non_empty(&self.id, "coordinate frame id")?;
        if let Some(convention) = &self.axis_convention {
            validate_non_empty(convention, "axis convention")?;
        }
        Ok(())
    }
}

/// Similarity transform between two Cartesian coordinate frames.
///
/// `transform` maps a point expressed in `from` into `to`. Similarity rather than rigid transforms
/// are used because reconstruction spaces such as COLMAP can have arbitrary scale.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoordinateFrameTransform3d {
    /// Source frame.
    pub from: CoordinateFrameRef,
    /// Destination frame.
    pub to: CoordinateFrameRef,
    /// Source-to-destination transform.
    pub transform: SimilarityTransform3d,
}

impl CoordinateFrameTransform3d {
    /// Creates a validated Cartesian frame transform.
    pub fn new(
        from: CoordinateFrameRef,
        to: CoordinateFrameRef,
        transform: SimilarityTransform3d,
    ) -> Result<Self> {
        let transform = SimilarityTransform3d::new(
            transform.translation,
            transform.rotation,
            transform.scale,
        )?;
        let value = Self {
            from,
            to,
            transform,
        };
        value.validate()?;
        Ok(value)
    }

    /// Validates both frames and transform values.
    pub fn validate(&self) -> Result<()> {
        self.from.validate()?;
        self.to.validate()?;
        if self.from.id == self.to.id {
            return Err(invalid_argument(
                "coordinate frame transform must reference two different frame ids",
            ));
        }
        SimilarityTransform3d::new(
            self.transform.translation,
            self.transform.rotation,
            self.transform.scale,
        )?;
        Ok(())
    }
}

/// WGS84 longitude/latitude position with optional ellipsoidal altitude.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeographicPosition {
    /// Longitude in degrees in the closed range [-180, 180].
    pub longitude_degrees: f64,
    /// Latitude in degrees in the closed range [-90, 90].
    pub latitude_degrees: f64,
    /// Optional altitude in meters above the WGS84 ellipsoid.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub altitude_meters: Option<f64>,
}

impl GeographicPosition {
    /// Creates a WGS84 geographic position.
    pub fn new(
        longitude_degrees: f64,
        latitude_degrees: f64,
        altitude_meters: Option<f64>,
    ) -> Result<Self> {
        let position = Self {
            longitude_degrees,
            latitude_degrees,
            altitude_meters,
        };
        position.validate()?;
        Ok(position)
    }

    /// Validates WGS84 coordinate ranges and finite altitude.
    pub fn validate(self) -> Result<()> {
        if !self.longitude_degrees.is_finite()
            || !(-180.0..=180.0).contains(&self.longitude_degrees)
        {
            return Err(invalid_argument(
                "longitude_degrees must be finite and in [-180, 180]",
            ));
        }
        if !self.latitude_degrees.is_finite()
            || !(-90.0..=90.0).contains(&self.latitude_degrees)
        {
            return Err(invalid_argument(
                "latitude_degrees must be finite and in [-90, 90]",
            ));
        }
        if self.altitude_meters.is_some_and(|value| !value.is_finite()) {
            return Err(invalid_argument("altitude_meters must be finite when present"));
        }
        Ok(())
    }
}

/// Local tangent-frame convention used to orient a Cartesian scene at a geographic anchor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GeographicTangentFrame {
    /// East, north, up.
    EastNorthUp,
    /// North, east, down.
    NorthEastDown,
}

/// Georeferencing anchor for a local Cartesian frame.
///
/// This keeps non-linear global geographic coordinates out of Cartesian transforms. The local
/// frame origin is anchored at `origin`; `orientation` rotates local-frame vectors into the chosen
/// tangent frame, and `meters_per_unit` establishes reconstruction scale.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeographicFrameAnchor {
    /// Local scene/reconstruction frame being anchored.
    pub frame: CoordinateFrameRef,
    /// WGS84 position of the local frame origin.
    pub origin: GeographicPosition,
    /// Tangent convention at the geographic origin.
    pub tangent_frame: GeographicTangentFrame,
    /// Rotation from local-frame axes into tangent-frame axes.
    pub orientation: Quaterniond,
    /// Uniform metric scale for local coordinates.
    pub meters_per_unit: f64,
}

impl GeographicFrameAnchor {
    /// Creates a validated geographic frame anchor.
    pub fn new(
        frame: CoordinateFrameRef,
        origin: GeographicPosition,
        tangent_frame: GeographicTangentFrame,
        orientation: Quaterniond,
        meters_per_unit: f64,
    ) -> Result<Self> {
        let value = Self {
            frame,
            origin,
            tangent_frame,
            orientation: orientation.normalize()?,
            meters_per_unit,
        };
        value.validate()?;
        Ok(value)
    }

    /// Validates the frame, anchor, orientation, and positive scale.
    pub fn validate(&self) -> Result<()> {
        self.frame.validate()?;
        self.origin.validate()?;
        self.orientation.normalize()?;
        if !self.meters_per_unit.is_finite() || self.meters_per_unit <= 0.0 {
            return Err(invalid_argument(
                "meters_per_unit must be finite and greater than zero",
            ));
        }
        Ok(())
    }
}

/// Independent uncertainty attached to a spatial location or orientation.
///
/// This is intentionally separate from annotation confidence: semantic confidence answers whether
/// a finding is believed, while these fields describe where or how accurately it is located.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpatialUncertainty {
    /// Optional non-negative linear radius in the selected frame's unit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linear_radius: Option<f64>,
    /// Optional non-negative angular uncertainty in radians.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub angular_radians: Option<f64>,
}

impl SpatialUncertainty {
    /// Validates finite non-negative uncertainty values.
    pub fn validate(self) -> Result<()> {
        validate_optional_non_negative(self.linear_radius, "linear_radius")?;
        validate_optional_non_negative(self.angular_radians, "angular_radians")
    }
}

/// Opaque reference to an entity owned by another domain model.
///
/// Examples include a COLMAP camera/image/point id or a Gaussian-splat scene/object id. Keeping
/// these references opaque prevents this core module from taking ownership of those formats.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpatialEntityRef {
    /// Owning namespace or adapter, for example `colmap`.
    pub namespace: String,
    /// Domain entity kind, for example `camera`, `image`, or `point3d`.
    pub entity_kind: String,
    /// Stable id inside that namespace.
    pub entity_id: String,
}

impl SpatialEntityRef {
    /// Creates a validated domain entity reference.
    pub fn new(
        namespace: impl Into<String>,
        entity_kind: impl Into<String>,
        entity_id: impl Into<String>,
    ) -> Result<Self> {
        let value = Self {
            namespace: namespace.into(),
            entity_kind: entity_kind.into(),
            entity_id: entity_id.into(),
        };
        value.validate()?;
        Ok(value)
    }

    /// Validates reference components.
    pub fn validate(&self) -> Result<()> {
        validate_non_empty(&self.namespace, "spatial entity namespace")?;
        validate_non_empty(&self.entity_kind, "spatial entity kind")?;
        validate_non_empty(&self.entity_id, "spatial entity id")
    }
}

/// Typed spatial location selected by an annotation or correspondence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SpatialSelector {
    /// A single point in a Cartesian 3D frame.
    Point3 {
        /// Coordinate frame containing the point.
        frame: CoordinateFrameRef,
        /// Point coordinates.
        point: Point3d,
        /// Optional localization uncertainty.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        uncertainty: Option<SpatialUncertainty>,
    },
    /// Axis-aligned 3D box in a Cartesian frame.
    Box3 {
        /// Coordinate frame containing the box.
        frame: CoordinateFrameRef,
        /// Inclusive minimum corner.
        min: Point3d,
        /// Inclusive maximum corner.
        max: Point3d,
    },
    /// Sphere in a Cartesian 3D frame.
    Sphere3 {
        /// Coordinate frame containing the sphere.
        frame: CoordinateFrameRef,
        /// Sphere center.
        center: Point3d,
        /// Positive radius in frame units.
        radius: f64,
    },
    /// Generic rigid pose whose transform maps local object coordinates into `frame`.
    Pose3 {
        /// Parent coordinate frame.
        frame: CoordinateFrameRef,
        /// Object-to-frame rigid transform with quaternion rotation.
        pose: RigidTransform3d,
        /// Optional pose uncertainty.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        uncertainty: Option<SpatialUncertainty>,
    },
    /// Camera position/orientation in a Cartesian scene/reconstruction frame.
    CameraPose {
        /// Parent scene/reconstruction frame.
        frame: CoordinateFrameRef,
        /// Double-precision camera pose using the canonical workspace camera convention.
        pose: CameraPose3d,
        /// Optional pinhole calibration when representable by the canonical core type.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        intrinsics: Option<PinholeIntrinsicsd>,
        /// Optional reference to richer calibration owned by another domain model.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        calibration_ref: Option<SpatialEntityRef>,
        /// Optional camera-pose uncertainty.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        uncertainty: Option<SpatialUncertainty>,
    },
    /// WGS84 geographic point.
    GeographicPoint {
        /// Geographic position.
        position: GeographicPosition,
        /// Optional horizontal accuracy in meters.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        horizontal_accuracy_meters: Option<f64>,
        /// Optional vertical accuracy in meters.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        vertical_accuracy_meters: Option<f64>,
    },
    /// Reference to a domain-owned spatial entity without copying its geometry.
    Entity {
        /// Referenced entity.
        entity: SpatialEntityRef,
    },
}

impl SpatialSelector {
    /// Validates geometry, pose, reference, and uncertainty invariants.
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Point3 {
                frame,
                point,
                uncertainty,
            } => {
                frame.validate()?;
                if !point.is_finite() {
                    return Err(invalid_argument("spatial point coordinates must be finite"));
                }
                if let Some(uncertainty) = uncertainty {
                    uncertainty.validate()?;
                }
            }
            Self::Box3 { frame, min, max } => {
                frame.validate()?;
                if !min.is_finite() || !max.is_finite() {
                    return Err(invalid_argument("spatial box coordinates must be finite"));
                }
                if min.x > max.x || min.y > max.y || min.z > max.z {
                    return Err(invalid_argument(
                        "spatial box min coordinates must not exceed max coordinates",
                    ));
                }
            }
            Self::Sphere3 {
                frame,
                center,
                radius,
            } => {
                frame.validate()?;
                if !center.is_finite() {
                    return Err(invalid_argument("spatial sphere center must be finite"));
                }
                if !radius.is_finite() || *radius <= 0.0 {
                    return Err(invalid_argument(
                        "spatial sphere radius must be finite and greater than zero",
                    ));
                }
            }
            Self::Pose3 {
                frame,
                pose,
                uncertainty,
            } => {
                frame.validate()?;
                RigidTransform3d::new(pose.rotation, pose.translation)?;
                if let Some(uncertainty) = uncertainty {
                    uncertainty.validate()?;
                }
            }
            Self::CameraPose {
                frame,
                pose,
                intrinsics,
                calibration_ref,
                uncertainty,
            } => {
                frame.validate()?;
                pose.validate()?;
                if let Some(intrinsics) = intrinsics {
                    intrinsics.validate()?;
                }
                if let Some(calibration_ref) = calibration_ref {
                    calibration_ref.validate()?;
                }
                if let Some(uncertainty) = uncertainty {
                    uncertainty.validate()?;
                }
            }
            Self::GeographicPoint {
                position,
                horizontal_accuracy_meters,
                vertical_accuracy_meters,
            } => {
                position.validate()?;
                validate_optional_non_negative(
                    *horizontal_accuracy_meters,
                    "horizontal_accuracy_meters",
                )?;
                validate_optional_non_negative(
                    *vertical_accuracy_meters,
                    "vertical_accuracy_meters",
                )?;
            }
            Self::Entity { entity } => entity.validate()?,
        }
        Ok(())
    }
}

/// Versioned correspondence between a spatial selection and an optional selector owned elsewhere.
///
/// The source selector is serialized as JSON on purpose. A consumer that already depends on a
/// source capability can insert and recover that capability's real selector type through
/// [`SpatialBinding::with_source_selector`] and [`SpatialBinding::source_selector_as`]. For
/// example, an integration crate can use the neutral media annotation selector for a video frame
/// or image region without forcing `three-d-processing-core` to depend on `media-core`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpatialBinding {
    /// Serialized contract version.
    pub schema_version: u32,
    /// Typed spatial selection.
    pub spatial: SpatialSelector,
    /// Optional selector from another capability, serialized without taking ownership of its type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_selector: Option<Value>,
}

impl SpatialBinding {
    /// Creates a spatial binding without a source-capability selector.
    pub fn new(spatial: SpatialSelector) -> Result<Self> {
        let value = Self {
            schema_version: SPATIAL_ANNOTATION_SCHEMA_VERSION,
            spatial,
            source_selector: None,
        };
        value.validate()?;
        Ok(value)
    }

    /// Serializes and attaches a selector owned by another capability.
    pub fn with_source_selector<T: Serialize>(mut self, selector: T) -> Result<Self> {
        self.source_selector = Some(serde_json::to_value(selector).map_err(|error| {
            invalid_argument(format!("could not serialize spatial source selector: {error}"))
        })?);
        self.validate()?;
        Ok(self)
    }

    /// Recovers the source selector as a consumer-owned type.
    pub fn source_selector_as<T: DeserializeOwned>(&self) -> Result<Option<T>> {
        self.source_selector
            .as_ref()
            .map(|selector| {
                serde_json::from_value(selector.clone()).map_err(|error| {
                    invalid_argument(format!(
                        "could not deserialize spatial source selector: {error}"
                    ))
                })
            })
            .transpose()
    }

    /// Validates schema and spatial invariants.
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != SPATIAL_ANNOTATION_SCHEMA_VERSION {
            return Err(invalid_argument(format!(
                "unsupported spatial annotation schema version {}",
                self.schema_version
            )));
        }
        self.spatial.validate()
    }
}

fn validate_non_empty(value: &str, field: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(invalid_argument(format!("{field} must not be empty")))
    } else {
        Ok(())
    }
}

fn validate_optional_non_negative(value: Option<f64>, field: &str) -> Result<()> {
    if value.is_some_and(|value| !value.is_finite() || value < 0.0) {
        Err(invalid_argument(format!(
            "{field} must be finite and non-negative when present"
        )))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(tag = "kind", rename_all = "snake_case")]
    enum ExampleMediaSelector {
        Frame {
            #[serde(rename = "frameIndex")]
            frame_index: u64,
        },
        Region2d {
            x: u32,
            y: u32,
            width: u32,
            height: u32,
        },
    }

    fn scene_frame() -> CoordinateFrameRef {
        CoordinateFrameRef::local("colmap-world")
            .unwrap()
            .unit(CoordinateUnit::Arbitrary)
            .axis_convention("workspace-right-handed-y-up-z-forward")
            .unwrap()
    }

    #[test]
    fn validates_wgs84_positions_and_geographic_scene_anchors() {
        let origin = GeographicPosition::new(8.682_127, 50.110_924, Some(112.5)).unwrap();
        let anchor = GeographicFrameAnchor::new(
            scene_frame(),
            origin,
            GeographicTangentFrame::EastNorthUp,
            Quaterniond::IDENTITY,
            0.025,
        )
        .unwrap();

        anchor.validate().unwrap();
        assert!(GeographicPosition::new(181.0, 0.0, None).is_err());
        assert!(GeographicPosition::new(0.0, -91.0, None).is_err());
    }

    #[test]
    fn similarity_frame_transform_preserves_reconstruction_scale() {
        let transform = CoordinateFrameTransform3d::new(
            scene_frame(),
            CoordinateFrameRef::local("metric-world")
                .unwrap()
                .unit(CoordinateUnit::Meter),
            SimilarityTransform3d::new(
                crate::Vector3d::new(1.0, 2.0, 3.0),
                Quaterniond::IDENTITY,
                0.001,
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(transform.transform.scale, 0.001);
    }

    #[test]
    fn colmap_camera_pose_binds_losslessly_to_a_video_frame_selector() {
        let pose = CameraPose3d::from_colmap_world_to_camera(
            1.0, 0.0, 0.0, 0.0, 1.25, -2.5, 3.75,
        )
        .unwrap();
        let selector = SpatialSelector::CameraPose {
            frame: scene_frame(),
            pose,
            intrinsics: Some(
                PinholeIntrinsicsd::new(1920, 1080, 1400.5, 1401.25, 959.75, 539.25).unwrap(),
            ),
            calibration_ref: Some(SpatialEntityRef::new("colmap", "camera", "7").unwrap()),
            uncertainty: Some(SpatialUncertainty {
                linear_radius: Some(0.002),
                angular_radians: Some(0.000_5),
            }),
        };
        let source_selector = ExampleMediaSelector::Frame { frame_index: 42 };
        let binding = SpatialBinding::new(selector.clone())
            .unwrap()
            .with_source_selector(source_selector.clone())
            .unwrap();

        assert_eq!(binding.spatial, selector);
        assert_eq!(
            binding
                .source_selector_as::<ExampleMediaSelector>()
                .unwrap(),
            Some(source_selector)
        );

        let encoded = serde_json::to_string(&binding).unwrap();
        let decoded: SpatialBinding = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, binding);
    }

    #[test]
    fn image_region_can_correspond_to_a_scene_point_without_duplicating_public_region_types() {
        let source_selector = ExampleMediaSelector::Region2d {
            x: 120,
            y: 80,
            width: 24,
            height: 30,
        };
        let binding = SpatialBinding::new(SpatialSelector::Point3 {
            frame: scene_frame(),
            point: Point3d::new(1.25, -0.5, 4.75),
            uncertainty: None,
        })
        .unwrap()
        .with_source_selector(source_selector.clone())
        .unwrap();

        assert!(matches!(binding.spatial, SpatialSelector::Point3 { .. }));
        assert_eq!(
            binding
                .source_selector_as::<ExampleMediaSelector>()
                .unwrap(),
            Some(source_selector)
        );
    }

    #[test]
    fn generic_pose_keeps_quaternion_rotation_and_translation() {
        let pose = RigidTransform3d::new(
            Quaterniond::new(
                0.0,
                0.0,
                std::f64::consts::FRAC_1_SQRT_2,
                std::f64::consts::FRAC_1_SQRT_2,
            ),
            crate::Vector3d::new(1.0, 2.0, 3.0),
        )
        .unwrap();
        let selector = SpatialSelector::Pose3 {
            frame: scene_frame(),
            pose,
            uncertainty: None,
        };

        selector.validate().unwrap();
        let encoded = serde_json::to_string(&selector).unwrap();
        let decoded: SpatialSelector = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, selector);
    }

    #[test]
    fn spatial_binding_rejects_unknown_schema_versions() {
        let mut binding = SpatialBinding::new(SpatialSelector::GeographicPoint {
            position: GeographicPosition::new(8.0, 49.0, None).unwrap(),
            horizontal_accuracy_meters: Some(2.0),
            vertical_accuracy_meters: None,
        })
        .unwrap();
        binding.schema_version += 1;

        assert!(binding.validate().is_err());
    }
}
