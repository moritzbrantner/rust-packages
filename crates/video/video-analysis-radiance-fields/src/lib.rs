use std::ops::{Add, AddAssign, Div, Mul, Sub};

use video_analysis_core::{DetectError, Result};

fn invalid_argument(message: impl Into<String>) -> DetectError {
    DetectError::InvalidArgument(message.into())
}

fn validate_finite(value: f32, name: &str) -> Result<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(invalid_argument(format!("{name} must be finite")))
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }

    pub fn length_squared(self) -> f32 {
        self.x.mul_add(self.x, self.y * self.y)
    }

    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }
}

impl Add for Vec2 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl Sub for Vec2 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl Mul<f32> for Vec2 {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    pub const X: Self = Self {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    };
    pub const Y: Self = Self {
        x: 0.0,
        y: 1.0,
        z: 0.0,
    };
    pub const Z: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 1.0,
    };

    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn splat(value: f32) -> Self {
        Self::new(value, value, value)
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }

    pub fn dot(self, rhs: Self) -> f32 {
        self.x.mul_add(rhs.x, self.y.mul_add(rhs.y, self.z * rhs.z))
    }

    pub fn cross(self, rhs: Self) -> Self {
        Self::new(
            self.y.mul_add(rhs.z, -(self.z * rhs.y)),
            self.z.mul_add(rhs.x, -(self.x * rhs.z)),
            self.x.mul_add(rhs.y, -(self.y * rhs.x)),
        )
    }

    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }

    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    pub fn normalize(self) -> Result<Self> {
        if !self.is_finite() {
            return Err(invalid_argument("vector components must be finite"));
        }
        let length = self.length();
        if length <= f32::EPSILON {
            return Err(invalid_argument("vector length must be greater than zero"));
        }
        Ok(self / length)
    }

    pub fn min(self, rhs: Self) -> Self {
        Self::new(self.x.min(rhs.x), self.y.min(rhs.y), self.z.min(rhs.z))
    }

    pub fn max(self, rhs: Self) -> Self {
        Self::new(self.x.max(rhs.x), self.y.max(rhs.y), self.z.max(rhs.z))
    }
}

impl Add for Vec3 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl AddAssign for Vec3 {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
        self.z += rhs.z;
    }
}

impl Sub for Vec3 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl Mul<f32> for Vec3 {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

impl Mul<Vec3> for f32 {
    type Output = Vec3;

    fn mul(self, rhs: Vec3) -> Self::Output {
        rhs * self
    }
}

impl Div<f32> for Vec3 {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs, self.z / rhs)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorRgb {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl ColorRgb {
    pub const BLACK: Self = Self::new(0.0, 0.0, 0.0);
    pub const WHITE: Self = Self::new(1.0, 1.0, 1.0);

    pub const fn new(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b }
    }

    pub fn is_finite(self) -> bool {
        self.r.is_finite() && self.g.is_finite() && self.b.is_finite()
    }

    pub fn clamp01(self) -> Self {
        Self::new(
            self.r.clamp(0.0, 1.0),
            self.g.clamp(0.0, 1.0),
            self.b.clamp(0.0, 1.0),
        )
    }
}

impl Default for ColorRgb {
    fn default() -> Self {
        Self::BLACK
    }
}

impl Add for ColorRgb {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.r + rhs.r, self.g + rhs.g, self.b + rhs.b)
    }
}

impl AddAssign for ColorRgb {
    fn add_assign(&mut self, rhs: Self) {
        self.r += rhs.r;
        self.g += rhs.g;
        self.b += rhs.b;
    }
}

impl Mul<f32> for ColorRgb {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.r * rhs, self.g * rhs, self.b * rhs)
    }
}

impl Mul<ColorRgb> for f32 {
    type Output = ColorRgb;

    fn mul(self, rhs: ColorRgb) -> Self::Output {
        rhs * self
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
    pub t_min: f32,
    pub t_max: f32,
}

impl Ray {
    pub fn new(origin: Vec3, direction: Vec3, t_min: f32, t_max: f32) -> Result<Self> {
        let ray = Self {
            origin,
            direction: direction.normalize()?,
            t_min,
            t_max,
        };
        ray.validate()?;
        Ok(ray)
    }

    pub fn at(self, t: f32) -> Vec3 {
        self.origin + (self.direction * t)
    }

