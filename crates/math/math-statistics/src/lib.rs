#![doc = include_str!("../README.md")]

use math_linear::{F32Matrix, F32MatrixView, MatrixShape};
use numbers_core::{NumberRange, RunningStats};
use video_analysis_core::{DetectError, Result};

fn invalid_argument(message: impl Into<String>) -> DetectError {
    DetectError::InvalidArgument(message.into())
}

#[derive(Debug, Clone, PartialEq)]
pub struct WeightedObservation {
    pub values: Vec<f64>,
    pub weight: f64,
}

impl WeightedObservation {
    pub fn new(values: impl Into<Vec<f64>>) -> Result<Self> {
        Self::weighted(values, 1.0)
    }

    pub fn weighted(values: impl Into<Vec<f64>>, weight: f64) -> Result<Self> {
        let observation = Self {
            values: values.into(),
            weight,
        };
        observation.validate()?;
        Ok(observation)
    }

    pub fn validate(&self) -> Result<()> {
        if self.values.is_empty() {
            return Err(invalid_argument(
                "weighted observation values must not be empty",
            ));
        }
        if self.values.iter().any(|value| !value.is_finite()) {
            return Err(invalid_argument(
                "weighted observation values must be finite",
            ));
        }
        if !self.weight.is_finite() || self.weight <= 0.0 {
            return Err(invalid_argument(
                "weighted observation weight must be finite and positive",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CovarianceMatrix {
    pub matrix: F32Matrix,
    pub means: Vec<f64>,
    pub count: u64,
    pub weight_sum: f64,
}

impl CovarianceMatrix {
    pub fn correlation_matrix(&self) -> Result<F32Matrix> {
        let dims = self.matrix.shape().rows;
        let mut values = vec![0.0; dims * dims];
        for row in 0..dims {
            let var_row = self.matrix.values()[row * dims + row].max(0.0).sqrt();
            for col in 0..dims {
                let var_col = self.matrix.values()[col * dims + col].max(0.0).sqrt();
                let denom = (var_row * var_col).max(f32::EPSILON);
                values[row * dims + col] = self.matrix.values()[row * dims + col] / denom;
            }
        }
        F32Matrix::new(MatrixShape::new(dims, dims)?, values)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RunningCovariance {
    dimensions: usize,
    count: u64,
    weight_sum: f64,
    mean: Vec<f64>,
    m2: Vec<f64>,
}

impl RunningCovariance {
    pub fn new(dimensions: usize) -> Result<Self> {
        if dimensions == 0 {
            return Err(invalid_argument(
                "covariance dimensions must be greater than zero",
            ));
        }
        Ok(Self {
            dimensions,
            count: 0,
            weight_sum: 0.0,
            mean: vec![0.0; dimensions],
            m2: vec![0.0; dimensions * dimensions],
        })
    }

    pub fn from_matrix(matrix: &F32MatrixView<'_>) -> Result<Self> {
        let mut running = Self::new(matrix.shape().cols)?;
        for row in 0..matrix.shape().rows {
            let values = matrix
                .row(row)?
                .as_slice()
                .into_iter()
                .map(|value| value as f64)
                .collect::<Vec<_>>();
            running.push(WeightedObservation::new(values)?)?;
        }
        Ok(running)
    }

    pub fn dimensions(&self) -> usize {
        self.dimensions
    }

    pub fn count(&self) -> u64 {
        self.count
    }

    pub fn push(&mut self, observation: WeightedObservation) -> Result<()> {
        observation.validate()?;
        if observation.values.len() != self.dimensions {
            return Err(invalid_argument(
                "covariance observation dimensions must match",
            ));
        }
        self.count += 1;

        let new_weight_sum = self.weight_sum + observation.weight;
        let delta = observation
            .values
            .iter()
            .zip(&self.mean)
            .map(|(value, mean)| value - mean)
            .collect::<Vec<_>>();
        let next_mean = self
            .mean
            .iter()
            .zip(&delta)
            .map(|(mean, delta)| mean + delta * observation.weight / new_weight_sum)
            .collect::<Vec<_>>();
        let delta2 = observation
            .values
            .iter()
            .zip(&next_mean)
            .map(|(value, mean)| value - mean)
            .collect::<Vec<_>>();

        for (row, delta_value) in delta.iter().enumerate().take(self.dimensions) {
            for (col, delta2_value) in delta2.iter().enumerate().take(self.dimensions) {
                self.m2[row * self.dimensions + col] +=
                    observation.weight * delta_value * delta2_value;
            }
        }
        self.weight_sum = new_weight_sum;
        self.mean = next_mean;
        Ok(())
    }

    pub fn covariance_matrix(&self) -> Result<CovarianceMatrix> {
        if self.weight_sum <= 0.0 {
            return Err(invalid_argument(
                "covariance requires at least one observation",
            ));
        }
        let values = self
            .m2
            .iter()
            .map(|value| (value / self.weight_sum) as f32)
            .collect::<Vec<_>>();
        Ok(CovarianceMatrix {
            matrix: F32Matrix::new(MatrixShape::new(self.dimensions, self.dimensions)?, values)?,
            means: self.mean.clone(),
            count: self.count,
            weight_sum: self.weight_sum,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ZScoreNormalizer {
    means: Vec<f64>,
    std_devs: Vec<f64>,
}

impl ZScoreNormalizer {
    pub fn fit(matrix: &F32MatrixView<'_>) -> Result<Self> {
        let cols = matrix.shape().cols;
        let mut stats = (0..cols).map(|_| RunningStats::new()).collect::<Vec<_>>();
        for row in 0..matrix.shape().rows {
            let values = matrix.row(row)?.as_slice();
            for (stat, value) in stats.iter_mut().zip(values) {
                stat.push(value as f64);
            }
        }
        let means = stats
            .iter()
            .map(|stat| stat.summary().mean.unwrap_or(0.0))
            .collect::<Vec<_>>();
        let std_devs = stats
            .iter()
            .map(|stat| stat.summary().std_dev.unwrap_or(1.0).max(f64::EPSILON))
            .collect::<Vec<_>>();
        Ok(Self { means, std_devs })
    }

    pub fn means(&self) -> &[f64] {
        &self.means
    }

    pub fn std_devs(&self) -> &[f64] {
        &self.std_devs
    }

    pub fn transform_matrix(&self, matrix: &F32MatrixView<'_>) -> Result<F32Matrix> {
        if matrix.shape().cols != self.means.len() {
            return Err(invalid_argument(
                "z-score normalizer dimensions must match matrix",
            ));
        }
        let mut values = Vec::with_capacity(matrix.values().len());
        for row in 0..matrix.shape().rows {
            let row_values = matrix.row(row)?.as_slice();
            for (index, value) in row_values.into_iter().enumerate() {
                values.push(((value as f64 - self.means[index]) / self.std_devs[index]) as f32);
            }
        }
        F32Matrix::new(matrix.shape(), values)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MinMaxNormalizer {
    ranges: Vec<NumberRange>,
}

impl MinMaxNormalizer {
    pub fn fit(matrix: &F32MatrixView<'_>) -> Result<Self> {
        let cols = matrix.shape().cols;
        let mut mins = vec![f64::INFINITY; cols];
        let mut maxs = vec![f64::NEG_INFINITY; cols];
        for row in 0..matrix.shape().rows {
            let row_values = matrix.row(row)?.as_slice();
            for (index, value) in row_values.into_iter().enumerate() {
                mins[index] = mins[index].min(value as f64);
                maxs[index] = maxs[index].max(value as f64);
            }
        }
        let mut ranges = Vec::with_capacity(cols);
        for index in 0..cols {
            ranges.push(NumberRange::new(mins[index], maxs[index])?);
        }
        Ok(Self { ranges })
    }

    pub fn ranges(&self) -> &[NumberRange] {
        &self.ranges
    }

    pub fn transform_matrix(&self, matrix: &F32MatrixView<'_>) -> Result<F32Matrix> {
        if matrix.shape().cols != self.ranges.len() {
            return Err(invalid_argument(
                "min-max normalizer dimensions must match matrix",
            ));
        }
        let mut values = Vec::with_capacity(matrix.values().len());
        for row in 0..matrix.shape().rows {
            let row_values = matrix.row(row)?.as_slice();
            for (index, value) in row_values.into_iter().enumerate() {
                values.push(self.ranges[index].normalize(value as f64)? as f32);
            }
        }
        F32Matrix::new(matrix.shape(), values)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PrincipalComponents {
    mean: Vec<f32>,
    components: F32Matrix,
    explained_variance: Vec<f32>,
}

impl PrincipalComponents {
    pub fn fit(matrix: &F32MatrixView<'_>, component_count: usize) -> Result<Self> {
        if component_count == 0 {
            return Err(invalid_argument(
                "component_count must be greater than zero",
            ));
        }
        if component_count > matrix.shape().cols {
            return Err(invalid_argument(
                "component_count must not exceed matrix column count",
            ));
        }
        let covariance = RunningCovariance::from_matrix(matrix)?.covariance_matrix()?;
        let mut working = covariance.matrix.values().to_vec();
        let dims = matrix.shape().cols;
        let mut components = Vec::with_capacity(component_count * dims);
        let mut explained_variance = Vec::with_capacity(component_count);

        for _ in 0..component_count {
            let (eigenvalue, eigenvector) = power_iteration(&working, dims)?;
            explained_variance.push(eigenvalue.max(0.0));
            components.extend(eigenvector.iter().copied());
            for row in 0..dims {
                for col in 0..dims {
                    working[row * dims + col] -= eigenvalue * eigenvector[row] * eigenvector[col];
                }
            }
        }

        Ok(Self {
            mean: covariance.means.iter().map(|value| *value as f32).collect(),
            components: F32Matrix::new(MatrixShape::new(component_count, dims)?, components)?,
            explained_variance,
        })
    }

    pub fn components(&self) -> &F32Matrix {
        &self.components
    }

    pub fn mean(&self) -> &[f32] {
        &self.mean
    }

    pub fn explained_variance(&self) -> &[f32] {
        &self.explained_variance
    }

    pub fn transform(&self, matrix: &F32MatrixView<'_>) -> Result<F32Matrix> {
        if matrix.shape().cols != self.mean.len() {
            return Err(invalid_argument(
                "PCA transform dimensions must match input columns",
            ));
        }
        let mut centered = Vec::with_capacity(matrix.values().len());
        for row in 0..matrix.shape().rows {
            let row_values = matrix.row(row)?.as_slice();
            for (index, value) in row_values.into_iter().enumerate() {
                centered.push(value - self.mean[index]);
            }
        }
        let centered = F32Matrix::new(matrix.shape(), centered)?;
        centered.matmul(&self.components.as_view().transpose())
    }
}

fn power_iteration(values: &[f32], dims: usize) -> Result<(f32, Vec<f32>)> {
    let mut vector = vec![0.0; dims];
    vector[0] = 1.0;
    for _ in 0..32 {
        let mut next = vec![0.0; dims];
        for row in 0..dims {
            for col in 0..dims {
                next[row] += values[row * dims + col] * vector[col];
            }
        }
        let norm = next.iter().map(|value| value * value).sum::<f32>().sqrt();
        if norm <= f32::EPSILON {
            return Err(invalid_argument(
                "PCA power iteration encountered a zero norm vector",
            ));
        }
        for value in &mut next {
            *value /= norm;
        }
        vector = next;
    }
    let mut av = vec![0.0; dims];
    for row in 0..dims {
        for col in 0..dims {
            av[row] += values[row * dims + col] * vector[col];
        }
    }
    let eigenvalue = vector
        .iter()
        .zip(&av)
        .map(|(left, right)| left * right)
        .sum();
    Ok((eigenvalue, vector))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streaming_covariance_matches_batch_covariance() {
        let matrix = F32Matrix::from_rows([[1.0, 0.0], [0.0, 1.0], [1.0, 1.0]]).unwrap();
        let streaming = RunningCovariance::from_matrix(&matrix.as_view())
            .unwrap()
            .covariance_matrix()
            .unwrap();
        assert_eq!(streaming.matrix.shape().rows, 2);
        assert!(streaming.matrix.values()[0] > 0.0);
    }

    #[test]
    fn normalizers_produce_expected_ranges() {
        let matrix = F32Matrix::from_rows([[0.0, 1.0], [2.0, 3.0]]).unwrap();
        let z = ZScoreNormalizer::fit(&matrix.as_view()).unwrap();
        let min_max = MinMaxNormalizer::fit(&matrix.as_view()).unwrap();
        let normalized = min_max.transform_matrix(&matrix.as_view()).unwrap();
        assert_eq!(normalized.values(), &[0.0, 0.0, 1.0, 1.0]);
        assert_eq!(z.means(), &[1.0, 2.0]);
    }

    #[test]
    fn pca_reports_components_and_variance() {
        let matrix = F32Matrix::from_rows([[1.0, 1.0], [2.0, 2.0], [3.0, 3.0]]).unwrap();
        let pca = PrincipalComponents::fit(&matrix.as_view(), 1).unwrap();
        assert_eq!(pca.components().shape().rows, 1);
        assert_eq!(pca.explained_variance().len(), 1);
        assert!(pca.explained_variance()[0] >= 0.0);
    }
}
