use serde::{Deserialize, Serialize};
use video_analysis_core::Result;

use crate::{invalid_argument, validate_finite_vector, Point3, Vector3};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
/// Data type for transform3.
pub struct Transform3 {
    /// The translation value.
    pub translation: Vector3,
    /// The scale value.
    pub scale: f32,
}

impl Transform3 {
    /// Constant for identity.
    pub const IDENTITY: Self = Self {
        translation: Vector3::ZERO,
        scale: 1.0,
    };

    /// Creates a new value.
    pub fn new(translation: Vector3, scale: f32) -> Result<Self> {
        if !translation.is_finite() || !scale.is_finite() || scale == 0.0 {
            return Err(invalid_argument(
                "transform translation must be finite and scale must be finite and non-zero",
            ));
        }
        Ok(Self { translation, scale })
    }

    /// Creates a translation transform.
    pub fn translation(translation: Vector3) -> Self {
        Self {
            translation,
            scale: 1.0,
        }
    }

    /// Creates a uniform scaling transform.
    pub fn scaling(scale: f32) -> Result<Self> {
        Self::new(Vector3::ZERO, scale)
    }

    /// Returns apply point.
    pub fn apply_point(self, point: Point3) -> Point3 {
        Point3::new(
            point.x * self.scale + self.translation.x,
            point.y * self.scale + self.translation.y,
            point.z * self.scale + self.translation.z,
        )
    }

    /// Returns apply vector.
    pub fn apply_vector(self, vector: Vector3) -> Vector3 {
        vector * self.scale
    }

    /// Returns inverse.
    pub fn inverse(self) -> Result<Self> {
        if !self.translation.is_finite() || !self.scale.is_finite() || self.scale == 0.0 {
            return Err(invalid_argument(
                "transform translation must be finite and scale must be finite and non-zero",
            ));
        }
        Self::new(self.translation * (-1.0 / self.scale), 1.0 / self.scale)
    }

    /// Returns compose.
    pub fn compose(self, next: Self) -> Result<Self> {
        Self::new(
            next.apply_vector(self.translation) + next.translation,
            self.scale * next.scale,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
/// Data type for quaternion.
pub struct Quaternion {
    /// The x value.
    pub x: f32,
    /// The y value.
    pub y: f32,
    /// The z value.
    pub z: f32,
    /// The w value.
    pub w: f32,
}

impl Quaternion {
    /// Constant for identity.
    pub const IDENTITY: Self = Self::new(0.0, 0.0, 0.0, 1.0);

    /// Creates a new value.
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    /// Returns whether this value is finite.
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite() && self.w.is_finite()
    }

    /// Builds this value from axis angle.
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

    /// Returns dot.
    pub fn dot(self, rhs: Self) -> f32 {
        self.x.mul_add(
            rhs.x,
            self.y.mul_add(rhs.y, self.z.mul_add(rhs.z, self.w * rhs.w)),
        )
    }

    /// Returns norm.
    pub fn norm(self) -> f32 {
        self.dot(self).sqrt()
    }

    /// Normalizes this value.
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

    /// Returns conjugate.
    pub fn conjugate(self) -> Self {
        Self::new(-self.x, -self.y, -self.z, self.w)
    }

    /// Returns rotate vector.
    pub fn rotate_vector(self, vector: Vector3) -> Result<Vector3> {
        let q = self.normalize()?;
        validate_finite_vector(vector, "vector")?;
        let u = Vector3::new(q.x, q.y, q.z);
        let uv = u.cross(vector);
        let uuv = u.cross(uv);
        Ok(vector + ((2.0 * q.w) * uv) + (2.0 * uuv))
    }

    /// Returns mul quaternion.
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

    /// Returns normalized linear interpolation.
    pub fn nlerp(self, rhs: Self, t: f32) -> Result<Self> {
        if !t.is_finite() {
            return Err(invalid_argument("interpolation factor must be finite"));
        }
        let lhs = self.normalize()?;
        let mut rhs = rhs.normalize()?;
        if lhs.dot(rhs) < 0.0 {
            rhs = Self::new(-rhs.x, -rhs.y, -rhs.z, -rhs.w);
        }
        Self::new(
            lhs.x + (rhs.x - lhs.x) * t,
            lhs.y + (rhs.y - lhs.y) * t,
            lhs.z + (rhs.z - lhs.z) * t,
            lhs.w + (rhs.w - lhs.w) * t,
        )
        .normalize()
    }

    /// Returns spherical linear interpolation.
    pub fn slerp(self, rhs: Self, t: f32) -> Result<Self> {
        if !t.is_finite() {
            return Err(invalid_argument("interpolation factor must be finite"));
        }
        let lhs = self.normalize()?;
        let mut rhs = rhs.normalize()?;
        let mut dot = lhs.dot(rhs);
        if dot < 0.0 {
            rhs = Self::new(-rhs.x, -rhs.y, -rhs.z, -rhs.w);
            dot = -dot;
        }
        if dot > 0.9995 {
            return lhs.nlerp(rhs, t);
        }
        let theta_0 = dot.clamp(-1.0, 1.0).acos();
        let theta = theta_0 * t;
        let sin_theta = theta.sin();
        let sin_theta_0 = theta_0.sin();
        let s0 = theta.cos() - dot * sin_theta / sin_theta_0;
        let s1 = sin_theta / sin_theta_0;
        Self::new(
            lhs.x * s0 + rhs.x * s1,
            lhs.y * s0 + rhs.y * s1,
            lhs.z * s0 + rhs.z * s1,
            lhs.w * s0 + rhs.w * s1,
        )
        .normalize()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
/// Data type for rigid transform3.
pub struct RigidTransform3 {
    /// The rotation value.
    pub rotation: Quaternion,
    /// The translation value.
    pub translation: Vector3,
}

impl RigidTransform3 {
    /// Constant for identity.
    pub const IDENTITY: Self = Self {
        rotation: Quaternion::IDENTITY,
        translation: Vector3::ZERO,
    };

    /// Creates a new value.
    pub fn new(rotation: Quaternion, translation: Vector3) -> Result<Self> {
        let rotation = rotation.normalize()?;
        validate_finite_vector(translation, "translation")?;
        Ok(Self {
            rotation,
            translation,
        })
    }

    /// Returns apply point.
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

    /// Returns apply vector.
    pub fn apply_vector(self, vector: Vector3) -> Result<Vector3> {
        self.rotation.rotate_vector(vector)
    }

    /// Returns inverse.
    pub fn inverse(self) -> Result<Self> {
        let rotation = self.rotation.normalize()?.conjugate();
        let translation = rotation.rotate_vector(self.translation * -1.0)?;
        Self::new(rotation, translation)
    }

    /// Returns compose.
    pub fn compose(self, rhs: Self) -> Result<Self> {
        let rotation = self.rotation.mul_quaternion(rhs.rotation)?;
        let translation = self.apply_vector(rhs.translation)? + self.translation;
        Self::new(rotation, translation)
    }
}