    pub fn validate(self) -> Result<()> {
        if !self.origin.is_finite() || !self.direction.is_finite() {
            return Err(invalid_argument("ray origin and direction must be finite"));
        }
        validate_finite(self.t_min, "ray t_min")?;
        validate_finite(self.t_max, "ray t_max")?;
        if self.t_min < 0.0 {
            return Err(invalid_argument(
                "ray t_min must be greater than or equal to zero",
            ));
        }
        if self.t_max <= self.t_min {
            return Err(invalid_argument("ray t_max must be greater than t_min"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraIntrinsics {
    pub width: u32,
    pub height: u32,
    pub fx: f32,
    pub fy: f32,
    pub cx: f32,
    pub cy: f32,
}

impl CameraIntrinsics {
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

    pub fn pinhole(width: u32, height: u32, vertical_fov_radians: f32) -> Result<Self> {
        if width == 0 || height == 0 {
            return Err(invalid_argument("camera dimensions must be positive"));
        }
        validate_finite(vertical_fov_radians, "vertical_fov_radians")?;
        if vertical_fov_radians <= 0.0 || vertical_fov_radians >= std::f32::consts::PI {
            return Err(invalid_argument(
                "vertical_fov_radians must be in the open range (0, pi)",
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
            validate_finite(value, name)?;
        }
        if self.fx <= 0.0 || self.fy <= 0.0 {
            return Err(invalid_argument("camera focal lengths must be positive"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraPose {
    pub position: Vec3,
    pub right: Vec3,
    pub up: Vec3,
    pub forward: Vec3,
}

impl CameraPose {
    pub fn new(position: Vec3, right: Vec3, up: Vec3, forward: Vec3) -> Result<Self> {
        let pose = Self {
            position,
            right: right.normalize()?,
            up: up.normalize()?,
            forward: forward.normalize()?,
        };
        pose.validate()?;
        Ok(pose)
    }

    pub fn identity() -> Self {
        Self {
            position: Vec3::ZERO,
            right: Vec3::X,
            up: Vec3::Y,
            forward: Vec3::Z,
        }
    }

    pub fn look_at(position: Vec3, target: Vec3, up_hint: Vec3) -> Result<Self> {
        let forward = (target - position).normalize()?;
        let right = up_hint.cross(forward).normalize()?;
        let up = forward.cross(right).normalize()?;
        Self::new(position, right, up, forward)
    }

    pub fn validate(self) -> Result<()> {
        if !self.position.is_finite() {
            return Err(invalid_argument("camera position must be finite"));
        }
        for (name, axis) in [
            ("right", self.right),
            ("up", self.up),
            ("forward", self.forward),
        ] {
            if !axis.is_finite() {
                return Err(invalid_argument(format!(
                    "camera {name} axis must be finite"
                )));
            }
            if (axis.length() - 1.0).abs() > 1.0e-3 {
                return Err(invalid_argument(format!(
                    "camera {name} axis must be normalized"
                )));
            }
        }
        if self.right.dot(self.up).abs() > 1.0e-3
            || self.right.dot(self.forward).abs() > 1.0e-3
            || self.up.dot(self.forward).abs() > 1.0e-3
        {
            return Err(invalid_argument("camera axes must be orthogonal"));
        }
        Ok(())
    }

    pub fn camera_to_world_direction(self, camera_direction: Vec3) -> Vec3 {
        (self.right * camera_direction.x)
            + (self.up * camera_direction.y)
            + (self.forward * camera_direction.z)
    }

    pub fn world_to_camera_point(self, point: Vec3) -> Vec3 {
        let offset = point - self.position;
        Vec3::new(
            offset.dot(self.right),
            offset.dot(self.up),
            offset.dot(self.forward),
        )
    }

    pub fn pixel_ray(
        self,
        intrinsics: CameraIntrinsics,
        pixel: Vec2,
        near: f32,
        far: f32,
    ) -> Result<Ray> {
        intrinsics.validate()?;
        if !pixel.is_finite() {
            return Err(invalid_argument("pixel coordinates must be finite"));
        }
        let camera_direction = Vec3::new(
            (pixel.x - intrinsics.cx) / intrinsics.fx,
            (pixel.y - intrinsics.cy) / intrinsics.fy,
            1.0,
        )
        .normalize()?;
        let world_direction = self.camera_to_world_direction(camera_direction);
        Ray::new(self.position, world_direction, near, far)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RadianceSample {
    pub position: Vec3,
    pub direction: Vec3,
    pub t: f32,
    pub delta: f32,
}

impl RadianceSample {
    pub fn new(position: Vec3, direction: Vec3, t: f32, delta: f32) -> Result<Self> {
        let sample = Self {
            position,
            direction: direction.normalize()?,
            t,
            delta,
        };
        sample.validate()?;
        Ok(sample)
    }

    pub fn validate(self) -> Result<()> {
        if !self.position.is_finite() || !self.direction.is_finite() {
            return Err(invalid_argument(
                "radiance sample position and direction must be finite",
            ));
        }
        validate_finite(self.t, "sample t")?;
        validate_finite(self.delta, "sample delta")?;
        if self.t < 0.0 || self.delta <= 0.0 {
            return Err(invalid_argument(
                "sample t must be non-negative and delta must be positive",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Radiance {
    pub color: ColorRgb,
    pub density: f32,
}

impl Radiance {
    pub const TRANSPARENT: Self = Self {
        color: ColorRgb::BLACK,
        density: 0.0,
    };

    pub fn new(color: ColorRgb, density: f32) -> Result<Self> {
        let radiance = Self { color, density };
        radiance.validate()?;
        Ok(radiance)
    }

    pub fn validate(self) -> Result<()> {
        if !self.color.is_finite() {
            return Err(invalid_argument("radiance color must be finite"));
        }
        validate_finite(self.density, "radiance density")?;
        if self.density < 0.0 {
            return Err(invalid_argument(
                "radiance density must be greater than or equal to zero",
            ));
        }
        Ok(())
    }
}

pub trait RadianceField {
    fn query(&self, sample: RadianceSample) -> Result<Radiance>;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConstantRadianceField {
    pub radiance: Radiance,
}

impl ConstantRadianceField {
    pub fn new(radiance: Radiance) -> Self {
        Self { radiance }
    }
}

impl RadianceField for ConstantRadianceField {
    fn query(&self, _sample: RadianceSample) -> Result<Radiance> {
        Ok(self.radiance)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VolumeRenderConfig {
    pub near: f32,
    pub far: f32,
    pub step_size: f32,
    pub background: ColorRgb,
    pub opacity_stop: f32,
}

impl VolumeRenderConfig {
    pub fn new(near: f32, far: f32, step_size: f32) -> Result<Self> {
        let config = Self {
            near,
            far,
            step_size,
            ..Self::default()
        };
        config.validate()?;
        Ok(config)
    }

    pub fn background(mut self, color: ColorRgb) -> Self {
        self.background = color;
        self
    }

    pub fn opacity_stop(mut self, opacity: f32) -> Self {
        self.opacity_stop = opacity;
        self
    }

    pub fn validate(self) -> Result<()> {
        for (name, value) in [
            ("near", self.near),
            ("far", self.far),
            ("step_size", self.step_size),
            ("opacity_stop", self.opacity_stop),
        ] {
            validate_finite(value, name)?;
        }
        if self.near < 0.0 {
            return Err(invalid_argument(
                "near must be greater than or equal to zero",
            ));
        }
        if self.far <= self.near {
            return Err(invalid_argument("far must be greater than near"));
        }
        if self.step_size <= 0.0 {
            return Err(invalid_argument("step_size must be positive"));
        }
        if !(0.0..=1.0).contains(&self.opacity_stop) {
            return Err(invalid_argument("opacity_stop must be in the range [0, 1]"));
        }
        if !self.background.is_finite() {
            return Err(invalid_argument("background color must be finite"));
        }
        Ok(())
    }
}

impl Default for VolumeRenderConfig {
    fn default() -> Self {
        Self {
            near: 0.0,
            far: 1.0,
            step_size: 0.01,
            background: ColorRgb::BLACK,
            opacity_stop: 0.995,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderedRay {
    pub color: ColorRgb,
    pub opacity: f32,
    pub samples: u32,
}

pub fn render_ray<F: RadianceField>(
    field: &F,
    ray: Ray,
    config: VolumeRenderConfig,
) -> Result<RenderedRay> {
    ray.validate()?;
    config.validate()?;

    let near = config.near.max(ray.t_min);
    let far = config.far.min(ray.t_max);
    if far <= near {
        return Err(invalid_argument(
            "render interval must overlap the ray interval",
        ));
    }

    let mut color = ColorRgb::BLACK;
    let mut transmittance = 1.0_f32;
    let mut t = near;
    let mut samples = 0_u32;

    while t < far && 1.0 - transmittance < config.opacity_stop {
        let delta = config.step_size.min(far - t);
        let sample = RadianceSample::new(ray.at(t + delta * 0.5), ray.direction, t, delta)?;
        let radiance = field.query(sample)?;
        radiance.validate()?;

        let alpha = 1.0 - (-radiance.density * delta).exp();
        let weight = transmittance * alpha;
        color += radiance.color * weight;
        transmittance *= 1.0 - alpha;

        t += delta;
        samples += 1;
    }

    let opacity = 1.0 - transmittance;
    color += config.background * transmittance;

    Ok(RenderedRay {
        color: color.clamp01(),
        opacity,
        samples,
    })
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxisAlignedBounds {
    pub min: Vec3,
    pub max: Vec3,
}

impl AxisAlignedBounds {
    pub fn new(min: Vec3, max: Vec3) -> Result<Self> {
        let bounds = Self { min, max };
        bounds.validate()?;
        Ok(bounds)
    }

    pub fn validate(self) -> Result<()> {
        if !self.min.is_finite() || !self.max.is_finite() {
            return Err(invalid_argument("bounds must be finite"));
        }
        if self.max.x <= self.min.x || self.max.y <= self.min.y || self.max.z <= self.min.z {
            return Err(invalid_argument(
                "bounds max components must be greater than min components",
            ));
        }
        Ok(())
    }

    pub fn contains(self, point: Vec3) -> bool {
        point.x >= self.min.x
            && point.y >= self.min.y
            && point.z >= self.min.z
            && point.x <= self.max.x
            && point.y <= self.max.y
            && point.z <= self.max.z
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridResolution {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

impl GridResolution {
    pub fn new(x: u32, y: u32, z: u32) -> Result<Self> {
        if x == 0 || y == 0 || z == 0 {
            return Err(invalid_argument("grid resolution must be positive"));
        }
        Ok(Self { x, y, z })
    }

    pub fn voxel_count(self) -> u64 {
        u64::from(self.x) * u64::from(self.y) * u64::from(self.z)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RadianceGridSpec {
    pub bounds: AxisAlignedBounds,
    pub resolution: GridResolution,
}

impl RadianceGridSpec {
    pub fn new(bounds: AxisAlignedBounds, resolution: GridResolution) -> Result<Self> {
        bounds.validate()?;
        Ok(Self { bounds, resolution })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(left: f32, right: f32) {
        assert!(
            (left - right).abs() < 1.0e-4,
            "expected {left} to be approximately {right}"
        );
    }

    #[test]
    fn camera_center_pixel_points_forward() {
        let intrinsics = CameraIntrinsics::new(101, 101, 50.0, 50.0, 50.0, 50.0).unwrap();
        let ray = CameraPose::identity()
            .pixel_ray(intrinsics, Vec2::new(50.0, 50.0), 0.1, 10.0)
            .unwrap();

        approx_eq(ray.direction.x, 0.0);
        approx_eq(ray.direction.y, 0.0);
        approx_eq(ray.direction.z, 1.0);
    }

    #[test]
    fn render_transparent_field_returns_background() {
        let field = ConstantRadianceField::new(Radiance::TRANSPARENT);
        let ray = Ray::new(Vec3::ZERO, Vec3::Z, 0.0, 1.0).unwrap();
        let config = VolumeRenderConfig::new(0.0, 1.0, 0.1)
            .unwrap()
            .background(ColorRgb::new(0.2, 0.4, 0.6));

        let rendered = render_ray(&field, ray, config).unwrap();

        approx_eq(rendered.opacity, 0.0);
        approx_eq(rendered.color.r, 0.2);
        approx_eq(rendered.color.g, 0.4);
        approx_eq(rendered.color.b, 0.6);
    }

    #[test]
    fn render_constant_field_accumulates_opacity() {
        let radiance = Radiance::new(ColorRgb::new(1.0, 0.0, 0.0), 2.0).unwrap();
        let field = ConstantRadianceField::new(radiance);
        let ray = Ray::new(Vec3::ZERO, Vec3::Z, 0.0, 1.0).unwrap();
        let rendered = render_ray(
            &field,
            ray,
            VolumeRenderConfig::new(0.0, 1.0, 0.05).unwrap(),
        )
        .unwrap();

        assert!(rendered.opacity > 0.8);
        assert!(rendered.color.r > rendered.color.g);
        assert_eq!(rendered.samples, 20);
    }

    #[test]
    fn invalid_grid_resolution_is_rejected() {
        assert!(GridResolution::new(16, 0, 16).is_err());
    }
}
