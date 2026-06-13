use serde::{Deserialize, Serialize};
use video_analysis_core::Result;

use crate::{invalid_argument, Point3, Quaternion, Vector3};

const ORTHONORMAL_EPSILON_F32: f32 = 1.0e-3;
const ORTHONORMAL_EPSILON_F64: f64 = 1.0e-9;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
/// Explicit Euler rotation order.
pub enum EulerOrder {
    /// Apply X, then Y, then Z rotations.
    Xyz,
    /// Apply X, then Z, then Y rotations.
    Xzy,
    /// Apply Y, then X, then Z rotations.
    Yxz,
    /// Apply Y, then Z, then X rotations.
    Yzx,
    /// Apply Z, then X, then Y rotations.
    Zxy,
    /// Apply Z, then Y, then X rotations.
    Zyx,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
/// Double-precision 3D vector.
pub struct Vector3d {
    /// The x value.
    pub x: f64,
    /// The y value.
    pub y: f64,
    /// The z value.
    pub z: f64,
}

impl Vector3d {
    /// Constant for zero.
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0);
    /// Constant for x.
    pub const X: Self = Self::new(1.0, 0.0, 0.0);
    /// Constant for y.
    pub const Y: Self = Self::new(0.0, 1.0, 0.0);
    /// Constant for z.
    pub const Z: Self = Self::new(0.0, 0.0, 1.0);

    /// Creates a new vector.
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// Returns whether all components are finite.
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }

    /// Returns the dot product.
    pub fn dot(self, rhs: Self) -> f64 {
        self.x.mul_add(rhs.x, self.y.mul_add(rhs.y, self.z * rhs.z))
    }

    /// Returns the cross product.
    pub fn cross(self, rhs: Self) -> Self {
        Self::new(
            self.y.mul_add(rhs.z, -(self.z * rhs.y)),
            self.z.mul_add(rhs.x, -(self.x * rhs.z)),
            self.x.mul_add(rhs.y, -(self.y * rhs.x)),
        )
    }

    /// Returns the squared length.
    pub fn length_squared(self) -> f64 {
        self.dot(self)
    }

    /// Returns the length.
    pub fn length(self) -> f64 {
        self.length_squared().sqrt()
    }

    /// Returns the normalized vector.
    pub fn normalize(self) -> Result<Self> {
        validate_vector3d(self, "vector")?;
        let length = self.length();
        if length <= f64::EPSILON {
            return Err(invalid_argument("vector length must be greater than zero"));
        }
        Ok(self / length)
    }

    /// Converts this value to the single-precision vector type.
    pub fn to_f32_checked(self) -> Result<Vector3> {
        Ok(Vector3::new(
            f64_to_f32(self.x, "x")?,
            f64_to_f32(self.y, "y")?,
            f64_to_f32(self.z, "z")?,
        ))
    }

    /// Returns this value as an array.
    pub fn to_array(self) -> [f64; 3] {
        [self.x, self.y, self.z]
    }
}

impl std::ops::Add for Vector3d {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl std::ops::Sub for Vector3d {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl std::ops::Mul<f64> for Vector3d {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

impl std::ops::Mul<Vector3d> for f64 {
    type Output = Vector3d;

    fn mul(self, rhs: Vector3d) -> Self::Output {
        rhs * self
    }
}

impl std::ops::Div<f64> for Vector3d {
    type Output = Self;

    fn div(self, rhs: f64) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs, self.z / rhs)
    }
}

impl From<Vector3> for Vector3d {
    fn from(value: Vector3) -> Self {
        Self::new(value.x as f64, value.y as f64, value.z as f64)
    }
}

impl From<[f64; 3]> for Vector3d {
    fn from(value: [f64; 3]) -> Self {
        Self::new(value[0], value[1], value[2])
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
/// Double-precision 3D point.
pub struct Point3d {
    /// The x value.
    pub x: f64,
    /// The y value.
    pub y: f64,
    /// The z value.
    pub z: f64,
}

impl Point3d {
    /// Creates a new point.
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// Returns whether all components are finite.
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }

    /// Converts this value to the single-precision point type.
    pub fn to_f32_checked(self) -> Result<Point3> {
        Ok(Point3::new(
            f64_to_f32(self.x, "x")?,
            f64_to_f32(self.y, "y")?,
            f64_to_f32(self.z, "z")?,
        ))
    }

    /// Returns this value as an array.
    pub fn to_array(self) -> [f64; 3] {
        [self.x, self.y, self.z]
    }
}

impl std::ops::Add<Vector3d> for Point3d {
    type Output = Self;

