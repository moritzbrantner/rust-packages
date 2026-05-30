use video_analysis_core::{DetectError, Result};

use crate::{Point3, Vector3};

pub(crate) fn validate_points(points: &[Point3]) -> Result<()> {
    if points.iter().any(|point| !point.is_finite()) {
        return Err(invalid_argument("points must be finite"));
    }
    Ok(())
}

pub(crate) fn validate_finite_vector(vector: Vector3, name: &str) -> Result<()> {
    if vector.is_finite() {
        Ok(())
    } else {
        Err(invalid_argument(format!(
            "{name} components must be finite"
        )))
    }
}

pub(crate) fn invalid_argument(message: impl Into<String>) -> DetectError {
    DetectError::InvalidArgument(message.into())
}
