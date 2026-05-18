#![doc = include_str!("../README.md")]

use video_analysis_core::{BoundingBox, DetectError, Result};

fn invalid_argument(message: impl Into<String>) -> DetectError {
    DetectError::InvalidArgument(message.into())
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Data type for point2f.
pub struct Point2f {
    /// The x value.
    pub x: f32,
    /// The y value.
    pub y: f32,
}

impl Point2f {
    /// Creates a new value.
    pub fn new(x: f32, y: f32) -> Result<Self> {
        let point = Self { x, y };
        point.validate()?;
        Ok(point)
    }

    /// Validates this value.
    pub fn validate(self) -> Result<()> {
        if !self.x.is_finite() || !self.y.is_finite() {
            return Err(invalid_argument("2D point coordinates must be finite"));
        }
        Ok(())
    }

    /// Returns translate.
    pub fn translate(self, delta: Vector2f) -> Result<Self> {
        delta.validate()?;
        Self::new(self.x + delta.x, self.y + delta.y)
    }

    /// Converts this value to normalized.
    pub fn to_normalized(self, size: Size2u) -> Result<NormalizedPoint2> {
        size.validate()?;
        NormalizedPoint2::new(self.x / size.width as f32, self.y / size.height as f32)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Data type for point2i.
pub struct Point2i {
    /// The x value.
    pub x: i32,
    /// The y value.
    pub y: i32,
}

impl Point2i {
    /// Creates a new value.
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Data type for vector2f.
pub struct Vector2f {
    /// The x value.
    pub x: f32,
    /// The y value.
    pub y: f32,
}

impl Vector2f {
    /// Creates a new value.
    pub fn new(x: f32, y: f32) -> Result<Self> {
        let vector = Self { x, y };
        vector.validate()?;
        Ok(vector)
    }

    /// Validates this value.
    pub fn validate(self) -> Result<()> {
        if !self.x.is_finite() || !self.y.is_finite() {
            return Err(invalid_argument("2D vector components must be finite"));
        }
        Ok(())
    }

    /// Returns length.
    pub fn length(self) -> Result<f32> {
        self.validate()?;
        Ok((self.x * self.x + self.y * self.y).sqrt())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Data type for size2u.
pub struct Size2u {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

impl Size2u {
    /// Creates a new value.
    pub fn new(width: u32, height: u32) -> Result<Self> {
        let size = Self { width, height };
        size.validate()?;
        Ok(size)
    }

    /// Validates this value.
    pub fn validate(self) -> Result<()> {
        if self.width == 0 || self.height == 0 {
            return Err(DetectError::InvalidDimensions {
                width: self.width,
                height: self.height,
            });
        }
        Ok(())
    }

    /// Returns area.
    pub fn area(self) -> Result<u64> {
        self.validate()?;
        Ok(self.width as u64 * self.height as u64)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Data type for rect u32.
pub struct RectU32 {
    /// The x value.
    pub x: u32,
    /// The y value.
    pub y: u32,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

impl RectU32 {
    /// Creates a new value.
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Result<Self> {
        let rect = Self {
            x,
            y,
            width,
            height,
        };
        rect.validate()?;
        Ok(rect)
    }

    /// Validates this value.
    pub fn validate(self) -> Result<()> {
        if self.width == 0 || self.height == 0 {
            return Err(DetectError::InvalidDimensions {
                width: self.width,
                height: self.height,
            });
        }
        let _ = self.max_x()?;
        let _ = self.max_y()?;
        Ok(())
    }

    /// Returns max x.
    pub fn max_x(self) -> Result<u32> {
        self.x
            .checked_add(self.width)
            .ok_or_else(|| invalid_argument("rectangle x range overflows"))
    }

    /// Returns max y.
    pub fn max_y(self) -> Result<u32> {
        self.y
            .checked_add(self.height)
            .ok_or_else(|| invalid_argument("rectangle y range overflows"))
    }

    /// Returns contains.
    pub fn contains(self, other: Self) -> Result<bool> {
        self.validate()?;
        other.validate()?;
        Ok(self.x <= other.x
            && self.y <= other.y
            && self.max_x()? >= other.max_x()?
            && self.max_y()? >= other.max_y()?)
    }

    /// Returns contains point.
    pub fn contains_point(self, x: u32, y: u32) -> Result<bool> {
        self.validate()?;
        Ok(x >= self.x && x < self.max_x()? && y >= self.y && y < self.max_y()?)
    }

    /// Returns intersects.
    pub fn intersects(self, other: Self) -> Result<bool> {
        self.validate()?;
        other.validate()?;
        Ok(self.x < other.max_x()?
            && other.x < self.max_x()?
            && self.y < other.max_y()?
            && other.y < self.max_y()?)
    }

    /// Returns intersection.
    pub fn intersection(self, other: Self) -> Result<Option<Self>> {
        if !self.intersects(other)? {
            return Ok(None);
        }
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let max_x = self.max_x()?.min(other.max_x()?);
        let max_y = self.max_y()?.min(other.max_y()?);
        Self::new(x, y, max_x - x, max_y - y).map(Some)
    }

    /// Returns union.
    pub fn union(self, other: Self) -> Result<Self> {
        self.validate()?;
        other.validate()?;
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        let max_x = self.max_x()?.max(other.max_x()?);
        let max_y = self.max_y()?.max(other.max_y()?);
        Self::new(x, y, max_x - x, max_y - y)
    }

    /// Returns clamp to.
    pub fn clamp_to(self, bounds: Self) -> Result<Option<Self>> {
        self.intersection(bounds)
    }

    /// Returns translate.
    pub fn translate(self, dx: i32, dy: i32) -> Result<Self> {
        self.validate()?;
        let x = self.x as i64 + dx as i64;
        let y = self.y as i64 + dy as i64;
        if x < 0 || y < 0 || x > u32::MAX as i64 || y > u32::MAX as i64 {
            return Err(invalid_argument(
                "translated rectangle is out of u32 bounds",
            ));
        }
        Self::new(x as u32, y as u32, self.width, self.height)
    }

    /// Returns scale.
    pub fn scale(self, factor: f32) -> Result<Self> {
        if !factor.is_finite() || factor <= 0.0 {
            return Err(invalid_argument("scale factor must be finite and positive"));
        }
        let x = (self.x as f32 * factor).round();
        let y = (self.y as f32 * factor).round();
        let width = (self.width as f32 * factor).round();
        let height = (self.height as f32 * factor).round();
        if [x, y, width, height]
            .iter()
            .any(|value| *value < 0.0 || *value > u32::MAX as f32)
        {
            return Err(invalid_argument("scaled rectangle is out of u32 bounds"));
        }
        Self::new(x as u32, y as u32, width as u32, height as u32)
    }

    /// Returns inflate.
    pub fn inflate(self, dx: i32, dy: i32) -> Result<Self> {
        self.validate()?;
        let x = self.x as i64 - dx as i64;
        let y = self.y as i64 - dy as i64;
        let width = self.width as i64 + 2 * dx as i64;
        let height = self.height as i64 + 2 * dy as i64;
        if x < 0 || y < 0 || width <= 0 || height <= 0 {
            return Err(invalid_argument(
                "inflated rectangle must stay within positive bounds",
            ));
        }
        if x > u32::MAX as i64
            || y > u32::MAX as i64
            || width > u32::MAX as i64
            || height > u32::MAX as i64
        {
            return Err(invalid_argument("inflated rectangle exceeds u32 bounds"));
        }
        Self::new(x as u32, y as u32, width as u32, height as u32)
    }

    /// Returns center f32.
    pub fn center_f32(self) -> Point2f {
        Point2f {
            x: self.x as f32 + self.width as f32 / 2.0,
            y: self.y as f32 + self.height as f32 / 2.0,
        }
    }

    /// Returns area.
    pub fn area(self) -> Result<u64> {
        self.validate()?;
        Ok(self.width as u64 * self.height as u64)
    }

    /// Returns size.
    pub fn size(self) -> Size2u {
        Size2u {
            width: self.width,
            height: self.height,
        }
    }
}

impl From<BoundingBox> for RectU32 {
    fn from(value: BoundingBox) -> Self {
        Self {
            x: value.x,
            y: value.y,
            width: value.width,
            height: value.height,
        }
    }
}

impl TryFrom<RectU32> for BoundingBox {
    type Error = DetectError;

    fn try_from(value: RectU32) -> Result<Self> {
        BoundingBox::new(value.x, value.y, value.width, value.height)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Data type for rect f32.
pub struct RectF32 {
    /// The x value.
    pub x: f32,
    /// The y value.
    pub y: f32,
    /// Width in pixels.
    pub width: f32,
    /// Height in pixels.
    pub height: f32,
}

impl RectF32 {
    /// Creates a new value.
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Result<Self> {
        let rect = Self {
            x,
            y,
            width,
            height,
        };
        rect.validate()?;
        Ok(rect)
    }

    /// Validates this value.
    pub fn validate(self) -> Result<()> {
        if [self.x, self.y, self.width, self.height]
            .iter()
            .any(|value| !value.is_finite())
        {
            return Err(invalid_argument("rectangle values must be finite"));
        }
        if self.width <= 0.0 || self.height <= 0.0 {
            return Err(invalid_argument(
                "rectangle width and height must be greater than zero",
            ));
        }
        Ok(())
    }

    /// Returns max x.
    pub fn max_x(self) -> Result<f32> {
        self.validate()?;
        Ok(self.x + self.width)
    }

    /// Returns max y.
    pub fn max_y(self) -> Result<f32> {
        self.validate()?;
        Ok(self.y + self.height)
    }

    /// Returns contains point.
    pub fn contains_point(self, point: Point2f) -> Result<bool> {
        self.validate()?;
        point.validate()?;
        Ok(point.x >= self.x
            && point.x <= self.max_x()?
            && point.y >= self.y
            && point.y <= self.max_y()?)
    }

    /// Returns intersects.
    pub fn intersects(self, other: Self) -> Result<bool> {
        self.validate()?;
        other.validate()?;
        Ok(self.x < other.max_x()?
            && other.x < self.max_x()?
            && self.y < other.max_y()?
            && other.y < self.max_y()?)
    }

    /// Returns intersection.
    pub fn intersection(self, other: Self) -> Result<Option<Self>> {
        if !self.intersects(other)? {
            return Ok(None);
        }
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let max_x = self.max_x()?.min(other.max_x()?);
        let max_y = self.max_y()?.min(other.max_y()?);
        Self::new(x, y, max_x - x, max_y - y).map(Some)
    }

    /// Returns union.
    pub fn union(self, other: Self) -> Result<Self> {
        self.validate()?;
        other.validate()?;
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        let max_x = self.max_x()?.max(other.max_x()?);
        let max_y = self.max_y()?.max(other.max_y()?);
        Self::new(x, y, max_x - x, max_y - y)
    }

    /// Returns clamp to.
    pub fn clamp_to(self, bounds: Self) -> Result<Option<Self>> {
        self.intersection(bounds)
    }

    /// Returns translate.
    pub fn translate(self, delta: Vector2f) -> Result<Self> {
        delta.validate()?;
        Self::new(self.x + delta.x, self.y + delta.y, self.width, self.height)
    }

    /// Returns scale.
    pub fn scale(self, factor: f32) -> Result<Self> {
        if !factor.is_finite() || factor <= 0.0 {
            return Err(invalid_argument("scale factor must be finite and positive"));
        }
        Self::new(
            self.x * factor,
            self.y * factor,
            self.width * factor,
            self.height * factor,
        )
    }

    /// Returns inflate.
    pub fn inflate(self, dx: f32, dy: f32) -> Result<Self> {
        if !dx.is_finite() || !dy.is_finite() {
            return Err(invalid_argument("inflate deltas must be finite"));
        }
        Self::new(
            self.x - dx,
            self.y - dy,
            self.width + 2.0 * dx,
            self.height + 2.0 * dy,
        )
    }

    /// Returns center.
    pub fn center(self) -> Result<Point2f> {
        self.validate()?;
        Point2f::new(self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    /// Returns area.
    pub fn area(self) -> Result<f32> {
        self.validate()?;
        Ok(self.width * self.height)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Data type for normalized point2.
pub struct NormalizedPoint2 {
    /// The x value.
    pub x: f32,
    /// The y value.
    pub y: f32,
}

impl NormalizedPoint2 {
    /// Creates a new value.
    pub fn new(x: f32, y: f32) -> Result<Self> {
        let point = Self { x, y };
        point.validate()?;
        Ok(point)
    }

    /// Validates this value.
    pub fn validate(self) -> Result<()> {
        if !self.x.is_finite() || !self.y.is_finite() {
            return Err(invalid_argument("normalized coordinates must be finite"));
        }
        if !(0.0..=1.0).contains(&self.x) || !(0.0..=1.0).contains(&self.y) {
            return Err(invalid_argument(
                "normalized coordinates must be within the inclusive range [0, 1]",
            ));
        }
        Ok(())
    }

    /// Converts this value to pixel point.
    pub fn to_pixel_point(self, size: Size2u) -> Point2i {
        Point2i {
            x: (self.x * size.width as f32).round() as i32,
            y: (self.y * size.height as f32).round() as i32,
        }
    }

    /// Converts this value to pixel point f32.
    pub fn to_pixel_point_f32(self, size: Size2u) -> Point2f {
        Point2f {
            x: self.x * size.width as f32,
            y: self.y * size.height as f32,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Data type for affine2.
pub struct Affine2 {
    /// The m11 value.
    pub m11: f32,
    /// The m12 value.
    pub m12: f32,
    /// The m21 value.
    pub m21: f32,
    /// The m22 value.
    pub m22: f32,
    /// The tx value.
    pub tx: f32,
    /// The ty value.
    pub ty: f32,
}

impl Affine2 {
    /// Creates a new value.
    pub const fn identity() -> Self {
        Self {
            m11: 1.0,
            m12: 0.0,
            m21: 0.0,
            m22: 1.0,
            tx: 0.0,
            ty: 0.0,
        }
    }

    /// Creates a new value.
    pub const fn translation(tx: f32, ty: f32) -> Self {
        Self {
            tx,
            ty,
            ..Self::identity()
        }
    }

    /// Creates a new value.
    pub const fn scaling(sx: f32, sy: f32) -> Self {
        Self {
            m11: sx,
            m12: 0.0,
            m21: 0.0,
            m22: sy,
            tx: 0.0,
            ty: 0.0,
        }
    }

    /// Validates this value.
    pub fn validate(self) -> Result<()> {
        if [self.m11, self.m12, self.m21, self.m22, self.tx, self.ty]
            .iter()
            .any(|value| !value.is_finite())
        {
            return Err(invalid_argument("affine transform values must be finite"));
        }
        Ok(())
    }

    /// Returns determinant.
    pub fn determinant(self) -> Result<f32> {
        self.validate()?;
        Ok(self.m11 * self.m22 - self.m12 * self.m21)
    }

    /// Returns apply point.
    pub fn apply_point(self, point: Point2f) -> Point2f {
        Point2f {
            x: self.m11 * point.x + self.m12 * point.y + self.tx,
            y: self.m21 * point.x + self.m22 * point.y + self.ty,
        }
    }

    /// Returns invert.
    pub fn invert(self) -> Result<Self> {
        self.validate()?;
        let det = self.determinant()?;
        if det.abs() <= f32::EPSILON {
            return Err(invalid_argument("affine transform is not invertible"));
        }
        let inv_det = 1.0 / det;
        let m11 = self.m22 * inv_det;
        let m12 = -self.m12 * inv_det;
        let m21 = -self.m21 * inv_det;
        let m22 = self.m11 * inv_det;
        let tx = -(m11 * self.tx + m12 * self.ty);
        let ty = -(m21 * self.tx + m22 * self.ty);
        Ok(Self {
            m11,
            m12,
            m21,
            m22,
            tx,
            ty,
        })
    }

    /// Returns compose.
    pub fn compose(self, next: Self) -> Result<Self> {
        self.validate()?;
        next.validate()?;
        Ok(Self {
            m11: next.m11 * self.m11 + next.m12 * self.m21,
            m12: next.m11 * self.m12 + next.m12 * self.m22,
            m21: next.m21 * self.m11 + next.m22 * self.m21,
            m22: next.m21 * self.m12 + next.m22 * self.m22,
            tx: next.m11 * self.tx + next.m12 * self.ty + next.tx,
            ty: next.m21 * self.tx + next.m22 * self.ty + next.ty,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for polygon2f.
pub struct Polygon2f {
    points: Vec<Point2f>,
}

impl Polygon2f {
    /// Creates a new value.
    pub fn new(points: impl Into<Vec<Point2f>>) -> Result<Self> {
        let polygon = Self {
            points: points.into(),
        };
        polygon.validate()?;
        Ok(polygon)
    }

    /// Returns points.
    pub fn points(&self) -> &[Point2f] {
        &self.points
    }

    /// Validates this value.
    pub fn validate(&self) -> Result<()> {
        if self.points.len() < 3 {
            return Err(invalid_argument(
                "polygon must contain at least three points",
            ));
        }
        for point in &self.points {
            point.validate()?;
        }
        Ok(())
    }

    /// Returns bounds.
    pub fn bounds(&self) -> Result<Bounds2f> {
        self.validate()?;
        let mut min_x = self.points[0].x;
        let mut min_y = self.points[0].y;
        let mut max_x = self.points[0].x;
        let mut max_y = self.points[0].y;
        for point in &self.points[1..] {
            min_x = min_x.min(point.x);
            min_y = min_y.min(point.y);
            max_x = max_x.max(point.x);
            max_y = max_y.max(point.y);
        }
        Bounds2f::new(
            Point2f { x: min_x, y: min_y },
            Point2f { x: max_x, y: max_y },
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Data type for bounds2f.
pub struct Bounds2f {
    /// The min value.
    pub min: Point2f,
    /// The max value.
    pub max: Point2f,
}

impl Bounds2f {
    /// Creates a new value.
    pub fn new(min: Point2f, max: Point2f) -> Result<Self> {
        min.validate()?;
        max.validate()?;
        if min.x > max.x || min.y > max.y {
            return Err(invalid_argument("bounds min must not exceed max"));
        }
        Ok(Self { min, max })
    }

    /// Returns contains.
    pub fn contains(self, point: Point2f) -> Result<bool> {
        point.validate()?;
        Ok(point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y)
    }

    /// Returns union.
    pub fn union(self, other: Self) -> Result<Self> {
        Self::new(
            Point2f {
                x: self.min.x.min(other.min.x),
                y: self.min.y.min(other.min.y),
            },
            Point2f {
                x: self.max.x.max(other.max.x),
                y: self.max.y.max(other.max.y),
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_rectangles_and_non_finite_points() {
        assert!(RectU32::new(0, 0, 0, 2).is_err());
        assert!(RectF32::new(0.0, 0.0, f32::NAN, 1.0).is_err());
        assert!(Point2f::new(f32::INFINITY, 0.0).is_err());
    }

    #[test]
    fn computes_intersection_and_union() {
        let a = RectU32::new(0, 0, 10, 10).unwrap();
        let b = RectU32::new(5, 5, 10, 10).unwrap();
        assert_eq!(
            a.intersection(b).unwrap(),
            Some(RectU32::new(5, 5, 5, 5).unwrap())
        );
        assert_eq!(a.union(b).unwrap(), RectU32::new(0, 0, 15, 15).unwrap());
    }

    #[test]
    fn affine_round_trip_recovers_point() {
        let point = Point2f::new(4.0, 5.0).unwrap();
        let affine = Affine2::translation(3.0, -2.0)
            .compose(Affine2::scaling(2.0, 0.5))
            .unwrap();
        let restored = affine
            .invert()
            .unwrap()
            .apply_point(affine.apply_point(point));
        assert!((restored.x - point.x).abs() < 1.0e-5);
        assert!((restored.y - point.y).abs() < 1.0e-5);
    }

    #[test]
    fn converts_between_normalized_and_pixel_coordinates() {
        let point = NormalizedPoint2::new(0.25, 0.5).unwrap();
        let size = Size2u::new(200, 100).unwrap();
        assert_eq!(point.to_pixel_point(size), Point2i::new(50, 50));
        let normalized = Point2f::new(50.0, 50.0)
            .unwrap()
            .to_normalized(size)
            .unwrap();
        assert!((normalized.x - 0.25).abs() < 1.0e-6);
        assert!((normalized.y - 0.5).abs() < 1.0e-6);
    }
}
