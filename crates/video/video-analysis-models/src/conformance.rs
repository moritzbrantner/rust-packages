use video_analysis_core::{DetectError, Result};

use crate::RawPrediction;

#[derive(Debug, Clone, Copy, PartialEq)]
/// Data type for blue/green prediction test options.
pub struct BlueGreenPredictionTestOptions {
    /// Maximum allowed absolute score delta.
    pub max_score_delta: f32,
    /// Whether bounding boxes are compared.
    pub compare_regions: bool,
}

impl Default for BlueGreenPredictionTestOptions {
    fn default() -> Self {
        Self {
            max_score_delta: 1.0e-4,
            compare_regions: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for blue/green prediction test report.
pub struct BlueGreenPredictionTestReport {
    /// Number of predictions compared.
    pub compared_predictions: usize,
    /// Maximum observed absolute score delta.
    pub max_score_delta: f32,
}

/// Compares green and blue model predictions for runtime conformance tests.
pub fn compare_blue_green_predictions(
    green: &[RawPrediction],
    blue: &[RawPrediction],
    options: BlueGreenPredictionTestOptions,
) -> Result<BlueGreenPredictionTestReport> {
    if green.len() != blue.len() {
        return Err(DetectError::InvalidArgument(format!(
            "blue/green prediction counts differ: green={}, blue={}",
            green.len(),
            blue.len()
        )));
    }

    let mut max_score_delta = 0.0_f32;
    for (index, (green, blue)) in green.iter().zip(blue).enumerate() {
        if green.kind != blue.kind {
            return Err(blue_green_mismatch(index, "kind", &green.kind, &blue.kind));
        }
        if green.label != blue.label {
            return Err(blue_green_mismatch(
                index,
                "label",
                &green.label,
                &blue.label,
            ));
        }
        if green.text != blue.text {
            return Err(blue_green_mismatch(index, "text", &green.text, &blue.text));
        }
        match (green.score, blue.score) {
            (Some(green_score), Some(blue_score)) => {
                if !green_score.is_finite() || !blue_score.is_finite() {
                    return Err(DetectError::InvalidArgument(format!(
                        "blue/green prediction {index} has a non-finite score"
                    )));
                }
                let delta = (green_score - blue_score).abs();
                max_score_delta = max_score_delta.max(delta);
                if delta > options.max_score_delta {
                    return Err(DetectError::InvalidArgument(format!(
                        "blue/green prediction {index} score delta {delta} exceeds {}",
                        options.max_score_delta
                    )));
                }
            }
            (None, None) => {}
            _ => {
                return Err(blue_green_mismatch(
                    index,
                    "score",
                    &green.score,
                    &blue.score,
                ))
            }
        }
        if options.compare_regions && green.region != blue.region {
            return Err(blue_green_mismatch(
                index,
                "region",
                &green.region,
                &blue.region,
            ));
        }
    }

    Ok(BlueGreenPredictionTestReport {
        compared_predictions: green.len(),
        max_score_delta,
    })
}

fn blue_green_mismatch<T: std::fmt::Debug>(
    index: usize,
    field: &str,
    green: &T,
    blue: &T,
) -> DetectError {
    DetectError::InvalidArgument(format!(
        "blue/green prediction {index} {field} mismatch: green={green:?}, blue={blue:?}"
    ))
}