    fn add(self, rhs: Vector3d) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl std::ops::Sub<Vector3d> for Point3d {
    type Output = Self;

    fn sub(self, rhs: Vector3d) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl std::ops::Sub<Point3d> for Point3d {
    type Output = Vector3d;

    fn sub(self, rhs: Point3d) -> Self::Output {
        Vector3d::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl From<Point3> for Point3d {
    fn from(value: Point3) -> Self {
        Self::new(value.x as f64, value.y as f64, value.z as f64)
    }
}

impl From<[f64; 3]> for Point3d {
    fn from(value: [f64; 3]) -> Self {
        Self::new(value[0], value[1], value[2])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
/// Double-precision quaternion stored as x, y, z, w.
pub struct Quaterniond {
    /// The x value.
    pub x: f64,
    /// The y value.
    pub y: f64,
    /// The z value.
    pub z: f64,
    /// The w value.
    pub w: f64,
}

impl Quaterniond {
    /// Identity rotation.
    pub const IDENTITY: Self = Self::new(0.0, 0.0, 0.0, 1.0);

    /// Creates a quaternion.
    pub const fn new(x: f64, y: f64, z: f64, w: f64) -> Self {
        Self { x, y, z, w }
    }

    /// Returns whether all components are finite.
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite() && self.w.is_finite()
    }

    /// Builds a quaternion from an axis and angle in radians.
    pub fn from_axis_angle(axis: Vector3d, angle_radians: f64) -> Result<Self> {
        validate_vector3d(axis, "axis")?;
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

    /// Builds a quaternion from explicit Euler angles.
    pub fn from_euler(order: EulerOrder, x: f64, y: f64, z: f64) -> Result<Self> {
        for (name, value) in [("x", x), ("y", y), ("z", z)] {
            if !value.is_finite() {
                return Err(invalid_argument(format!(
                    "Euler angle {name} must be finite"
                )));
            }
        }
        let qx = Self::from_axis_angle(Vector3d::X, x)?;
        let qy = Self::from_axis_angle(Vector3d::Y, y)?;
        let qz = Self::from_axis_angle(Vector3d::Z, z)?;
        match order {
            EulerOrder::Xyz => qz.mul_quaternion(qy)?.mul_quaternion(qx),
            EulerOrder::Xzy => qy.mul_quaternion(qz)?.mul_quaternion(qx),
            EulerOrder::Yxz => qz.mul_quaternion(qx)?.mul_quaternion(qy),
            EulerOrder::Yzx => qx.mul_quaternion(qz)?.mul_quaternion(qy),
            EulerOrder::Zxy => qy.mul_quaternion(qx)?.mul_quaternion(qz),
            EulerOrder::Zyx => qx.mul_quaternion(qy)?.mul_quaternion(qz),
        }
    }

    /// Builds a quaternion from a row-major rotation matrix.
    pub fn from_rotation_matrix(matrix: Matrix3d) -> Result<Self> {
        matrix.validate_rotation()?;
        let flat = matrix.to_row_major_array();
        let matrix = nalgebra::Matrix3::<f64>::from_row_slice(&flat);
        let rotation = nalgebra::Rotation3::from_matrix_unchecked(matrix);
        let q = nalgebra::UnitQuaternion::from_rotation_matrix(&rotation);
        let q = q.quaternion();
        Self::new(q.i, q.j, q.k, q.w).normalize()
    }

    /// Returns the dot product.
    pub fn dot(self, rhs: Self) -> f64 {
        self.x.mul_add(
            rhs.x,
            self.y.mul_add(rhs.y, self.z.mul_add(rhs.z, self.w * rhs.w)),
        )
    }

    /// Returns the norm.
    pub fn norm(self) -> f64 {
        self.dot(self).sqrt()
    }

    /// Returns the normalized quaternion.
    pub fn normalize(self) -> Result<Self> {
        if !self.is_finite() {
            return Err(invalid_argument("quaternion components must be finite"));
        }
        let norm = self.norm();
        if norm <= f64::EPSILON {
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

    /// Returns the conjugate.
    pub fn conjugate(self) -> Self {
        Self::new(-self.x, -self.y, -self.z, self.w)
    }

    /// Returns the inverse rotation.
    pub fn inverse(self) -> Result<Self> {
        Ok(self.normalize()?.conjugate())
    }

    /// Multiplies two rotations.
    pub fn mul_quaternion(self, rhs: Self) -> Result<Self> {
        let lhs = self.normalize()?;
        let rhs = rhs.normalize()?;
        Self::new(
            lhs.w
                .mul_add(rhs.x, lhs.x.mul_add(rhs.w, lhs.y * rhs.z - lhs.z * rhs.y)),
            lhs.w
                .mul_add(rhs.y, -lhs.x * rhs.z + lhs.y.mul_add(rhs.w, lhs.z * rhs.x)),
            lhs.w
                .mul_add(rhs.z, lhs.x * rhs.y - lhs.y * rhs.x + lhs.z * rhs.w),
            lhs.w
                .mul_add(rhs.w, -(lhs.x * rhs.x + lhs.y * rhs.y + lhs.z * rhs.z)),
        )
        .normalize()
    }

    /// Rotates a vector.
    pub fn rotate_vector(self, vector: Vector3d) -> Result<Vector3d> {
        let q = self.normalize()?;
        validate_vector3d(vector, "vector")?;
        let u = Vector3d::new(q.x, q.y, q.z);
        let uv = u.cross(vector);
        let uuv = u.cross(uv);
        Ok(vector + ((2.0 * q.w) * uv) + (2.0 * uuv))
    }

    /// Converts this rotation to a row-major 3x3 matrix.
    pub fn to_rotation_matrix(self) -> Result<Matrix3d> {
        let q = self.normalize()?;
        let xx = q.x * q.x;
        let yy = q.y * q.y;
        let zz = q.z * q.z;
        let xy = q.x * q.y;
        let xz = q.x * q.z;
        let yz = q.y * q.z;
        let wx = q.w * q.x;
        let wy = q.w * q.y;
        let wz = q.w * q.z;

        Matrix3d::new([
            [1.0 - 2.0 * (yy + zz), 2.0 * (xy - wz), 2.0 * (xz + wy)],
            [2.0 * (xy + wz), 1.0 - 2.0 * (xx + zz), 2.0 * (yz - wx)],
            [2.0 * (xz - wy), 2.0 * (yz + wx), 1.0 - 2.0 * (xx + yy)],
        ])
    }

    /// Converts this quaternion to axis-angle form.
    pub fn to_axis_angle(self) -> Result<(Vector3d, f64)> {
        let q = self.normalize()?;
        let angle = 2.0 * q.w.clamp(-1.0, 1.0).acos();
        let sin_half = (1.0 - q.w * q.w).max(0.0).sqrt();
        if sin_half <= f64::EPSILON {
            return Ok((Vector3d::X, 0.0));
        }
        Ok((
            Vector3d::new(q.x / sin_half, q.y / sin_half, q.z / sin_half).normalize()?,
            angle,
        ))
    }

    /// Converts this quaternion to Euler angles for import/export and UI controls.
    ///
    /// Quaternions remain the primary workspace rotation representation. Euler
    /// angles are provided only for boundary formats and direct manipulation UI.
    pub fn to_euler(self, order: EulerOrder) -> Result<(f64, f64, f64)> {
        let matrix = self.to_rotation_matrix()?;
        euler_from_matrix(order, matrix.rows)
    }

    /// Returns normalized linear interpolation.
    pub fn nlerp(self, rhs: Self, t: f64) -> Result<Self> {
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
    pub fn slerp(self, rhs: Self, t: f64) -> Result<Self> {
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

    /// Converts this value to the single-precision quaternion type.
    pub fn to_f32_checked(self) -> Result<Quaternion> {
        Quaternion::new(
            f64_to_f32(self.x, "x")?,
            f64_to_f32(self.y, "y")?,
            f64_to_f32(self.z, "z")?,
            f64_to_f32(self.w, "w")?,
        )
        .normalize()
    }
}

impl Default for Quaterniond {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl From<Quaternion> for Quaterniond {
    fn from(value: Quaternion) -> Self {
        Self::new(
            value.x as f64,
            value.y as f64,
            value.z as f64,
            value.w as f64,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
/// Row-major 3x3 single-precision matrix.
pub struct Matrix3 {
    /// Matrix rows.
    pub rows: [[f32; 3]; 3],
}

impl Matrix3 {
    /// Creates a matrix after finite-value validation.
    pub fn new(rows: [[f32; 3]; 3]) -> Result<Self> {
        let matrix = Self { rows };
        matrix.validate()?;
        Ok(matrix)
    }

    /// Returns identity.
    pub const fn identity() -> Self {
        Self {
            rows: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        }
    }

    /// Builds a matrix from a flat row-major array.
    pub fn from_row_major_array(values: [f32; 9]) -> Result<Self> {
        Self::new([
            [values[0], values[1], values[2]],
            [values[3], values[4], values[5]],
            [values[6], values[7], values[8]],
        ])
    }

    /// Builds a matrix from a flat column-major array.
    pub fn from_column_major_array(values: [f32; 9]) -> Result<Self> {
        Self::new([
            [values[0], values[3], values[6]],
            [values[1], values[4], values[7]],
            [values[2], values[5], values[8]],
        ])
    }

    /// Validates finite values.
    pub fn validate(self) -> Result<()> {
        for value in self.to_row_major_array() {
            if !value.is_finite() {
                return Err(invalid_argument("matrix values must be finite"));
            }
        }
        Ok(())
    }

    /// Validates this matrix as an orthonormal rotation matrix.
    pub fn validate_rotation(self) -> Result<()> {
        self.validate()?;
        let x = Vector3::new(self.rows[0][0], self.rows[1][0], self.rows[2][0]);
        let y = Vector3::new(self.rows[0][1], self.rows[1][1], self.rows[2][1]);
        let z = Vector3::new(self.rows[0][2], self.rows[1][2], self.rows[2][2]);
        validate_orthonormal3(x, y, z)
    }

    /// Returns a flat row-major array.
    pub fn to_row_major_array(self) -> [f32; 9] {
        [
            self.rows[0][0],
            self.rows[0][1],
            self.rows[0][2],
            self.rows[1][0],
            self.rows[1][1],
            self.rows[1][2],
            self.rows[2][0],
            self.rows[2][1],
            self.rows[2][2],
        ]
    }

    /// Returns a flat column-major array for graphics API upload.
    pub fn to_column_major_array(self) -> [f32; 9] {
        [
            self.rows[0][0],
            self.rows[1][0],
            self.rows[2][0],
            self.rows[0][1],
            self.rows[1][1],
            self.rows[2][1],
            self.rows[0][2],
            self.rows[1][2],
            self.rows[2][2],
        ]
    }

    /// Returns the transposed matrix.
    pub fn transpose(self) -> Result<Self> {
        Self::new([
            [self.rows[0][0], self.rows[1][0], self.rows[2][0]],
            [self.rows[0][1], self.rows[1][1], self.rows[2][1]],
            [self.rows[0][2], self.rows[1][2], self.rows[2][2]],
        ])
    }

    /// Returns the determinant.
    pub fn determinant(self) -> Result<f32> {
        self.validate()?;
        Ok(self.rows[0][0]
            * (self.rows[1][1] * self.rows[2][2] - self.rows[1][2] * self.rows[2][1])
            - self.rows[0][1]
                * (self.rows[1][0] * self.rows[2][2] - self.rows[1][2] * self.rows[2][0])
            + self.rows[0][2]
                * (self.rows[1][0] * self.rows[2][1] - self.rows[1][1] * self.rows[2][0]))
    }

    /// Converts this value to double precision.
    pub fn to_f64(self) -> Matrix3d {
        Matrix3d {
            rows: self.rows.map(|row| row.map(f64::from)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
/// Row-major 3x3 double-precision matrix.
pub struct Matrix3d {
    /// Matrix rows.
    pub rows: [[f64; 3]; 3],
}

impl Matrix3d {
    /// Creates a matrix after finite-value validation.
    pub fn new(rows: [[f64; 3]; 3]) -> Result<Self> {
        let matrix = Self { rows };
        matrix.validate()?;
        Ok(matrix)
    }

    /// Returns identity.
    pub const fn identity() -> Self {
        Self {
            rows: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        }
    }

    /// Builds a matrix from a flat row-major array.
    pub fn from_row_major_array(values: [f64; 9]) -> Result<Self> {
        Self::new([
            [values[0], values[1], values[2]],
            [values[3], values[4], values[5]],
            [values[6], values[7], values[8]],
        ])
    }

    /// Builds a matrix from a flat column-major array.
    pub fn from_column_major_array(values: [f64; 9]) -> Result<Self> {
        Self::new([
            [values[0], values[3], values[6]],
            [values[1], values[4], values[7]],
            [values[2], values[5], values[8]],
        ])
    }

    /// Validates finite values.
    pub fn validate(self) -> Result<()> {
        for value in self.to_row_major_array() {
            if !value.is_finite() {
                return Err(invalid_argument("matrix values must be finite"));
            }
        }
        Ok(())
    }

    /// Validates this matrix as an orthonormal rotation matrix.
    pub fn validate_rotation(self) -> Result<()> {
        self.validate()?;
        let x = Vector3d::new(self.rows[0][0], self.rows[1][0], self.rows[2][0]);
        let y = Vector3d::new(self.rows[0][1], self.rows[1][1], self.rows[2][1]);
        let z = Vector3d::new(self.rows[0][2], self.rows[1][2], self.rows[2][2]);
        validate_orthonormal3d(x, y, z)
    }

    /// Returns a flat row-major array.
    pub fn to_row_major_array(self) -> [f64; 9] {
        [
            self.rows[0][0],
            self.rows[0][1],
            self.rows[0][2],
            self.rows[1][0],
            self.rows[1][1],
            self.rows[1][2],
            self.rows[2][0],
            self.rows[2][1],
            self.rows[2][2],
        ]
    }

    /// Returns a flat column-major array for graphics API upload.
    pub fn to_column_major_array(self) -> [f64; 9] {
        [
            self.rows[0][0],
            self.rows[1][0],
            self.rows[2][0],
            self.rows[0][1],
            self.rows[1][1],
            self.rows[2][1],
            self.rows[0][2],
            self.rows[1][2],
            self.rows[2][2],
        ]
    }

    /// Returns the transposed matrix.
    pub fn transpose(self) -> Result<Self> {
        Self::new([
            [self.rows[0][0], self.rows[1][0], self.rows[2][0]],
            [self.rows[0][1], self.rows[1][1], self.rows[2][1]],
            [self.rows[0][2], self.rows[1][2], self.rows[2][2]],
        ])
    }

    /// Returns the determinant.
    pub fn determinant(self) -> Result<f64> {
        self.validate()?;
        Ok(self.rows[0][0]
            * (self.rows[1][1] * self.rows[2][2] - self.rows[1][2] * self.rows[2][1])
            - self.rows[0][1]
                * (self.rows[1][0] * self.rows[2][2] - self.rows[1][2] * self.rows[2][0])
            + self.rows[0][2]
                * (self.rows[1][0] * self.rows[2][1] - self.rows[1][1] * self.rows[2][0]))
    }

    /// Converts this value to single precision.
    pub fn to_f32_checked(self) -> Result<Matrix3> {
        let mut rows = [[0.0; 3]; 3];
        for (row_index, row) in rows.iter_mut().enumerate() {
            for (col_index, cell) in row.iter_mut().enumerate() {
                *cell = f64_to_f32(self.rows[row_index][col_index], "matrix value")?;
            }
        }
        Matrix3::new(rows)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
/// Row-major 4x4 single-precision matrix with column-vector multiplication semantics.
pub struct Matrix4 {
    /// Matrix rows.
    pub rows: [[f32; 4]; 4],
}

impl Matrix4 {
    /// Creates a matrix after finite-value validation.
    pub fn new(rows: [[f32; 4]; 4]) -> Result<Self> {
        let matrix = Self { rows };
        matrix.validate()?;
        Ok(matrix)
    }

    /// Returns identity.
    pub const fn identity() -> Self {
        Self {
            rows: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    /// Builds a matrix from a flat row-major array.
    pub fn from_row_major_array(values: [f32; 16]) -> Result<Self> {
        Self::new([
            [values[0], values[1], values[2], values[3]],
            [values[4], values[5], values[6], values[7]],
            [values[8], values[9], values[10], values[11]],
            [values[12], values[13], values[14], values[15]],
        ])
    }

    /// Validates finite values.
    pub fn validate(self) -> Result<()> {
        for value in self.to_row_major_array() {
            if !value.is_finite() {
                return Err(invalid_argument("matrix values must be finite"));
            }
        }
        Ok(())
    }

    /// Returns a flat row-major array.
    pub fn to_row_major_array(self) -> [f32; 16] {
        [
            self.rows[0][0],
            self.rows[0][1],
            self.rows[0][2],
            self.rows[0][3],
            self.rows[1][0],
            self.rows[1][1],
            self.rows[1][2],
            self.rows[1][3],
            self.rows[2][0],
            self.rows[2][1],
            self.rows[2][2],
            self.rows[2][3],
            self.rows[3][0],
            self.rows[3][1],
            self.rows[3][2],
            self.rows[3][3],
        ]
    }

    /// Returns a flat column-major array for graphics API upload.
    pub fn to_column_major_array(self) -> [f32; 16] {
        [
            self.rows[0][0],
            self.rows[1][0],
            self.rows[2][0],
            self.rows[3][0],
            self.rows[0][1],
            self.rows[1][1],
            self.rows[2][1],
            self.rows[3][1],
            self.rows[0][2],
            self.rows[1][2],
            self.rows[2][2],
            self.rows[3][2],
            self.rows[0][3],
            self.rows[1][3],
            self.rows[2][3],
            self.rows[3][3],
        ]
    }

    /// Builds a matrix from a flat column-major array.
    pub fn from_column_major_array(values: [f32; 16]) -> Result<Self> {
        Self::new([
            [values[0], values[4], values[8], values[12]],
            [values[1], values[5], values[9], values[13]],
            [values[2], values[6], values[10], values[14]],
            [values[3], values[7], values[11], values[15]],
        ])
    }

    /// Returns the transposed matrix.
    pub fn transpose(self) -> Result<Self> {
        let mut rows = [[0.0; 4]; 4];
        for (row_index, row) in rows.iter_mut().enumerate() {
            for (col_index, cell) in row.iter_mut().enumerate() {
                *cell = self.rows[col_index][row_index];
            }
        }
        Self::new(rows)
    }

    /// Returns the determinant.
    pub fn determinant(self) -> Result<f32> {
        f64_to_f32(self.to_f64().determinant()?, "determinant")
    }

    /// Multiplies two matrices.
    pub fn matmul(self, rhs: Self) -> Result<Self> {
        let mut rows = [[0.0; 4]; 4];
        for (row_index, row) in rows.iter_mut().enumerate() {
            for (col_index, cell) in row.iter_mut().enumerate() {
                *cell = (0..4)
                    .map(|k| self.rows[row_index][k] * rhs.rows[k][col_index])
                    .sum();
            }
        }
        Self::new(rows)
    }

    /// Applies this matrix to a point with homogeneous divide.
    pub fn transform_point(self, point: Point3) -> Result<Point3> {
        let x = self.rows[0][0] * point.x
            + self.rows[0][1] * point.y
            + self.rows[0][2] * point.z
            + self.rows[0][3];
        let y = self.rows[1][0] * point.x
            + self.rows[1][1] * point.y
            + self.rows[1][2] * point.z
            + self.rows[1][3];
        let z = self.rows[2][0] * point.x
            + self.rows[2][1] * point.y
            + self.rows[2][2] * point.z
            + self.rows[2][3];
        let w = self.rows[3][0] * point.x
            + self.rows[3][1] * point.y
            + self.rows[3][2] * point.z
            + self.rows[3][3];
        if !w.is_finite() || w.abs() <= f32::EPSILON {
            return Err(invalid_argument(
                "homogeneous point w must be finite and non-zero",
            ));
        }
        Ok(Point3::new(x / w, y / w, z / w))
    }

    /// Applies this matrix to a direction vector.
    pub fn transform_vector(self, vector: Vector3) -> Result<Vector3> {
        Ok(Vector3::new(
            self.rows[0][0] * vector.x + self.rows[0][1] * vector.y + self.rows[0][2] * vector.z,
            self.rows[1][0] * vector.x + self.rows[1][1] * vector.y + self.rows[1][2] * vector.z,
            self.rows[2][0] * vector.x + self.rows[2][1] * vector.y + self.rows[2][2] * vector.z,
        ))
    }

    /// Returns the inverse matrix.
    pub fn inverse(self) -> Result<Self> {
        self.to_f64().inverse()?.to_f32_checked()
    }

    /// Converts this value to double precision.
    pub fn to_f64(self) -> Matrix4d {
        Matrix4d {
            rows: self.rows.map(|row| row.map(f64::from)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
/// Row-major 4x4 double-precision matrix with column-vector multiplication semantics.
pub struct Matrix4d {
    /// Matrix rows.
    pub rows: [[f64; 4]; 4],
}

impl Matrix4d {
    /// Creates a matrix after finite-value validation.
    pub fn new(rows: [[f64; 4]; 4]) -> Result<Self> {
        let matrix = Self { rows };
        matrix.validate()?;
        Ok(matrix)
    }

    /// Returns identity.
    pub const fn identity() -> Self {
        Self {
            rows: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    /// Builds a matrix from a flat row-major array.
    pub fn from_row_major_array(values: [f64; 16]) -> Result<Self> {
        Self::new([
            [values[0], values[1], values[2], values[3]],
            [values[4], values[5], values[6], values[7]],
            [values[8], values[9], values[10], values[11]],
            [values[12], values[13], values[14], values[15]],
        ])
    }

    /// Validates finite values.
    pub fn validate(self) -> Result<()> {
        for value in self.to_row_major_array() {
            if !value.is_finite() {
                return Err(invalid_argument("matrix values must be finite"));
            }
        }
        Ok(())
    }

    /// Returns a flat row-major array.
    pub fn to_row_major_array(self) -> [f64; 16] {
        [
            self.rows[0][0],
            self.rows[0][1],
            self.rows[0][2],
            self.rows[0][3],
            self.rows[1][0],
            self.rows[1][1],
            self.rows[1][2],
            self.rows[1][3],
            self.rows[2][0],
            self.rows[2][1],
            self.rows[2][2],
            self.rows[2][3],
            self.rows[3][0],
            self.rows[3][1],
            self.rows[3][2],
            self.rows[3][3],
        ]
    }

    /// Returns a flat column-major array for graphics API upload.
    pub fn to_column_major_array(self) -> [f64; 16] {
        [
            self.rows[0][0],
            self.rows[1][0],
            self.rows[2][0],
            self.rows[3][0],
            self.rows[0][1],
            self.rows[1][1],
            self.rows[2][1],
            self.rows[3][1],
            self.rows[0][2],
            self.rows[1][2],
            self.rows[2][2],
            self.rows[3][2],
            self.rows[0][3],
            self.rows[1][3],
            self.rows[2][3],
            self.rows[3][3],
        ]
    }

    /// Builds a matrix from a flat column-major array.
    pub fn from_column_major_array(values: [f64; 16]) -> Result<Self> {
        Self::new([
            [values[0], values[4], values[8], values[12]],
            [values[1], values[5], values[9], values[13]],
            [values[2], values[6], values[10], values[14]],
            [values[3], values[7], values[11], values[15]],
        ])
    }

    /// Returns the transposed matrix.
    pub fn transpose(self) -> Result<Self> {
        let mut rows = [[0.0; 4]; 4];
        for (row_index, row) in rows.iter_mut().enumerate() {
            for (col_index, cell) in row.iter_mut().enumerate() {
                *cell = self.rows[col_index][row_index];
            }
        }
        Self::new(rows)
    }

    /// Returns the determinant.
    pub fn determinant(self) -> Result<f64> {
        self.validate()?;
        let matrix = nalgebra::Matrix4::<f64>::from_row_slice(&self.to_row_major_array());
        Ok(matrix.determinant())
    }

    /// Multiplies two matrices.
    pub fn matmul(self, rhs: Self) -> Result<Self> {
        let mut rows = [[0.0; 4]; 4];
        for (row_index, row) in rows.iter_mut().enumerate() {
            for (col_index, cell) in row.iter_mut().enumerate() {
                *cell = (0..4)
                    .map(|k| self.rows[row_index][k] * rhs.rows[k][col_index])
                    .sum();
            }
        }
        Self::new(rows)
    }

    /// Applies this matrix to a point with homogeneous divide.
    pub fn transform_point(self, point: Point3d) -> Result<Point3d> {
        let x = self.rows[0][0] * point.x
            + self.rows[0][1] * point.y
            + self.rows[0][2] * point.z
            + self.rows[0][3];
        let y = self.rows[1][0] * point.x
            + self.rows[1][1] * point.y
            + self.rows[1][2] * point.z
            + self.rows[1][3];
        let z = self.rows[2][0] * point.x
            + self.rows[2][1] * point.y
            + self.rows[2][2] * point.z
            + self.rows[2][3];
        let w = self.rows[3][0] * point.x
            + self.rows[3][1] * point.y
            + self.rows[3][2] * point.z
            + self.rows[3][3];
        if !w.is_finite() || w.abs() <= f64::EPSILON {
            return Err(invalid_argument(
                "homogeneous point w must be finite and non-zero",
            ));
        }
        Ok(Point3d::new(x / w, y / w, z / w))
    }

    /// Applies this matrix to a direction vector.
    pub fn transform_vector(self, vector: Vector3d) -> Result<Vector3d> {
        Ok(Vector3d::new(
            self.rows[0][0] * vector.x + self.rows[0][1] * vector.y + self.rows[0][2] * vector.z,
            self.rows[1][0] * vector.x + self.rows[1][1] * vector.y + self.rows[1][2] * vector.z,
            self.rows[2][0] * vector.x + self.rows[2][1] * vector.y + self.rows[2][2] * vector.z,
        ))
    }

    /// Returns the inverse matrix.
    pub fn inverse(self) -> Result<Self> {
        let matrix = nalgebra::Matrix4::<f64>::from_row_slice(&self.to_row_major_array());
        let inverse = matrix
            .try_inverse()
            .ok_or_else(|| invalid_argument("matrix must be invertible"))?;
        let mut rows = [[0.0; 4]; 4];
        for row in 0..4 {
            for col in 0..4 {
                rows[row][col] = inverse[(row, col)];
            }
        }
        Self::new(rows)
    }

    /// Converts this value to single precision.
    pub fn to_f32_checked(self) -> Result<Matrix4> {
        let mut rows = [[0.0; 4]; 4];
        for (row_index, row) in rows.iter_mut().enumerate() {
            for (col_index, cell) in row.iter_mut().enumerate() {
                *cell = f64_to_f32(self.rows[row_index][col_index], "matrix value")?;
            }
        }
        Matrix4::new(rows)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
/// Rigid transform with rotation and translation.
pub struct RigidTransform3d {
    /// Rotation.
    pub rotation: Quaterniond,
    /// Translation.
    pub translation: Vector3d,
}

impl RigidTransform3d {
    /// Identity transform.
    pub const IDENTITY: Self = Self {
        rotation: Quaterniond::IDENTITY,
        translation: Vector3d::ZERO,
    };

    /// Creates a rigid transform.
    pub fn new(rotation: Quaterniond, translation: Vector3d) -> Result<Self> {
        validate_vector3d(translation, "translation")?;
        Ok(Self {
            rotation: rotation.normalize()?,
            translation,
        })
    }

    /// Applies this transform to a point.
    pub fn apply_point(self, point: Point3d) -> Result<Point3d> {
        let rotated = self
            .rotation
            .rotate_vector(point - Point3d::new(0.0, 0.0, 0.0))?;
        Ok(Point3d::new(
            rotated.x + self.translation.x,
            rotated.y + self.translation.y,
            rotated.z + self.translation.z,
        ))
    }

    /// Applies this transform to a vector.
    pub fn apply_vector(self, vector: Vector3d) -> Result<Vector3d> {
        self.rotation.rotate_vector(vector)
    }

    /// Returns the inverse transform.
    pub fn inverse(self) -> Result<Self> {
        let rotation = self.rotation.inverse()?;
        let translation = rotation.rotate_vector(self.translation * -1.0)?;
        Self::new(rotation, translation)
    }

    /// Returns an affine matrix representation.
    pub fn to_affine(self) -> Result<AffineTransform3d> {
        SimilarityTransform3d::new(self.translation, self.rotation, 1.0)?.to_affine()
    }

    /// Returns a row-major 4x4 matrix representation.
    pub fn to_matrix4(self) -> Result<Matrix4d> {
        Ok(self.to_affine()?.matrix)
    }

    /// Converts this value to single precision.
    pub fn to_f32_checked(self) -> Result<crate::RigidTransform3> {
        crate::RigidTransform3::new(
            self.rotation.to_f32_checked()?,
            self.translation.to_f32_checked()?,
        )
    }
}

impl From<crate::RigidTransform3> for RigidTransform3d {
    fn from(value: crate::RigidTransform3) -> Self {
        Self {
            rotation: value.rotation.into(),
            translation: value.translation.into(),
        }
    }
}

impl crate::RigidTransform3 {
    /// Converts this value to double precision.
    pub fn to_f64(self) -> RigidTransform3d {
        self.into()
    }

    /// Returns an affine matrix representation.
    pub fn to_affine(self) -> Result<AffineTransform3> {
        self.to_f64().to_affine()?.to_f32_checked()
    }

    /// Returns a row-major 4x4 matrix representation.
    pub fn to_matrix4(self) -> Result<Matrix4> {
        Ok(self.to_affine()?.matrix)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
/// Single-precision similarity transform with rotation, translation, and uniform scale.
pub struct SimilarityTransform3 {
    /// Translation.
    pub translation: Vector3,
    /// Rotation.
    pub rotation: Quaternion,
    /// Uniform scale.
    pub scale: f32,
}

impl SimilarityTransform3 {
    /// Identity transform.
    pub const IDENTITY: Self = Self {
        translation: Vector3::ZERO,
        rotation: Quaternion::IDENTITY,
        scale: 1.0,
    };

    /// Creates a similarity transform.
    pub fn new(translation: Vector3, rotation: Quaternion, scale: f32) -> Result<Self> {
        SimilarityTransform3d::new(translation.into(), rotation.into(), scale as f64)?;
        Ok(Self {
            translation,
            rotation: rotation.normalize()?,
            scale,
        })
    }

    /// Converts this transform to affine form.
    pub fn to_affine(self) -> Result<AffineTransform3> {
        self.to_f64()?.to_affine()?.to_f32_checked()
    }

    /// Converts this value to double precision.
    pub fn to_f64(self) -> Result<SimilarityTransform3d> {
        SimilarityTransform3d::new(
            self.translation.into(),
            self.rotation.into(),
            self.scale as f64,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
/// Similarity transform with rotation, translation, and uniform scale.
pub struct SimilarityTransform3d {
    /// Translation.
    pub translation: Vector3d,
    /// Rotation.
    pub rotation: Quaterniond,
    /// Uniform scale.
    pub scale: f64,
}

impl SimilarityTransform3d {
    /// Identity transform.
    pub const IDENTITY: Self = Self {
        translation: Vector3d::ZERO,
        rotation: Quaterniond::IDENTITY,
        scale: 1.0,
    };

    /// Creates a similarity transform.
    pub fn new(translation: Vector3d, rotation: Quaterniond, scale: f64) -> Result<Self> {
        validate_vector3d(translation, "translation")?;
        if !scale.is_finite() || scale == 0.0 {
            return Err(invalid_argument("scale must be finite and non-zero"));
        }
        Ok(Self {
            translation,
            rotation: rotation.normalize()?,
            scale,
        })
    }

    /// Converts this transform to affine form.
    pub fn to_affine(self) -> Result<AffineTransform3d> {
        TrsTransform3d::new(
            self.translation,
            self.rotation,
            Vector3d::new(self.scale, self.scale, self.scale),
        )?
        .to_affine()
    }

    /// Converts this value to single precision.
    pub fn to_f32_checked(self) -> Result<SimilarityTransform3> {
        SimilarityTransform3::new(
            self.translation.to_f32_checked()?,
            self.rotation.to_f32_checked()?,
            f64_to_f32(self.scale, "scale")?,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
/// Translation, rotation, and non-uniform scale transform.
pub struct TrsTransform3d {
    /// Translation.
    pub translation: Vector3d,
    /// Rotation.
    pub rotation: Quaterniond,
    /// Non-uniform scale.
    pub scale: Vector3d,
}

impl TrsTransform3d {
    /// Identity transform.
    pub const IDENTITY: Self = Self {
        translation: Vector3d::ZERO,
        rotation: Quaterniond::IDENTITY,
        scale: Vector3d::new(1.0, 1.0, 1.0),
    };

    /// Creates a TRS transform.
    pub fn new(translation: Vector3d, rotation: Quaterniond, scale: Vector3d) -> Result<Self> {
        validate_vector3d(translation, "translation")?;
        validate_vector3d(scale, "scale")?;
        if scale.x == 0.0 || scale.y == 0.0 || scale.z == 0.0 {
            return Err(invalid_argument("scale components must be non-zero"));
        }
        Ok(Self {
            translation,
            rotation: rotation.normalize()?,
            scale,
        })
    }

    /// Applies this transform to a point.
    pub fn apply_point(self, point: Point3d) -> Result<Point3d> {
        self.to_affine()?.apply_point(point)
    }

    /// Applies this transform to a vector.
    pub fn apply_vector(self, vector: Vector3d) -> Result<Vector3d> {
        self.to_affine()?.apply_vector(vector)
    }

    /// Converts this transform to affine form.
    pub fn to_affine(self) -> Result<AffineTransform3d> {
        let rotation = self.rotation.to_rotation_matrix()?;
        AffineTransform3d::from_matrix(Matrix4d::new([
            [
                rotation.rows[0][0] * self.scale.x,
                rotation.rows[0][1] * self.scale.y,
                rotation.rows[0][2] * self.scale.z,
                self.translation.x,
            ],
            [
                rotation.rows[1][0] * self.scale.x,
                rotation.rows[1][1] * self.scale.y,
                rotation.rows[1][2] * self.scale.z,
                self.translation.y,
            ],
            [
                rotation.rows[2][0] * self.scale.x,
                rotation.rows[2][1] * self.scale.y,
                rotation.rows[2][2] * self.scale.z,
                self.translation.z,
            ],
            [0.0, 0.0, 0.0, 1.0],
        ])?)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
/// Single-precision TRS transform.
pub struct TrsTransform3 {
    /// Translation.
    pub translation: Vector3,
    /// Rotation.
    pub rotation: Quaternion,
    /// Non-uniform scale.
    pub scale: Vector3,
}

impl TrsTransform3 {
    /// Identity transform.
    pub const IDENTITY: Self = Self {
        translation: Vector3::ZERO,
        rotation: Quaternion::IDENTITY,
        scale: Vector3::new(1.0, 1.0, 1.0),
    };

    /// Creates a TRS transform.
    pub fn new(translation: Vector3, rotation: Quaternion, scale: Vector3) -> Result<Self> {
        TrsTransform3d::new(translation.into(), rotation.into(), scale.into())?;
        Ok(Self {
            translation,
            rotation: rotation.normalize()?,
            scale,
        })
    }

    /// Applies this transform to a point.
    pub fn apply_point(self, point: Point3) -> Result<Point3> {
        self.to_affine()?.apply_point(point)
    }

    /// Applies this transform to a vector.
    pub fn apply_vector(self, vector: Vector3) -> Result<Vector3> {
        self.to_affine()?.apply_vector(vector)
    }

    /// Converts this transform to affine form.
    pub fn to_affine(self) -> Result<AffineTransform3> {
        self.to_f64()?.to_affine()?.to_f32_checked()
    }

    /// Converts this value to double precision.
    pub fn to_f64(self) -> Result<TrsTransform3d> {
        TrsTransform3d::new(
            self.translation.into(),
            self.rotation.into(),
            self.scale.into(),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
/// Affine 3D transform backed by a row-major 4x4 matrix.
pub struct AffineTransform3d {
    /// Transform matrix.
    pub matrix: Matrix4d,
}

impl AffineTransform3d {
    /// Identity transform.
    pub const IDENTITY: Self = Self {
        matrix: Matrix4d::identity(),
    };

    /// Creates an affine transform from a matrix.
    pub fn from_matrix(matrix: Matrix4d) -> Result<Self> {
        matrix.validate()?;
        if (matrix.rows[3][0].abs() > ORTHONORMAL_EPSILON_F64)
            || (matrix.rows[3][1].abs() > ORTHONORMAL_EPSILON_F64)
            || (matrix.rows[3][2].abs() > ORTHONORMAL_EPSILON_F64)
            || ((matrix.rows[3][3] - 1.0).abs() > ORTHONORMAL_EPSILON_F64)
        {
            return Err(invalid_argument(
                "affine transform bottom row must be [0, 0, 0, 1]",
            ));
        }
        Ok(Self { matrix })
    }

    /// Applies this transform to a point.
    pub fn apply_point(self, point: Point3d) -> Result<Point3d> {
        self.matrix.transform_point(point)
    }

    /// Applies this transform to a vector.
    pub fn apply_vector(self, vector: Vector3d) -> Result<Vector3d> {
        self.matrix.transform_vector(vector)
    }

    /// Composes this transform followed by `next`.
    pub fn compose(self, next: Self) -> Result<Self> {
        Self::from_matrix(next.matrix.matmul(self.matrix)?)
    }

    /// Returns the inverse transform.
    pub fn inverse(self) -> Result<Self> {
        Self::from_matrix(self.matrix.inverse()?)
    }

    /// Converts this value to single precision.
    pub fn to_f32_checked(self) -> Result<AffineTransform3> {
        Ok(AffineTransform3 {
            matrix: self.matrix.to_f32_checked()?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
/// Single-precision affine 3D transform backed by a row-major 4x4 matrix.
pub struct AffineTransform3 {
    /// Transform matrix.
    pub matrix: Matrix4,
}

impl AffineTransform3 {
    /// Identity transform.
    pub const IDENTITY: Self = Self {
        matrix: Matrix4::identity(),
    };

    /// Creates an affine transform from a matrix.
    pub fn from_matrix(matrix: Matrix4) -> Result<Self> {
        AffineTransform3d::from_matrix(matrix.to_f64())?;
        Ok(Self { matrix })
    }

    /// Applies this transform to a point.
    pub fn apply_point(self, point: Point3) -> Result<Point3> {
        self.matrix.transform_point(point)
    }

    /// Applies this transform to a vector.
    pub fn apply_vector(self, vector: Vector3) -> Result<Vector3> {
        self.matrix.transform_vector(vector)
    }

    /// Composes this transform followed by `next`.
    pub fn compose(self, next: Self) -> Result<Self> {
        Self::from_matrix(next.matrix.matmul(self.matrix)?)
    }

    /// Returns the inverse transform.
    pub fn inverse(self) -> Result<Self> {
        Self::from_matrix(self.matrix.inverse()?)
    }

    /// Converts this value to double precision.
    pub fn to_f64(self) -> Result<AffineTransform3d> {
        AffineTransform3d::from_matrix(self.matrix.to_f64())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
/// Pinhole camera intrinsics.
pub struct PinholeIntrinsics {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Horizontal focal length in pixels.
    pub fx: f32,
    /// Vertical focal length in pixels.
    pub fy: f32,
    /// Principal point x coordinate.
    pub cx: f32,
    /// Principal point y coordinate.
    pub cy: f32,
}

impl PinholeIntrinsics {
    /// Creates pinhole intrinsics.
    pub fn new(width: u32, height: u32, fx: f32, fy: f32, cx: f32, cy: f32) -> Result<Self> {
        let intrinsics = Self {
            width,
            height,
            fx,
            fy,
            cx,
            cy,
        };
        intrinsics.validate()?;
        Ok(intrinsics)
    }

    /// Builds intrinsics from vertical field of view.
    pub fn from_vertical_fov(width: u32, height: u32, vertical_fov_radians: f32) -> Result<Self> {
        if width == 0 || height == 0 {
            return Err(invalid_argument("camera dimensions must be positive"));
        }
        if !vertical_fov_radians.is_finite()
            || vertical_fov_radians <= 0.0
            || vertical_fov_radians >= std::f32::consts::PI
        {
            return Err(invalid_argument(
                "vertical_fov_radians must be finite and in the open range (0, pi)",
            ));
        }
        let fy = (height as f32 * 0.5) / (vertical_fov_radians * 0.5).tan();
        Self::new(
            width,
            height,
            fy,
            fy,
            (width as f32 - 1.0) * 0.5,
            (height as f32 - 1.0) * 0.5,
        )
    }

    /// Validates this value.
    pub fn validate(self) -> Result<()> {
        if self.width == 0 || self.height == 0 {
            return Err(invalid_argument("camera dimensions must be positive"));
        }
        for (name, value) in [
            ("fx", self.fx),
            ("fy", self.fy),
            ("cx", self.cx),
            ("cy", self.cy),
        ] {
            if !value.is_finite() {
                return Err(invalid_argument(format!("{name} must be finite")));
            }
        }
        if self.fx <= 0.0 || self.fy <= 0.0 {
            return Err(invalid_argument("camera focal lengths must be positive"));
        }
        Ok(())
    }

    /// Converts this value to double precision.
    pub fn to_f64(self) -> PinholeIntrinsicsd {
        PinholeIntrinsicsd {
            width: self.width,
            height: self.height,
            fx: self.fx as f64,
            fy: self.fy as f64,
            cx: self.cx as f64,
            cy: self.cy as f64,
        }
    }

    /// Builds a row-major projection matrix for workspace `+Z` camera depth.
    pub fn projection_matrix_opencv_depth(self, near: f32, far: f32) -> Result<Matrix4> {
        self.to_f64()
            .projection_matrix_opencv_depth(near as f64, far as f64)?
            .to_f32_checked()
    }

    /// Builds a row-major WebGL projection matrix.
    ///
    /// This matrix expects camera coordinates whose forward axis has already
    /// been mapped from workspace `+Z` to WebGL `-Z`.
    pub fn projection_matrix_webgl(self, near: f32, far: f32) -> Result<Matrix4> {
        self.to_f64()
            .projection_matrix_webgl(near as f64, far as f64)?
            .to_f32_checked()
    }

    /// Converts a pixel and positive camera depth to normalized device coordinates.
    pub fn normalized_device_coordinates(self, pixel: [f32; 2], depth: f32) -> Result<[f32; 3]> {
        let ndc = self
            .to_f64()
            .normalized_device_coordinates([pixel[0] as f64, pixel[1] as f64], depth as f64)?;
        Ok([
            f64_to_f32(ndc[0], "x")?,
            f64_to_f32(ndc[1], "y")?,
            f64_to_f32(ndc[2], "z")?,
        ])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
/// Double-precision pinhole camera intrinsics.
pub struct PinholeIntrinsicsd {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Horizontal focal length in pixels.
    pub fx: f64,
    /// Vertical focal length in pixels.
    pub fy: f64,
    /// Principal point x coordinate.
    pub cx: f64,
    /// Principal point y coordinate.
    pub cy: f64,
}

impl PinholeIntrinsicsd {
    /// Creates pinhole intrinsics.
    pub fn new(width: u32, height: u32, fx: f64, fy: f64, cx: f64, cy: f64) -> Result<Self> {
        let intrinsics = Self {
            width,
            height,
            fx,
            fy,
            cx,
            cy,
        };
        intrinsics.validate()?;
        Ok(intrinsics)
    }

    /// Validates this value.
    pub fn validate(self) -> Result<()> {
        if self.width == 0 || self.height == 0 {
            return Err(invalid_argument("camera dimensions must be positive"));
        }
        for (name, value) in [
            ("fx", self.fx),
            ("fy", self.fy),
            ("cx", self.cx),
            ("cy", self.cy),
        ] {
            if !value.is_finite() {
                return Err(invalid_argument(format!("{name} must be finite")));
            }
        }
        if self.fx <= 0.0 || self.fy <= 0.0 {
            return Err(invalid_argument("camera focal lengths must be positive"));
        }
        Ok(())
    }

    /// Converts this value to single precision.
    pub fn to_f32_checked(self) -> Result<PinholeIntrinsics> {
        PinholeIntrinsics::new(
            self.width,
            self.height,
            f64_to_f32(self.fx, "fx")?,
            f64_to_f32(self.fy, "fy")?,
            f64_to_f32(self.cx, "cx")?,
            f64_to_f32(self.cy, "cy")?,
        )
    }

    /// Builds a row-major projection matrix for workspace `+Z` camera depth.
    pub fn projection_matrix_opencv_depth(self, near: f64, far: f64) -> Result<Matrix4d> {
        self.validate_projection_range(near, far)?;
        let width = self.width as f64;
        let height = self.height as f64;
        Matrix4d::new([
            [2.0 * self.fx / width, 0.0, 2.0 * self.cx / width - 1.0, 0.0],
            [
                0.0,
                2.0 * self.fy / height,
                2.0 * self.cy / height - 1.0,
                0.0,
            ],
            [0.0, 0.0, far / (far - near), -(far * near) / (far - near)],
            [0.0, 0.0, 1.0, 0.0],
        ])
    }

    /// Builds a row-major WebGL projection matrix.
    ///
    /// This matrix expects camera coordinates whose forward axis has already
    /// been mapped from workspace `+Z` to WebGL `-Z`.
    pub fn projection_matrix_webgl(self, near: f64, far: f64) -> Result<Matrix4d> {
        self.validate_projection_range(near, far)?;
        let width = self.width as f64;
        let height = self.height as f64;
        Matrix4d::new([
            [2.0 * self.fx / width, 0.0, 1.0 - 2.0 * self.cx / width, 0.0],
            [
                0.0,
                2.0 * self.fy / height,
                2.0 * self.cy / height - 1.0,
                0.0,
            ],
            [
                0.0,
                0.0,
                -(far + near) / (far - near),
                -(2.0 * far * near) / (far - near),
            ],
            [0.0, 0.0, -1.0, 0.0],
        ])
    }

    /// Converts a pixel and positive camera depth to normalized device coordinates.
    pub fn normalized_device_coordinates(self, pixel: [f64; 2], depth: f64) -> Result<[f64; 3]> {
        self.validate()?;
        if !pixel[0].is_finite() || !pixel[1].is_finite() {
            return Err(invalid_argument("pixel coordinates must be finite"));
        }
        if !depth.is_finite() || depth <= 0.0 {
            return Err(invalid_argument("depth must be finite and positive"));
        }
        Ok([
            (2.0 * pixel[0] / self.width as f64) - 1.0,
            (2.0 * pixel[1] / self.height as f64) - 1.0,
            depth,
        ])
    }

    fn validate_projection_range(self, near: f64, far: f64) -> Result<()> {
        self.validate()?;
        if !near.is_finite() || !far.is_finite() || near < 0.0 || far <= near {
            return Err(invalid_argument(
                "near and far must be finite with 0 <= near < far",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
/// Camera pose in workspace coordinates: right-handed, +Y up, camera forward +Z.
pub struct CameraPose3 {
    /// Camera position.
    pub position: Point3,
    /// Camera right axis.
    pub right: Vector3,
    /// Camera up axis.
    pub up: Vector3,
    /// Camera forward axis.
    pub forward: Vector3,
}

impl CameraPose3 {
    /// Creates a camera pose from an orthonormal basis.
    pub fn new(position: Point3, right: Vector3, up: Vector3, forward: Vector3) -> Result<Self> {
        let pose = Self {
            position,
            right: right.normalize()?,
            up: up.normalize()?,
            forward: forward.normalize()?,
        };
        pose.validate()?;
        Ok(pose)
    }

    /// Returns identity pose.
    pub fn identity() -> Self {
        Self {
            position: Point3::new(0.0, 0.0, 0.0),
            right: Vector3::new(1.0, 0.0, 0.0),
            up: Vector3::new(0.0, 1.0, 0.0),
            forward: Vector3::new(0.0, 0.0, 1.0),
        }
    }

    /// Builds a look-at pose.
    pub fn look_at(position: Point3, target: Point3, up_hint: Vector3) -> Result<Self> {
        let forward = (target - position).normalize()?;
        let right = up_hint.cross(forward).normalize()?;
        let up = forward.cross(right).normalize()?;
        Self::new(position, right, up, forward)
    }

    /// Builds a workspace pose from COLMAP world-to-camera values.
    pub fn from_colmap_world_to_camera(
        qw: f32,
        qx: f32,
        qy: f32,
        qz: f32,
        tx: f32,
        ty: f32,
        tz: f32,
    ) -> Result<Self> {
        CameraPose3d::from_colmap_world_to_camera(
            qw as f64, qx as f64, qy as f64, qz as f64, tx as f64, ty as f64, tz as f64,
        )?
        .to_f32_checked()
    }

    /// Validates this pose.
    pub fn validate(self) -> Result<()> {
        if !self.position.is_finite() {
            return Err(invalid_argument("camera position must be finite"));
        }
        validate_orthonormal3(self.right, self.up, self.forward)
    }

    /// Converts this value to double precision.
    pub fn to_f64(self) -> Result<CameraPose3d> {
        CameraPose3d::new(
            self.position.into(),
            self.right.into(),
            self.up.into(),
            self.forward.into(),
        )
    }

    /// Converts camera-space direction into world-space direction.
    pub fn camera_to_world_direction(self, camera_direction: Vector3) -> Vector3 {
        (self.right * camera_direction.x)
            + (self.up * camera_direction.y)
            + (self.forward * camera_direction.z)
    }

    /// Converts a world-space point to camera-space.
    pub fn world_to_camera_point(self, point: Point3) -> Vector3 {
        let offset = point - self.position;
        Vector3::new(
            offset.dot(self.right),
            offset.dot(self.up),
            offset.dot(self.forward),
        )
    }

    /// Converts a camera-space point to world-space.
    pub fn camera_to_world_point(self, point: Vector3) -> Point3 {
        self.position + self.right * point.x + self.up * point.y + self.forward * point.z
    }

    /// Builds a ray through a pixel.
    pub fn pixel_ray(
        self,
        intrinsics: PinholeIntrinsics,
        pixel: [f32; 2],
        near: f32,
        far: f32,
    ) -> Result<CameraRay> {
        intrinsics.validate()?;
        if !pixel[0].is_finite() || !pixel[1].is_finite() {
            return Err(invalid_argument("pixel coordinates must be finite"));
        }
        if !near.is_finite() || !far.is_finite() || near < 0.0 || far <= near {
            return Err(invalid_argument(
                "near and far must be finite with 0 <= near < far",
            ));
        }
        let camera_direction = Vector3::new(
            (pixel[0] - intrinsics.cx) / intrinsics.fx,
            (pixel[1] - intrinsics.cy) / intrinsics.fy,
            1.0,
        )
        .normalize()?;
        CameraRay::new(
            self.position,
            self.camera_to_world_direction(camera_direction),
            near,
            far,
        )
    }

    /// Projects a point into pixel coordinates, returning `None` behind the camera.
    pub fn project_point(
        self,
        intrinsics: PinholeIntrinsics,
        point: Point3,
    ) -> Result<Option<[f32; 2]>> {
        intrinsics.validate()?;
        if !point.is_finite() {
            return Err(invalid_argument("point must be finite"));
        }
        let camera_space = self.world_to_camera_point(point);
        if camera_space.z <= 0.0 {
            return Ok(None);
        }
        Ok(Some([
            intrinsics.fx * (camera_space.x / camera_space.z) + intrinsics.cx,
            intrinsics.fy * (camera_space.y / camera_space.z) + intrinsics.cy,
        ]))
    }

    /// Returns a row-major view matrix for workspace `+Z` forward semantics.
    pub fn view_matrix(self) -> Result<Matrix4> {
        self.validate()?;
        let p = self.position;
        Matrix4::new([
            [
                self.right.x,
                self.right.y,
                self.right.z,
                -self.right.dot(p - Point3::new(0.0, 0.0, 0.0)),
            ],
            [
                self.up.x,
                self.up.y,
                self.up.z,
                -self.up.dot(p - Point3::new(0.0, 0.0, 0.0)),
            ],
            [
                self.forward.x,
                self.forward.y,
                self.forward.z,
                -self.forward.dot(p - Point3::new(0.0, 0.0, 0.0)),
            ],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    /// Returns a row-major WebGL-style view matrix with camera forward mapped to `-Z`.
    pub fn gltf_webgl_view_matrix(self) -> Result<Matrix4> {
        self.to_f64()?.gltf_webgl_view_matrix()?.to_f32_checked()
    }

    /// Returns COLMAP world-to-camera quaternion and translation values.
    pub fn to_colmap_world_to_camera(self) -> Result<ColmapWorldToCamera> {
        self.to_f64()?.to_colmap_world_to_camera()?.to_f32_checked()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
/// Double-precision camera pose in workspace coordinates.
pub struct CameraPose3d {
    /// Camera position.
    pub position: Point3d,
    /// Camera right axis.
    pub right: Vector3d,
    /// Camera up axis.
    pub up: Vector3d,
    /// Camera forward axis.
    pub forward: Vector3d,
}

impl CameraPose3d {
    /// Creates a camera pose from an orthonormal basis.
    pub fn new(
        position: Point3d,
        right: Vector3d,
        up: Vector3d,
        forward: Vector3d,
    ) -> Result<Self> {
        let pose = Self {
            position,
            right: right.normalize()?,
            up: up.normalize()?,
            forward: forward.normalize()?,
        };
        pose.validate()?;
        Ok(pose)
    }

    /// Returns identity pose.
    pub fn identity() -> Self {
        Self {
            position: Point3d::new(0.0, 0.0, 0.0),
            right: Vector3d::X,
            up: Vector3d::Y,
            forward: Vector3d::Z,
        }
    }

    /// Builds a look-at pose.
    pub fn look_at(position: Point3d, target: Point3d, up_hint: Vector3d) -> Result<Self> {
        let forward = (target - position).normalize()?;
        let right = up_hint.cross(forward).normalize()?;
        let up = forward.cross(right).normalize()?;
        Self::new(position, right, up, forward)
    }

    /// Builds a workspace pose from COLMAP world-to-camera values.
    pub fn from_colmap_world_to_camera(
        qw: f64,
        qx: f64,
        qy: f64,
        qz: f64,
        tx: f64,
        ty: f64,
        tz: f64,
    ) -> Result<Self> {
        for (name, value) in [
            ("qw", qw),
            ("qx", qx),
            ("qy", qy),
            ("qz", qz),
            ("tx", tx),
            ("ty", ty),
            ("tz", tz),
        ] {
            if !value.is_finite() {
                return Err(invalid_argument(format!("{name} must be finite")));
            }
        }
        let rotation = Quaterniond::new(qx, qy, qz, qw).normalize()?;
        let matrix = rotation.to_rotation_matrix()?;
        let t = Vector3d::new(tx, ty, tz);
        let position = Point3d::new(
            -(matrix.rows[0][0] * t.x + matrix.rows[1][0] * t.y + matrix.rows[2][0] * t.z),
            -(matrix.rows[0][1] * t.x + matrix.rows[1][1] * t.y + matrix.rows[2][1] * t.z),
            -(matrix.rows[0][2] * t.x + matrix.rows[1][2] * t.y + matrix.rows[2][2] * t.z),
        );
        Self::new(
            position,
            Vector3d::new(matrix.rows[0][0], matrix.rows[0][1], matrix.rows[0][2]),
            Vector3d::new(matrix.rows[1][0], matrix.rows[1][1], matrix.rows[1][2]),
            Vector3d::new(matrix.rows[2][0], matrix.rows[2][1], matrix.rows[2][2]),
        )
    }

    /// Validates this pose.
    pub fn validate(self) -> Result<()> {
        if !self.position.is_finite() {
            return Err(invalid_argument("camera position must be finite"));
        }
        validate_orthonormal3d(self.right, self.up, self.forward)
    }

    /// Converts this value to single precision.
    pub fn to_f32_checked(self) -> Result<CameraPose3> {
        CameraPose3::new(
            self.position.to_f32_checked()?,
            self.right.to_f32_checked()?,
            self.up.to_f32_checked()?,
            self.forward.to_f32_checked()?,
        )
    }

    /// Converts camera-space direction into world-space direction.
    pub fn camera_to_world_direction(self, camera_direction: Vector3d) -> Vector3d {
        (self.right * camera_direction.x)
            + (self.up * camera_direction.y)
            + (self.forward * camera_direction.z)
    }

    /// Converts a world-space point to camera-space.
    pub fn world_to_camera_point(self, point: Point3d) -> Vector3d {
        let offset = point - self.position;
        Vector3d::new(
            offset.dot(self.right),
            offset.dot(self.up),
            offset.dot(self.forward),
        )
    }

    /// Converts a camera-space point to world-space.
    pub fn camera_to_world_point(self, point: Vector3d) -> Point3d {
        self.position + self.right * point.x + self.up * point.y + self.forward * point.z
    }

    /// Builds a ray through a pixel.
    pub fn pixel_ray(
        self,
        intrinsics: PinholeIntrinsicsd,
        pixel: [f64; 2],
        near: f64,
        far: f64,
    ) -> Result<CameraRayd> {
        intrinsics.validate()?;
        if !pixel[0].is_finite() || !pixel[1].is_finite() {
            return Err(invalid_argument("pixel coordinates must be finite"));
        }
        if !near.is_finite() || !far.is_finite() || near < 0.0 || far <= near {
            return Err(invalid_argument(
                "near and far must be finite with 0 <= near < far",
            ));
        }
        let camera_direction = Vector3d::new(
            (pixel[0] - intrinsics.cx) / intrinsics.fx,
            (pixel[1] - intrinsics.cy) / intrinsics.fy,
            1.0,
        )
        .normalize()?;
        CameraRayd::new(
            self.position,
            self.camera_to_world_direction(camera_direction),
            near,
            far,
        )
    }

    /// Projects a point into pixel coordinates, returning `None` behind the camera.
    pub fn project_point(
        self,
        intrinsics: PinholeIntrinsicsd,
        point: Point3d,
    ) -> Result<Option<[f64; 2]>> {
        intrinsics.validate()?;
        if !point.is_finite() {
            return Err(invalid_argument("point must be finite"));
        }
        let camera_space = self.world_to_camera_point(point);
        if camera_space.z <= 0.0 {
            return Ok(None);
        }
        Ok(Some([
            intrinsics.fx * (camera_space.x / camera_space.z) + intrinsics.cx,
            intrinsics.fy * (camera_space.y / camera_space.z) + intrinsics.cy,
        ]))
    }

    /// Returns a row-major view matrix for workspace `+Z` forward semantics.
    pub fn view_matrix(self) -> Result<Matrix4d> {
        self.validate()?;
        let p = self.position - Point3d::new(0.0, 0.0, 0.0);
        Matrix4d::new([
            [self.right.x, self.right.y, self.right.z, -self.right.dot(p)],
            [self.up.x, self.up.y, self.up.z, -self.up.dot(p)],
            [
                self.forward.x,
                self.forward.y,
                self.forward.z,
                -self.forward.dot(p),
            ],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    /// Returns a row-major WebGL-style view matrix with camera forward mapped to `-Z`.
    pub fn gltf_webgl_view_matrix(self) -> Result<Matrix4d> {
        self.validate()?;
        let p = self.position - Point3d::new(0.0, 0.0, 0.0);
        Matrix4d::new([
            [self.right.x, self.right.y, self.right.z, -self.right.dot(p)],
            [self.up.x, self.up.y, self.up.z, -self.up.dot(p)],
            [
                -self.forward.x,
                -self.forward.y,
                -self.forward.z,
                self.forward.dot(p),
            ],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    /// Returns COLMAP world-to-camera quaternion and translation values.
    pub fn to_colmap_world_to_camera(self) -> Result<ColmapWorldToCamerad> {
        self.validate()?;
        let rotation = Matrix3d::new([
            [self.right.x, self.right.y, self.right.z],
            [self.up.x, self.up.y, self.up.z],
            [self.forward.x, self.forward.y, self.forward.z],
        ])?;
        let q = Quaterniond::from_rotation_matrix(rotation)?;
        let c = self.position - Point3d::new(0.0, 0.0, 0.0);
        Ok(ColmapWorldToCamerad {
            qw: q.w,
            qx: q.x,
            qy: q.y,
            qz: q.z,
            tx: -self.right.dot(c),
            ty: -self.up.dot(c),
            tz: -self.forward.dot(c),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
/// Camera ray.
pub struct CameraRay {
    /// Ray origin.
    pub origin: Point3,
    /// Normalized ray direction.
    pub direction: Vector3,
    /// Minimum ray distance.
    pub t_min: f32,
    /// Maximum ray distance.
    pub t_max: f32,
}

impl CameraRay {
    /// Creates a camera ray.
    pub fn new(origin: Point3, direction: Vector3, t_min: f32, t_max: f32) -> Result<Self> {
        if !origin.is_finite() {
            return Err(invalid_argument("ray origin must be finite"));
        }
        if !t_min.is_finite() || !t_max.is_finite() || t_min < 0.0 || t_max <= t_min {
            return Err(invalid_argument(
                "ray t_min and t_max must be finite with 0 <= t_min < t_max",
            ));
        }
        Ok(Self {
            origin,
            direction: direction.normalize()?,
            t_min,
            t_max,
        })
    }

    /// Returns a point along the ray.
    pub fn at(self, t: f32) -> Result<Point3> {
        if !t.is_finite() {
            return Err(invalid_argument("ray t must be finite"));
        }
        Ok(self.origin + self.direction * t)
    }

    /// Converts this value to double precision.
    pub fn to_f64(self) -> CameraRayd {
        CameraRayd {
            origin: self.origin.into(),
            direction: self.direction.into(),
            t_min: self.t_min as f64,
            t_max: self.t_max as f64,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
/// Double-precision camera ray.
pub struct CameraRayd {
    /// Ray origin.
    pub origin: Point3d,
    /// Normalized ray direction.
    pub direction: Vector3d,
    /// Minimum ray distance.
    pub t_min: f64,
    /// Maximum ray distance.
    pub t_max: f64,
}

impl CameraRayd {
    /// Creates a camera ray.
    pub fn new(origin: Point3d, direction: Vector3d, t_min: f64, t_max: f64) -> Result<Self> {
        if !origin.is_finite() {
            return Err(invalid_argument("ray origin must be finite"));
        }
        if !t_min.is_finite() || !t_max.is_finite() || t_min < 0.0 || t_max <= t_min {
            return Err(invalid_argument(
                "ray t_min and t_max must be finite with 0 <= t_min < t_max",
            ));
        }
        Ok(Self {
            origin,
            direction: direction.normalize()?,
            t_min,
            t_max,
        })
    }

    /// Returns a point along the ray.
    pub fn at(self, t: f64) -> Result<Point3d> {
        if !t.is_finite() {
            return Err(invalid_argument("ray t must be finite"));
        }
        Ok(self.origin + self.direction * t)
    }

    /// Converts this value to single precision.
    pub fn to_f32_checked(self) -> Result<CameraRay> {
        CameraRay::new(
            self.origin.to_f32_checked()?,
            self.direction.to_f32_checked()?,
            f64_to_f32(self.t_min, "t_min")?,
            f64_to_f32(self.t_max, "t_max")?,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
/// COLMAP world-to-camera pose payload.
pub struct ColmapWorldToCamera {
    /// Quaternion scalar component.
    pub qw: f32,
    /// Quaternion x component.
    pub qx: f32,
    /// Quaternion y component.
    pub qy: f32,
    /// Quaternion z component.
    pub qz: f32,
    /// Translation x component.
    pub tx: f32,
    /// Translation y component.
    pub ty: f32,
    /// Translation z component.
    pub tz: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
/// Double-precision COLMAP world-to-camera pose payload.
pub struct ColmapWorldToCamerad {
    /// Quaternion scalar component.
    pub qw: f64,
    /// Quaternion x component.
    pub qx: f64,
    /// Quaternion y component.
    pub qy: f64,
    /// Quaternion z component.
    pub qz: f64,
    /// Translation x component.
    pub tx: f64,
    /// Translation y component.
    pub ty: f64,
    /// Translation z component.
    pub tz: f64,
}

impl ColmapWorldToCamerad {
    /// Converts this value to single precision.
    pub fn to_f32_checked(self) -> Result<ColmapWorldToCamera> {
        Ok(ColmapWorldToCamera {
            qw: f64_to_f32(self.qw, "qw")?,
            qx: f64_to_f32(self.qx, "qx")?,
            qy: f64_to_f32(self.qy, "qy")?,
            qz: f64_to_f32(self.qz, "qz")?,
            tx: f64_to_f32(self.tx, "tx")?,
            ty: f64_to_f32(self.ty, "ty")?,
            tz: f64_to_f32(self.tz, "tz")?,
        })
    }
}

impl Quaternion {
    /// Converts this value to double precision.
    pub fn to_f64(self) -> Quaterniond {
        self.into()
    }

    /// Returns the inverse rotation.
    pub fn inverse(self) -> Result<Self> {
        Ok(self.normalize()?.conjugate())
    }

    /// Converts this rotation to a row-major 3x3 matrix.
    pub fn to_rotation_matrix(self) -> Result<Matrix3> {
        Quaterniond::from(self)
            .to_rotation_matrix()?
            .to_f32_checked()
    }

    /// Builds a quaternion from a row-major rotation matrix.
    pub fn from_rotation_matrix(matrix: Matrix3) -> Result<Self> {
        Quaterniond::from_rotation_matrix(matrix.to_f64())?.to_f32_checked()
    }

    /// Builds a quaternion from explicit Euler angles.
    pub fn from_euler(order: EulerOrder, x: f32, y: f32, z: f32) -> Result<Self> {
        Quaterniond::from_euler(order, x as f64, y as f64, z as f64)?.to_f32_checked()
    }

    /// Converts this quaternion to axis-angle form.
    pub fn to_axis_angle(self) -> Result<(Vector3, f32)> {
        let (axis, angle) = Quaterniond::from(self).to_axis_angle()?;
        Ok((axis.to_f32_checked()?, f64_to_f32(angle, "angle")?))
    }

    /// Converts this quaternion to Euler angles for import/export and UI controls.
    ///
    /// Quaternions remain the primary workspace rotation representation. Euler
    /// angles are provided only for boundary formats and direct manipulation UI.
    pub fn to_euler(self, order: EulerOrder) -> Result<(f32, f32, f32)> {
        let (x, y, z) = Quaterniond::from(self).to_euler(order)?;
        Ok((
            f64_to_f32(x, "x")?,
            f64_to_f32(y, "y")?,
            f64_to_f32(z, "z")?,
        ))
    }
}

impl Vector3 {
    /// Converts this value to double precision.
    pub fn to_f64(self) -> Vector3d {
        self.into()
    }

    /// Returns this value as an array.
    pub fn to_array(self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }
}

impl Point3 {
    /// Converts this value to double precision.
    pub fn to_f64(self) -> Point3d {
        self.into()
    }

    /// Returns this value as an array.
    pub fn to_array(self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }
}

fn validate_vector3d(vector: Vector3d, name: &str) -> Result<()> {
    if vector.is_finite() {
        Ok(())
    } else {
        Err(invalid_argument(format!(
            "{name} components must be finite"
        )))
    }
}

fn validate_orthonormal3(x: Vector3, y: Vector3, z: Vector3) -> Result<()> {
    for (name, axis) in [("right", x), ("up", y), ("forward", z)] {
        if !axis.is_finite() {
            return Err(invalid_argument(format!("axis {name} must be finite")));
        }
        if (axis.length() - 1.0).abs() > ORTHONORMAL_EPSILON_F32 {
            return Err(invalid_argument(format!("axis {name} must be normalized")));
        }
    }
    if x.dot(y).abs() > ORTHONORMAL_EPSILON_F32
        || x.dot(z).abs() > ORTHONORMAL_EPSILON_F32
        || y.dot(z).abs() > ORTHONORMAL_EPSILON_F32
    {
        return Err(invalid_argument("axes must be orthogonal"));
    }
    Ok(())
}

fn validate_orthonormal3d(x: Vector3d, y: Vector3d, z: Vector3d) -> Result<()> {
    for (name, axis) in [("right", x), ("up", y), ("forward", z)] {
        if !axis.is_finite() {
            return Err(invalid_argument(format!("axis {name} must be finite")));
        }
        if (axis.length() - 1.0).abs() > ORTHONORMAL_EPSILON_F64 {
            return Err(invalid_argument(format!("axis {name} must be normalized")));
        }
    }
    if x.dot(y).abs() > ORTHONORMAL_EPSILON_F64
        || x.dot(z).abs() > ORTHONORMAL_EPSILON_F64
        || y.dot(z).abs() > ORTHONORMAL_EPSILON_F64
    {
        return Err(invalid_argument("axes must be orthogonal"));
    }
    Ok(())
}

fn euler_from_matrix(order: EulerOrder, rows: [[f64; 3]; 3]) -> Result<(f64, f64, f64)> {
    let _nalgebra_reference = nalgebra::Matrix3::<f64>::from_row_slice(&[
        rows[0][0], rows[0][1], rows[0][2], rows[1][0], rows[1][1], rows[1][2], rows[2][0],
        rows[2][1], rows[2][2],
    ]);
    let epsilon = 1.0e-12;
    let angles = match order {
        EulerOrder::Xyz => {
            let y = (-rows[2][0]).clamp(-1.0, 1.0).asin();
            if y.cos().abs() > epsilon {
                (
                    rows[2][1].atan2(rows[2][2]),
                    y,
                    rows[1][0].atan2(rows[0][0]),
                )
            } else {
                (0.0, y, (-rows[0][1]).atan2(rows[1][1]))
            }
        }
        EulerOrder::Xzy => {
            let z = rows[1][0].clamp(-1.0, 1.0).asin();
            if z.cos().abs() > epsilon {
                (
                    (-rows[1][2]).atan2(rows[1][1]),
                    rows[2][0].atan2(rows[0][0]),
                    z,
                )
            } else {
                (rows[2][1].atan2(rows[2][2]), 0.0, z)
            }
        }
        EulerOrder::Yxz => {
            let x = rows[2][1].clamp(-1.0, 1.0).asin();
            if x.cos().abs() > epsilon {
                (
                    x,
                    (-rows[2][0]).atan2(rows[2][2]),
                    (-rows[0][1]).atan2(rows[1][1]),
                )
            } else {
                (x, rows[0][2].atan2(rows[0][0]), 0.0)
            }
        }
        EulerOrder::Yzx => {
            let z = (-rows[0][1]).clamp(-1.0, 1.0).asin();
            if z.cos().abs() > epsilon {
                (
                    rows[2][1].atan2(rows[1][1]),
                    rows[0][2].atan2(rows[0][0]),
                    z,
                )
            } else {
                (0.0, (-rows[2][0]).atan2(rows[2][2]), z)
            }
        }
        EulerOrder::Zxy => {
            let x = (-rows[1][2]).clamp(-1.0, 1.0).asin();
            if x.cos().abs() > epsilon {
                (
                    x,
                    rows[0][2].atan2(rows[2][2]),
                    rows[1][0].atan2(rows[1][1]),
                )
            } else {
                (x, 0.0, (-rows[0][1]).atan2(rows[0][0]))
            }
        }
        EulerOrder::Zyx => {
            let y = rows[0][2].clamp(-1.0, 1.0).asin();
            if y.cos().abs() > epsilon {
                (
                    (-rows[1][2]).atan2(rows[2][2]),
                    y,
                    (-rows[0][1]).atan2(rows[0][0]),
                )
            } else {
                (rows[2][1].atan2(rows[1][1]), y, 0.0)
            }
        }
    };
    if angles.0.is_finite() && angles.1.is_finite() && angles.2.is_finite() {
        Ok(angles)
    } else {
        Err(invalid_argument("Euler angles must be finite"))
    }
}

fn f64_to_f32(value: f64, name: &str) -> Result<f32> {
    if !value.is_finite() || value < f32::MIN as f64 || value > f32::MAX as f64 {
        return Err(invalid_argument(format!(
            "{name} must be finite and representable as f32"
        )));
    }
    Ok(value as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(left: f32, right: f32) {
        assert!((left - right).abs() < 1.0e-4, "{left} != {right}");
    }

    fn assert_close64(left: f64, right: f64) {
        assert!((left - right).abs() < 1.0e-9, "{left} != {right}");
    }

    #[test]
    fn similarity_transforms_and_camera_rayd_are_stable() {
        let similarity =
            SimilarityTransform3::new(Vector3::new(1.0, 0.0, 0.0), Quaternion::IDENTITY, 2.0)
                .unwrap();
        assert_eq!(
            similarity
                .to_affine()
                .unwrap()
                .apply_point(Point3::new(1.0, 0.0, 0.0))
                .unwrap(),
            Point3::new(3.0, 0.0, 0.0)
        );
        let doubled = similarity.to_f64().unwrap().to_f32_checked().unwrap();
        assert_eq!(doubled.scale, 2.0);

        let ray = CameraRayd::new(Point3d::new(0.0, 0.0, 0.0), Vector3d::Z, 0.1, 10.0).unwrap();
        assert_close64(ray.at(2.0).unwrap().z, 2.0);
        assert_close(ray.to_f32_checked().unwrap().direction.z, 1.0);
    }

    #[test]
    fn matrices_preserve_row_major_storage_and_column_vector_semantics() {
        let matrix = Matrix4::from_row_major_array([
            1.0, 0.0, 0.0, 2.0, 0.0, 1.0, 0.0, 3.0, 0.0, 0.0, 1.0, 4.0, 0.0, 0.0, 0.0, 1.0,
        ])
        .unwrap();
        assert_eq!(matrix.to_column_major_array()[12], 2.0);
        assert_eq!(
            Matrix4::from_column_major_array(matrix.to_column_major_array()).unwrap(),
            matrix
        );
        assert_eq!(
            matrix.transform_point(Point3::new(1.0, 1.0, 1.0)).unwrap(),
            Point3::new(3.0, 4.0, 5.0)
        );
        assert_close(matrix.determinant().unwrap(), 1.0);

        let matrix3 =
            Matrix3::from_column_major_array([1.0, 2.0, 3.0, 0.0, 1.0, 4.0, 5.0, 6.0, 0.0])
                .unwrap();
        assert_eq!(
            matrix3.to_row_major_array(),
            [1.0, 0.0, 5.0, 2.0, 1.0, 6.0, 3.0, 4.0, 0.0]
        );
        assert_close(matrix3.determinant().unwrap(), 1.0);
        assert_eq!(
            matrix3.transpose().unwrap().to_column_major_array(),
            matrix3.to_row_major_array()
        );
    }

    #[test]
    fn affine_inverse_round_trips_points_and_vectors() {
        let transform = TrsTransform3::new(
            Vector3::new(2.0, 3.0, 4.0),
            Quaternion::from_axis_angle(Vector3::new(0.0, 0.0, 1.0), std::f32::consts::FRAC_PI_2)
                .unwrap(),
            Vector3::new(2.0, 3.0, 4.0),
        )
        .unwrap()
        .to_affine()
        .unwrap();
        let inverse = transform.inverse().unwrap();
        let point = Point3::new(1.0, 2.0, 3.0);
        let vector = Vector3::new(1.0, 0.0, 0.0);
        let recovered_point = inverse
            .apply_point(transform.apply_point(point).unwrap())
            .unwrap();
        let recovered_vector = inverse
            .apply_vector(transform.apply_vector(vector).unwrap())
            .unwrap();
        assert_close(recovered_point.x, point.x);
        assert_close(recovered_point.y, point.y);
        assert_close(recovered_point.z, point.z);
        assert_close(recovered_vector.x, vector.x);
        assert_close(recovered_vector.y, vector.y);
        assert_close(recovered_vector.z, vector.z);
    }

    #[test]
    fn quaternion_axis_angle_and_euler_round_trip() {
        let q = Quaternion::from_axis_angle(Vector3::new(0.0, 1.0, 0.0), 0.5).unwrap();
        let (axis, angle) = q.to_axis_angle().unwrap();
        assert_close(axis.y, 1.0);
        assert_close(angle, 0.5);

        for order in [EulerOrder::Xyz, EulerOrder::Zyx] {
            let source = Quaternion::from_euler(order, 0.2, -0.3, 0.4).unwrap();
            let (x, y, z) = source.to_euler(order).unwrap();
            let roundtrip = Quaternion::from_euler(order, x, y, z).unwrap();
            assert!(source.dot(roundtrip).abs() > 0.9999);
        }
    }

    #[test]
    fn camera_view_and_projection_conventions_are_validated() {
        let pose = CameraPose3::identity();
        assert_eq!(pose.gltf_webgl_view_matrix().unwrap().rows[2][2], -1.0);

        let intrinsics = PinholeIntrinsics::new(32, 32, 30.0, 30.0, 15.0, 15.0).unwrap();
        assert!(intrinsics.projection_matrix_webgl(0.1, 100.0).is_ok());
        assert!(intrinsics
            .projection_matrix_opencv_depth(0.1, 100.0)
            .is_ok());
        assert!(intrinsics.projection_matrix_webgl(1.0, 1.0).is_err());
        assert!(PinholeIntrinsics::new(32, 32, 0.0, 30.0, 15.0, 15.0).is_err());
    }
}
