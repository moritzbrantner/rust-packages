#![doc = include_str!("../README.md")]

pub mod surface;
use std::cmp::Ordering;

use video_analysis_core::{DetectError, Result};
use video_analysis_radiance_fields::{
    AxisAlignedBounds, CameraIntrinsics, CameraPose, ColorRgb, Vec2, Vec3,
};

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

#[derive(Debug, Clone, Copy, PartialEq)]
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
    pub const IDENTITY: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        w: 1.0,
    };

    /// Creates a new value.
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    /// Returns length squared.
    pub fn length_squared(self) -> f32 {
        self.x.mul_add(
            self.x,
            self.y
                .mul_add(self.y, self.z.mul_add(self.z, self.w * self.w)),
        )
    }

    /// Normalizes this value.
    pub fn normalize(self) -> Result<Self> {
        if !self.x.is_finite() || !self.y.is_finite() || !self.z.is_finite() || !self.w.is_finite()
        {
            return Err(invalid_argument("quaternion components must be finite"));
        }
        let length = self.length_squared().sqrt();
        if length <= f32::EPSILON {
            return Err(invalid_argument(
                "quaternion length must be greater than zero",
            ));
        }
        Ok(Self::new(
            self.x / length,
            self.y / length,
            self.z / length,
            self.w / length,
        ))
    }

    /// Converts this value to rotation matrix.
    pub fn to_rotation_matrix(self) -> Result<[[f32; 3]; 3]> {
        Ok(self.to_core_quaternion()?.to_rotation_matrix()?.rows)
    }

    /// Converts this value to the canonical 3D core quaternion.
    pub fn to_core_quaternion(self) -> Result<three_d_processing_core::Quaternion> {
        three_d_processing_core::Quaternion::new(self.x, self.y, self.z, self.w).normalize()
    }

    /// Builds this value from the canonical 3D core quaternion.
    pub fn from_core_quaternion(value: three_d_processing_core::Quaternion) -> Result<Self> {
        let value = value.normalize()?;
        Ok(Self::new(value.x, value.y, value.z, value.w))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Data type for covariance3.
pub struct Covariance3 {
    /// The xx value.
    pub xx: f32,
    /// The xy value.
    pub xy: f32,
    /// The xz value.
    pub xz: f32,
    /// The yy value.
    pub yy: f32,
    /// The yz value.
    pub yz: f32,
    /// The zz value.
    pub zz: f32,
}

impl Covariance3 {
    /// Builds this value from scale rotation.
    pub fn from_scale_rotation(scale: Vec3, rotation: Quaternion) -> Result<Self> {
        validate_scale(scale)?;
        let r = rotation.to_rotation_matrix()?;
        let variances = [scale.x * scale.x, scale.y * scale.y, scale.z * scale.z];

        let covariance = |row: usize, col: usize| -> f32 {
            r[row][0] * variances[0] * r[col][0]
                + r[row][1] * variances[1] * r[col][1]
                + r[row][2] * variances[2] * r[col][2]
        };

        Ok(Self {
            xx: covariance(0, 0),
            xy: covariance(0, 1),
            xz: covariance(0, 2),
            yy: covariance(1, 1),
            yz: covariance(1, 2),
            zz: covariance(2, 2),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Data type for gaussian3d.
pub struct Gaussian3d {
    /// The center value.
    pub center: Vec3,
    /// The scale value.
    pub scale: Vec3,
    /// The rotation value.
    pub rotation: Quaternion,
    /// The color value.
    pub color: ColorRgb,
    /// The opacity value.
    pub opacity: f32,
}

impl Gaussian3d {
    /// Creates a new value.
    pub fn new(
        center: Vec3,
        scale: Vec3,
        rotation: Quaternion,
        color: ColorRgb,
        opacity: f32,
    ) -> Result<Self> {
        let gaussian = Self {
            center,
            scale,
            rotation: rotation.normalize()?,
            color,
            opacity,
        };
        gaussian.validate()?;
        Ok(gaussian)
    }

    /// Returns isotropic.
    pub fn isotropic(center: Vec3, radius: f32, color: ColorRgb, opacity: f32) -> Result<Self> {
        Self::new(
            center,
            Vec3::splat(radius),
            Quaternion::IDENTITY,
            color,
            opacity,
        )
    }

    /// Validates this value.
    pub fn validate(self) -> Result<()> {
        if !self.center.is_finite() {
            return Err(invalid_argument("gaussian center must be finite"));
        }
        validate_scale(self.scale)?;
        self.rotation.normalize()?;
        if !self.color.is_finite() {
            return Err(invalid_argument("gaussian color must be finite"));
        }
        validate_finite(self.opacity, "opacity")?;
        if !(0.0..=1.0).contains(&self.opacity) {
            return Err(invalid_argument("opacity must be in the range [0, 1]"));
        }
        Ok(())
    }

    /// Returns covariance.
    pub fn covariance(self) -> Result<Covariance3> {
        Covariance3::from_scale_rotation(self.scale, self.rotation)
    }

    /// Returns max scale.
    pub fn max_scale(self) -> f32 {
        self.scale.x.max(self.scale.y).max(self.scale.z)
    }
}

fn validate_scale(scale: Vec3) -> Result<()> {
    if !scale.is_finite() {
        return Err(invalid_argument("gaussian scale must be finite"));
    }
    if scale.x <= 0.0 || scale.y <= 0.0 || scale.z <= 0.0 {
        return Err(invalid_argument(
            "gaussian scale components must be positive",
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for gaussian scene.
pub struct GaussianScene {
    splats: Vec<Gaussian3d>,
}

impl GaussianScene {
    /// Creates a new value.
    pub fn new(splats: impl Into<Vec<Gaussian3d>>) -> Result<Self> {
        let scene = Self {
            splats: splats.into(),
        };
        scene.validate()?;
        Ok(scene)
    }

    /// Returns empty.
    pub fn empty() -> Self {
        Self { splats: Vec::new() }
    }

    /// Returns splats.
    pub fn splats(&self) -> &[Gaussian3d] {
        &self.splats
    }

    /// Adds push to this value.
    pub fn push(&mut self, splat: Gaussian3d) -> Result<()> {
        splat.validate()?;
        self.splats.push(splat);
        Ok(())
    }

    /// Validates this value.
    pub fn validate(&self) -> Result<()> {
        for splat in &self.splats {
            splat.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Data type for projected gaussian.
pub struct ProjectedGaussian {
    /// The center value.
    pub center: Vec2,
    /// The radius pixels value.
    pub radius_pixels: f32,
    /// The depth value.
    pub depth: f32,
    /// The color value.
    pub color: ColorRgb,
    /// The opacity value.
    pub opacity: f32,
}

impl ProjectedGaussian {
    /// Validates this value.
    pub fn validate(self) -> Result<()> {
        if !self.center.is_finite() {
            return Err(invalid_argument("projected gaussian center must be finite"));
        }
        validate_finite(self.radius_pixels, "radius_pixels")?;
        validate_finite(self.depth, "depth")?;
        validate_finite(self.opacity, "opacity")?;
        if self.radius_pixels <= 0.0 {
            return Err(invalid_argument("radius_pixels must be positive"));
        }
        if self.depth <= 0.0 {
            return Err(invalid_argument("depth must be positive"));
        }
        if !(0.0..=1.0).contains(&self.opacity) {
            return Err(invalid_argument("opacity must be in the range [0, 1]"));
        }
        if !self.color.is_finite() {
            return Err(invalid_argument("projected gaussian color must be finite"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Data type for projection config.
pub struct ProjectionConfig {
    /// The min depth value.
    pub min_depth: f32,
    /// The max radius pixels value.
    pub max_radius_pixels: f32,
    /// The standard deviations value.
    pub standard_deviations: f32,
}

impl ProjectionConfig {
    /// Validates this value.
    pub fn validate(self) -> Result<()> {
        validate_finite(self.min_depth, "min_depth")?;
        validate_finite(self.max_radius_pixels, "max_radius_pixels")?;
        validate_finite(self.standard_deviations, "standard_deviations")?;
        if self.min_depth <= 0.0 {
            return Err(invalid_argument("min_depth must be positive"));
        }
        if self.max_radius_pixels <= 0.0 {
            return Err(invalid_argument("max_radius_pixels must be positive"));
        }
        if self.standard_deviations <= 0.0 {
            return Err(invalid_argument("standard_deviations must be positive"));
        }
        Ok(())
    }
}

impl Default for ProjectionConfig {
    fn default() -> Self {
        Self {
            min_depth: 1.0e-4,
            max_radius_pixels: 4096.0,
            standard_deviations: 3.0,
        }
    }
}

/// Returns project gaussian.
pub fn project_gaussian(
    gaussian: Gaussian3d,
    intrinsics: CameraIntrinsics,
    pose: CameraPose,
    config: ProjectionConfig,
) -> Result<Option<ProjectedGaussian>> {
    gaussian.validate()?;
    intrinsics.validate()?;
    pose.validate()?;
    config.validate()?;

    let camera_space = pose.world_to_camera_point(gaussian.center);
    if camera_space.z <= config.min_depth {
        return Ok(None);
    }

    let center = Vec2::new(
        intrinsics.fx * (camera_space.x / camera_space.z) + intrinsics.cx,
        intrinsics.fy * (camera_space.y / camera_space.z) + intrinsics.cy,
    );
    let focal = intrinsics.fx.max(intrinsics.fy);
    let radius_pixels = (gaussian.max_scale() * focal / camera_space.z)
        .abs()
        .mul_add(config.standard_deviations, 0.0)
        .clamp(1.0, config.max_radius_pixels);

    let projected = ProjectedGaussian {
        center,
        radius_pixels,
        depth: camera_space.z,
        color: gaussian.color,
        opacity: gaussian.opacity,
    };
    projected.validate()?;
    Ok(Some(projected))
}

/// Returns project scene.
pub fn project_scene(
    scene: &GaussianScene,
    intrinsics: CameraIntrinsics,
    pose: CameraPose,
    config: ProjectionConfig,
) -> Result<Vec<ProjectedGaussian>> {
    let mut projected = Vec::new();
    for gaussian in scene.splats() {
        if let Some(splat) = project_gaussian(*gaussian, intrinsics, pose, config)? {
            projected.push(splat);
        }
    }
    sort_back_to_front(&mut projected);
    Ok(projected)
}

/// Returns sort back to front.
pub fn sort_back_to_front(splats: &mut [ProjectedGaussian]) {
    splats.sort_by(|left, right| {
        right
            .depth
            .partial_cmp(&left.depth)
            .unwrap_or(Ordering::Equal)
    });
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Data type for splat pixel.
pub struct SplatPixel {
    /// The color value.
    pub color: ColorRgb,
    /// The alpha value.
    pub alpha: f32,
}

impl SplatPixel {
    /// Constant for transparent.
    pub const TRANSPARENT: Self = Self {
        color: ColorRgb::BLACK,
        alpha: 0.0,
    };
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Data type for splat render config.
pub struct SplatRenderConfig {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// The background value.
    pub background: ColorRgb,
    /// The alpha cutoff value.
    pub alpha_cutoff: f32,
}

impl SplatRenderConfig {
    /// Creates a new value.
    pub fn new(width: u32, height: u32) -> Result<Self> {
        let config = Self {
            width,
            height,
            ..Self::default()
        };
        config.validate()?;
        Ok(config)
    }

    /// Returns background.
    pub fn background(mut self, color: ColorRgb) -> Self {
        self.background = color;
        self
    }

    /// Validates this value.
    pub fn validate(self) -> Result<()> {
        if self.width == 0 || self.height == 0 {
            return Err(invalid_argument("render dimensions must be positive"));
        }
        if !self.background.is_finite() {
            return Err(invalid_argument("background color must be finite"));
        }
        validate_finite(self.alpha_cutoff, "alpha_cutoff")?;
        if !(0.0..=1.0).contains(&self.alpha_cutoff) {
            return Err(invalid_argument("alpha_cutoff must be in the range [0, 1]"));
        }
        Ok(())
    }
}

impl Default for SplatRenderConfig {
    fn default() -> Self {
        Self {
            width: 1,
            height: 1,
            background: ColorRgb::BLACK,
            alpha_cutoff: 0.995,
        }
    }
}

/// Returns gaussian weight.
pub fn gaussian_weight(splat: ProjectedGaussian, pixel: Vec2) -> Result<f32> {
    splat.validate()?;
    if !pixel.is_finite() {
        return Err(invalid_argument("pixel coordinates must be finite"));
    }
    let offset = pixel - splat.center;
    let variance = splat.radius_pixels * splat.radius_pixels;
    Ok((-0.5 * offset.length_squared() / variance).exp())
}

/// Returns composite splats at pixel.
pub fn composite_splats_at_pixel(
    splats_back_to_front: &[ProjectedGaussian],
    pixel: Vec2,
    background: ColorRgb,
) -> Result<SplatPixel> {
    if !background.is_finite() {
        return Err(invalid_argument("background color must be finite"));
    }

    let mut color = background;
    let mut alpha = 0.0_f32;
    for splat in splats_back_to_front {
        let weight = gaussian_weight(*splat, pixel)?;
        let source_alpha = (splat.opacity * weight).clamp(0.0, 1.0);
        color = (splat.color * source_alpha) + (color * (1.0 - source_alpha));
        alpha = source_alpha + alpha * (1.0 - source_alpha);
    }

    Ok(SplatPixel {
        color: color.clamp01(),
        alpha,
    })
}

/// Returns splat pixel index.
pub fn splat_pixel_index(x: u32, y: u32, width: u32) -> usize {
    y as usize * width as usize + x as usize
}

/// Returns render projected splats.
pub fn render_projected_splats(
    splats_back_to_front: &[ProjectedGaussian],
    config: SplatRenderConfig,
) -> Result<Vec<SplatPixel>> {
    config.validate()?;
    let mut pixels = Vec::with_capacity(config.width as usize * config.height as usize);
    for y in 0..config.height {
        for x in 0..config.width {
            let pixel = Vec2::new(x as f32, y as f32);
            let mut relevant = Vec::new();
            for splat in splats_back_to_front {
                let offset = pixel - splat.center;
                if offset.length() <= splat.radius_pixels {
                    relevant.push(*splat);
                }
            }
            let splatted = composite_splats_at_pixel(&relevant, pixel, config.background)?;
            pixels.push(splatted);
        }
    }
    Ok(pixels)
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for spherical harmonics RGB.
pub struct SphericalHarmonicsRgb {
    /// The degree value.
    pub degree: u8,
    /// The coeffs value.
    pub coeffs: Vec<[f32; 3]>,
}

impl SphericalHarmonicsRgb {
    /// Returns dc.
    pub fn dc(color: ColorRgb) -> Self {
        Self {
            degree: 0,
            coeffs: vec![[color.r, color.g, color.b]],
        }
    }

    /// Validates this value.
    pub fn validate(&self) -> Result<()> {
        let expected = spherical_harmonic_coeff_count(self.degree);
        if self.coeffs.len() != expected {
            return Err(invalid_argument(format!(
                "spherical harmonics degree {} requires {expected} coefficient(s)",
                self.degree
            )));
        }
        for coeff in &self.coeffs {
            if coeff.iter().any(|value| !value.is_finite()) {
                return Err(invalid_argument(
                    "spherical harmonics coefficients must be finite",
                ));
            }
        }
        Ok(())
    }

    /// Returns preview color.
    pub fn preview_color(&self) -> ColorRgb {
        const SH_C0: f32 = 0.282_094_8;
        self.coeffs
            .first()
            .map(|coeff| {
                ColorRgb::new(
                    coeff[0].mul_add(SH_C0, 0.5),
                    coeff[1].mul_add(SH_C0, 0.5),
                    coeff[2].mul_add(SH_C0, 0.5),
                )
                .clamp01()
            })
            .unwrap_or(ColorRgb::WHITE)
    }
}

fn spherical_harmonic_coeff_count(degree: u8) -> usize {
    let degree = usize::from(degree) + 1;
    degree * degree
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Data type for scene transform3.
pub struct SceneTransform3 {
    /// The translation value.
    pub translation: Vec3,
    /// The uniform scale value.
    pub uniform_scale: f32,
}

impl SceneTransform3 {
    /// Constant for identity.
    pub const IDENTITY: Self = Self {
        translation: Vec3::ZERO,
        uniform_scale: 1.0,
    };

    /// Validates this value.
    pub fn validate(self) -> Result<()> {
        if !self.translation.is_finite() {
            return Err(invalid_argument(
                "scene transform translation must be finite",
            ));
        }
        validate_finite(self.uniform_scale, "uniform_scale")?;
        if self.uniform_scale <= 0.0 {
            return Err(invalid_argument("uniform_scale must be positive"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for gaussian splat3d.
pub struct GaussianSplat3d {
    /// The mean value.
    pub mean: Vec3,
    /// The scale log value.
    pub scale_log: Vec3,
    /// The rotation value.
    pub rotation: Quaternion,
    /// The opacity logit value.
    pub opacity_logit: f32,
    /// The sh value.
    pub sh: SphericalHarmonicsRgb,
}

impl GaussianSplat3d {
    /// Validates this value.
    pub fn validate(&self) -> Result<()> {
        if !self.mean.is_finite() {
            return Err(invalid_argument("splat mean must be finite"));
        }
        if !self.scale_log.is_finite() {
            return Err(invalid_argument("splat log scale must be finite"));
        }
        self.rotation.normalize()?;
        validate_finite(self.opacity_logit, "opacity_logit")?;
        self.sh.validate()?;
        Ok(())
    }

    /// Returns scale.
    pub fn scale(&self) -> Vec3 {
        Vec3::new(
            self.scale_log.x.exp(),
            self.scale_log.y.exp(),
            self.scale_log.z.exp(),
        )
    }

    /// Returns opacity.
    pub fn opacity(&self) -> f32 {
        1.0 / (1.0 + (-self.opacity_logit).exp())
    }

    /// Returns preview color.
    pub fn preview_color(&self) -> ColorRgb {
        self.sh.preview_color()
    }

    /// Converts this value to preview gaussian.
    pub fn to_preview_gaussian(&self) -> Result<Gaussian3d> {
        self.validate()?;
        Gaussian3d::new(
            self.mean,
            self.scale(),
            self.rotation,
            self.preview_color(),
            self.opacity(),
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for gaussian splat scene.
pub struct GaussianSplatScene {
    /// The splats value.
    pub splats: Vec<GaussianSplat3d>,
}

impl GaussianSplatScene {
    /// Validates this value.
    pub fn validate(&self) -> Result<()> {
        for splat in &self.splats {
            splat.validate()?;
        }
        Ok(())
    }

    /// Returns stats.
    pub fn stats(&self) -> Result<GaussianSceneStats> {
        self.validate()?;
        if self.splats.is_empty() {
            return Ok(GaussianSceneStats {
                count: 0,
                bounds: None,
                mean_opacity: 0.0,
                min_scale: Vec3::ZERO,
                max_scale: Vec3::ZERO,
            });
        }

        let mut min = self.splats[0].mean;
        let mut max = self.splats[0].mean;
        let mut min_scale = self.splats[0].scale();
        let mut max_scale = min_scale;
        let mut opacity_sum = 0.0_f32;
        for splat in &self.splats {
            min = min.min(splat.mean);
            max = max.max(splat.mean);
            let scale = splat.scale();
            min_scale = min_scale.min(scale);
            max_scale = max_scale.max(scale);
            opacity_sum += splat.opacity();
        }
        expand_degenerate_bounds(&mut min, &mut max);

        Ok(GaussianSceneStats {
            count: self.splats.len(),
            bounds: Some(AxisAlignedBounds::new(min, max)?),
            mean_opacity: opacity_sum / self.splats.len() as f32,
            min_scale,
            max_scale,
        })
    }

    /// Returns transformed.
    pub fn transformed(&self, transform: SceneTransform3) -> Result<Self> {
        self.validate()?;
        transform.validate()?;
        let log_scale_delta = transform.uniform_scale.ln();
        Ok(Self {
            splats: self
                .splats
                .iter()
                .map(|splat| GaussianSplat3d {
                    mean: splat.mean * transform.uniform_scale + transform.translation,
                    scale_log: splat.scale_log + Vec3::splat(log_scale_delta),
                    rotation: splat.rotation,
                    opacity_logit: splat.opacity_logit,
                    sh: splat.sh.clone(),
                })
                .collect(),
        })
    }

    /// Returns retain opacity at least.
    pub fn retain_opacity_at_least(&mut self, min_opacity: f32) -> Result<()> {
        validate_finite(min_opacity, "min_opacity")?;
        if !(0.0..=1.0).contains(&min_opacity) {
            return Err(invalid_argument("min_opacity must be in the range [0, 1]"));
        }
        self.validate()?;
        self.splats.retain(|splat| splat.opacity() >= min_opacity);
        Ok(())
    }

    /// Returns retain in bounds.
    pub fn retain_in_bounds(&mut self, bounds: AxisAlignedBounds) -> Result<()> {
        bounds.validate()?;
        self.validate()?;
        self.splats.retain(|splat| bounds.contains(splat.mean));
        Ok(())
    }

    /// Returns downsample stride.
    pub fn downsample_stride(&self, stride: usize) -> Result<Self> {
        self.validate()?;
        if stride == 0 {
            return Err(invalid_argument("downsample stride must be positive"));
        }
        Ok(Self {
            splats: self.splats.iter().step_by(stride).cloned().collect(),
        })
    }
}

fn expand_degenerate_bounds(min: &mut Vec3, max: &mut Vec3) {
    let epsilon = 1.0e-6;
    if min.x == max.x {
        min.x -= epsilon;
        max.x += epsilon;
    }
    if min.y == max.y {
        min.y -= epsilon;
        max.y += epsilon;
    }
    if min.z == max.z {
        min.z -= epsilon;
        max.z += epsilon;
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for gaussian scene stats.
pub struct GaussianSceneStats {
    /// Number of items represented by this value.
    pub count: usize,
    /// The bounds value.
    pub bounds: Option<AxisAlignedBounds>,
    /// The mean opacity value.
    pub mean_opacity: f32,
    /// The min scale value.
    pub min_scale: Vec3,
    /// The max scale value.
    pub max_scale: Vec3,
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
    fn gaussian_validation_rejects_invalid_scale() {
        let gaussian = Gaussian3d::isotropic(Vec3::ZERO, 0.0, ColorRgb::WHITE, 1.0);
        assert!(gaussian.is_err());
    }

    #[test]
    fn covariance_for_identity_rotation_matches_scale_variance() {
        let gaussian = Gaussian3d::new(
            Vec3::ZERO,
            Vec3::new(2.0, 3.0, 4.0),
            Quaternion::IDENTITY,
            ColorRgb::WHITE,
            1.0,
        )
        .unwrap();
        let covariance = gaussian.covariance().unwrap();

        approx_eq(covariance.xx, 4.0);
        approx_eq(covariance.yy, 9.0);
        approx_eq(covariance.zz, 16.0);
        approx_eq(covariance.xy, 0.0);
    }

    #[test]
    fn projection_places_center_splat_on_center_pixel() {
        let gaussian =
            Gaussian3d::isotropic(Vec3::new(0.0, 0.0, 2.0), 0.1, ColorRgb::WHITE, 0.5).unwrap();
        let intrinsics = CameraIntrinsics::new(101, 101, 50.0, 50.0, 50.0, 50.0).unwrap();
        let projected = project_gaussian(
            gaussian,
            intrinsics,
            CameraPose::identity(),
            ProjectionConfig::default(),
        )
        .unwrap()
        .unwrap();

        approx_eq(projected.center.x, 50.0);
        approx_eq(projected.center.y, 50.0);
        approx_eq(projected.depth, 2.0);
    }

    #[test]
    fn projection_culls_points_behind_camera() {
        let gaussian =
            Gaussian3d::isotropic(Vec3::new(0.0, 0.0, -1.0), 0.1, ColorRgb::WHITE, 1.0).unwrap();
        let intrinsics = CameraIntrinsics::new(101, 101, 50.0, 50.0, 50.0, 50.0).unwrap();
        let projected = project_gaussian(
            gaussian,
            intrinsics,
            CameraPose::identity(),
            ProjectionConfig::default(),
        )
        .unwrap();

        assert!(projected.is_none());
    }

    #[test]
    fn compositing_uses_back_to_front_order() {
        let far = ProjectedGaussian {
            center: Vec2::ZERO,
            radius_pixels: 10.0,
            depth: 2.0,
            color: ColorRgb::new(0.0, 0.0, 1.0),
            opacity: 1.0,
        };
        let near = ProjectedGaussian {
            center: Vec2::ZERO,
            radius_pixels: 10.0,
            depth: 1.0,
            color: ColorRgb::new(1.0, 0.0, 0.0),
            opacity: 0.5,
        };
        let pixel = composite_splats_at_pixel(&[far, near], Vec2::ZERO, ColorRgb::BLACK).unwrap();

        assert!(pixel.color.r > 0.49);
        assert!(pixel.color.b > 0.49);
        approx_eq(pixel.alpha, 1.0);
    }

    #[test]
    fn splat_scene_stats_transform_filter_and_downsample_are_deterministic() {
        let splat = |x: f32, opacity_logit: f32| GaussianSplat3d {
            mean: Vec3::new(x, 0.0, 1.0),
            scale_log: Vec3::new(0.0, 1.0_f32.ln(), 2.0_f32.ln()),
            rotation: Quaternion::IDENTITY,
            opacity_logit,
            sh: SphericalHarmonicsRgb::dc(ColorRgb::WHITE),
        };
        let scene = GaussianSplatScene {
            splats: vec![splat(0.0, 0.0), splat(2.0, 4.0), splat(4.0, -4.0)],
        };

        let stats = scene.stats().unwrap();
        assert_eq!(stats.count, 3);
        approx_eq(stats.min_scale.x, 1.0);
        approx_eq(stats.max_scale.z, 2.0);
        assert!(stats.mean_opacity > 0.3);

        let transformed = scene
            .transformed(SceneTransform3 {
                translation: Vec3::new(1.0, 0.0, 0.0),
                uniform_scale: 2.0,
            })
            .unwrap();
        approx_eq(transformed.splats[1].mean.x, 5.0);
        approx_eq(transformed.splats[1].scale().z, 4.0);

        let mut filtered = transformed.downsample_stride(2).unwrap();
        assert_eq!(filtered.splats.len(), 2);
        filtered.retain_opacity_at_least(0.5).unwrap();
        assert_eq!(filtered.splats.len(), 1);
    }
}
