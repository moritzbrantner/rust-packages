#![doc = include_str!("../README.md")]

pub mod surface;
use video_analysis_core::{DetectError, Result};

#[derive(Debug, Clone, PartialEq)]
/// Data type for dense vector.
pub struct DenseVector {
    values: Vec<f32>,
}

impl DenseVector {
    /// Creates a new value.
    pub fn new(values: impl Into<Vec<f32>>) -> Result<Self> {
        let vector = Self {
            values: values.into(),
        };
        vector.validate()?;
        Ok(vector)
    }

    /// Borrows this value as a slice.
    pub fn as_slice(&self) -> &[f32] {
        &self.values
    }

    /// Consumes this value into a values.
    pub fn into_values(self) -> Vec<f32> {
        self.values
    }

    /// Returns dimensions.
    pub fn dimensions(&self) -> usize {
        self.values.len()
    }

    /// Validates this value.
    pub fn validate(&self) -> Result<()> {
        if self.values.is_empty() {
            return Err(invalid_argument("vector must not be empty"));
        }
        if self.values.iter().any(|value| !value.is_finite()) {
            return Err(invalid_argument("vector components must be finite"));
        }
        Ok(())
    }

    /// Returns l2 normalized.
    pub fn l2_normalized(&self) -> Result<Self> {
        let norm = l2_norm(self.as_slice())?;
        if norm <= f32::EPSILON {
            return Err(invalid_argument("vector norm must be greater than zero"));
        }
        Self::new(
            self.values
                .iter()
                .map(|value| value / norm)
                .collect::<Vec<_>>(),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Variants describing vector metric.
pub enum VectorMetric {
    /// The cosine variant.
    Cosine,
    /// The euclidean variant.
    Euclidean,
    /// The manhattan variant.
    Manhattan,
    /// The dot variant.
    Dot,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for vector stats.
pub struct VectorStats {
    /// The dimensions value.
    pub dimensions: usize,
    /// The mean value.
    pub mean: Vec<f32>,
    /// The min value.
    pub min: Vec<f32>,
    /// The max value.
    pub max: Vec<f32>,
}

/// Returns dot.
pub fn dot(left: &[f32], right: &[f32]) -> Result<f32> {
    validate_pair(left, right)?;
    Ok(left
        .iter()
        .zip(right)
        .map(|(left, right)| left * right)
        .sum())
}

/// Returns l2 norm.
pub fn l2_norm(values: &[f32]) -> Result<f32> {
    validate_slice(values, "vector")?;
    Ok(values.iter().map(|value| value * value).sum::<f32>().sqrt())
}

/// Returns cosine similarity.
pub fn cosine_similarity(left: &[f32], right: &[f32]) -> Result<f32> {
    let numerator = dot(left, right)?;
    let left_norm = l2_norm(left)?;
    let right_norm = l2_norm(right)?;
    if left_norm <= f32::EPSILON || right_norm <= f32::EPSILON {
        return Err(invalid_argument(
            "cosine similarity requires non-zero vectors",
        ));
    }
    Ok(numerator / (left_norm * right_norm))
}

/// Returns euclidean distance.
pub fn euclidean_distance(left: &[f32], right: &[f32]) -> Result<f32> {
    validate_pair(left, right)?;
    Ok(left
        .iter()
        .zip(right)
        .map(|(left, right)| {
            let diff = left - right;
            diff * diff
        })
        .sum::<f32>()
        .sqrt())
}

/// Returns manhattan distance.
pub fn manhattan_distance(left: &[f32], right: &[f32]) -> Result<f32> {
    validate_pair(left, right)?;
    Ok(left
        .iter()
        .zip(right)
        .map(|(left, right)| (left - right).abs())
        .sum())
}

/// Returns mean vector.
pub fn mean_vector(vectors: &[DenseVector]) -> Result<DenseVector> {
    let dimensions = validate_vector_set(vectors)?;
    let mut mean = vec![0.0_f32; dimensions];
    for vector in vectors {
        for (index, value) in vector.as_slice().iter().enumerate() {
            mean[index] += value;
        }
    }
    for value in &mut mean {
        *value /= vectors.len() as f32;
    }
    DenseVector::new(mean)
}

/// Returns vector stats.
pub fn vector_stats(vectors: &[DenseVector]) -> Result<VectorStats> {
    let dimensions = validate_vector_set(vectors)?;
    let mut mean = vec![0.0_f32; dimensions];
    let mut min = vectors[0].as_slice().to_vec();
    let mut max = vectors[0].as_slice().to_vec();
    for vector in vectors {
        for (index, value) in vector.as_slice().iter().enumerate() {
            mean[index] += value;
            min[index] = min[index].min(*value);
            max[index] = max[index].max(*value);
        }
    }
    for value in &mut mean {
        *value /= vectors.len() as f32;
    }
    Ok(VectorStats {
        dimensions,
        mean,
        min,
        max,
    })
}

/// Returns metric distance.
pub fn metric_distance(metric: VectorMetric, left: &[f32], right: &[f32]) -> Result<f32> {
    match metric {
        VectorMetric::Cosine => Ok(1.0 - cosine_similarity(left, right)?),
        VectorMetric::Euclidean => euclidean_distance(left, right),
        VectorMetric::Manhattan => manhattan_distance(left, right),
        VectorMetric::Dot => Ok(-dot(left, right)?),
    }
}

fn validate_vector_set(vectors: &[DenseVector]) -> Result<usize> {
    if vectors.is_empty() {
        return Err(invalid_argument("vector set must not be empty"));
    }
    let dimensions = vectors[0].dimensions();
    for vector in vectors {
        if vector.dimensions() != dimensions {
            return Err(invalid_argument(
                "all vectors must have the same dimensions",
            ));
        }
        vector.validate()?;
    }
    Ok(dimensions)
}

fn validate_pair(left: &[f32], right: &[f32]) -> Result<()> {
    validate_slice(left, "left vector")?;
    validate_slice(right, "right vector")?;
    if left.len() != right.len() {
        return Err(invalid_argument("vectors must have the same dimensions"));
    }
    Ok(())
}

fn validate_slice(values: &[f32], name: &str) -> Result<()> {
    if values.is_empty() {
        return Err(invalid_argument(format!("{name} must not be empty")));
    }
    if values.iter().any(|value| !value.is_finite()) {
        return Err(invalid_argument(format!(
            "{name} components must be finite"
        )));
    }
    Ok(())
}

fn invalid_argument(message: impl Into<String>) -> DetectError {
    DetectError::InvalidArgument(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_cosine_similarity() {
        let similarity = cosine_similarity(&[1.0, 0.0], &[0.0, 1.0]).unwrap();
        assert_eq!(similarity, 0.0);
    }

    #[test]
    fn computes_mean_vector() {
        let mean = mean_vector(&[
            DenseVector::new([1.0, 3.0]).unwrap(),
            DenseVector::new([3.0, 5.0]).unwrap(),
        ])
        .unwrap();
        assert_eq!(mean.as_slice(), &[2.0, 4.0]);
    }
}
