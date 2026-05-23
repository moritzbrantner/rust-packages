#![doc = include_str!("../README.md")]

pub mod surface;
use video_analysis_core::{DetectError, Result};

#[derive(Debug, Clone, PartialEq)]
/// Data type for number summary.
pub struct NumberSummary {
    /// Number of items represented by this value.
    pub count: u64,
    /// The finite count value.
    pub finite_count: u64,
    /// The non finite count value.
    pub non_finite_count: u64,
    /// The min value.
    pub min: Option<f64>,
    /// The max value.
    pub max: Option<f64>,
    /// The sum value.
    pub sum: Option<f64>,
    /// The mean value.
    pub mean: Option<f64>,
    /// The variance value.
    pub variance: Option<f64>,
    /// The std dev value.
    pub std_dev: Option<f64>,
    /// The weight sum value.
    pub weight_sum: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Data type for number range.
pub struct NumberRange {
    /// The min value.
    pub min: f64,
    /// The max value.
    pub max: f64,
}

impl NumberRange {
    /// Creates a new value.
    pub fn new(min: f64, max: f64) -> Result<Self> {
        if !min.is_finite() || !max.is_finite() {
            return Err(invalid_argument("number range bounds must be finite"));
        }
        if min > max {
            return Err(invalid_argument("number range min must not exceed max"));
        }
        Ok(Self { min, max })
    }

    /// Clamps a value into this range.
    pub fn clamp(self, value: f64) -> Result<f64> {
        if !value.is_finite() {
            return Err(invalid_argument("range value must be finite"));
        }
        Ok(value.clamp(self.min, self.max))
    }

    /// Normalizes this value.
    pub fn normalize(self, value: f64) -> Result<f64> {
        let value = self.clamp(value)?;
        if self.min == self.max {
            return Ok(0.0);
        }
        Ok((value - self.min) / (self.max - self.min))
    }

    /// Denormalizes this value.
    pub fn denormalize(self, value: f64) -> Result<f64> {
        if !value.is_finite() {
            return Err(invalid_argument("normalized value must be finite"));
        }
        if self.min == self.max {
            return Ok(self.min);
        }
        Ok(self.min + value * (self.max - self.min))
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for quartile summary.
pub struct QuartileSummary {
    /// The first quartile value.
    pub q1: f64,
    /// The median value.
    pub median: f64,
    /// The third quartile value.
    pub q3: f64,
    /// The interquartile range value.
    pub iqr: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Data type for histogram config.
pub struct HistogramConfig {
    /// The bins value.
    pub bins: usize,
    /// The range value.
    pub range: Option<NumberRange>,
}

impl HistogramConfig {
    /// Creates a new value.
    pub fn new(bins: usize) -> Result<Self> {
        let config = Self { bins, range: None };
        config.validate()?;
        Ok(config)
    }

    /// Returns this value with range.
    pub fn with_range(mut self, range: NumberRange) -> Self {
        self.range = Some(range);
        self
    }

    /// Validates this value.
    pub fn validate(&self) -> Result<()> {
        if self.bins == 0 {
            return Err(invalid_argument("histogram bin count must be positive"));
        }
        if let Some(range) = self.range {
            NumberRange::new(range.min, range.max)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for histogram bin.
pub struct HistogramBin {
    /// The start value.
    pub start: f64,
    /// The end value.
    pub end: f64,
    /// Number of items represented by this value.
    pub count: u64,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for histogram.
pub struct Histogram {
    /// The bins value.
    pub bins: Vec<HistogramBin>,
    /// Number of items represented by this value.
    pub count: u64,
    /// The min value.
    pub min: f64,
    /// The max value.
    pub max: f64,
}

#[derive(Debug, Clone, Default, PartialEq)]
/// Data type for running stats.
pub struct RunningStats {
    count: u64,
    finite_count: u64,
    non_finite_count: u64,
    min: Option<f64>,
    max: Option<f64>,
    sum: f64,
    weight_sum: f64,
    mean: f64,
    m2: f64,
}

impl RunningStats {
    /// Creates a new value.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds push to this value.
    pub fn push(&mut self, value: f64) {
        self.push_weighted_internal(value, 1.0, false)
            .expect("unit weight is valid");
    }

    /// Adds push weighted to this value.
    pub fn push_weighted(&mut self, value: f64, weight: f64) -> Result<()> {
        self.push_weighted_internal(value, weight, true)
    }

    /// Returns extend.
    pub fn extend(&mut self, values: impl IntoIterator<Item = f64>) {
        for value in values {
            self.push(value);
        }
    }

    /// Returns merge.
    pub fn merge(&mut self, other: &Self) {
        self.count += other.count;
        self.finite_count += other.finite_count;
        self.non_finite_count += other.non_finite_count;
        self.min = merge_lower_bound(self.min, other.min);
        self.max = merge_upper_bound(self.max, other.max);
        self.sum += other.sum;

        if other.weight_sum == 0.0 {
            return;
        }
        if self.weight_sum == 0.0 {
            self.weight_sum = other.weight_sum;
            self.mean = other.mean;
            self.m2 = other.m2;
            return;
        }

        let total_weight = self.weight_sum + other.weight_sum;
        let delta = other.mean - self.mean;
        self.m2 += other.m2 + delta * delta * self.weight_sum * other.weight_sum / total_weight;
        self.mean += delta * other.weight_sum / total_weight;
        self.weight_sum = total_weight;
    }

    /// Returns summary.
    pub fn summary(&self) -> NumberSummary {
        let variance = (self.weight_sum > 0.0).then_some(self.m2 / self.weight_sum);
        NumberSummary {
            count: self.count,
            finite_count: self.finite_count,
            non_finite_count: self.non_finite_count,
            min: self.min,
            max: self.max,
            sum: (self.weight_sum > 0.0).then_some(self.sum),
            mean: (self.weight_sum > 0.0).then_some(self.mean),
            variance,
            std_dev: variance.map(f64::sqrt),
            weight_sum: self.weight_sum,
        }
    }

    fn push_weighted_internal(
        &mut self,
        value: f64,
        weight: f64,
        validate_weight: bool,
    ) -> Result<()> {
        if validate_weight && (!weight.is_finite() || weight <= 0.0) {
            return Err(invalid_argument("stat weight must be finite and positive"));
        }

        self.count += 1;
        if !value.is_finite() {
            self.non_finite_count += 1;
            return Ok(());
        }

        self.record_finite_value(value, weight);
        Ok(())
    }

    fn record_finite_value(&mut self, value: f64, weight: f64) {
        self.finite_count += 1;
        self.min = Some(self.min.map_or(value, |min| min.min(value)));
        self.max = Some(self.max.map_or(value, |max| max.max(value)));

        let previous_weight = self.weight_sum;
        let next_weight = previous_weight + weight;
        let delta = value - self.mean;
        let next_mean = if previous_weight == 0.0 {
            value
        } else {
            self.mean + delta * weight / next_weight
        };
        let delta2 = value - next_mean;

        self.sum += value * weight;
        self.weight_sum = next_weight;
        self.mean = next_mean;
        self.m2 += weight * delta * delta2;
    }
}

/// Returns summarize numbers.
pub fn summarize_numbers(values: &[f64]) -> NumberSummary {
    let mut stats = RunningStats::new();
    stats.extend(values.iter().copied());
    stats.summary()
}

/// Returns quantile.
pub fn quantile(values: &[f64], quantile: f64) -> Result<f64> {
    if !quantile.is_finite() || !(0.0..=1.0).contains(&quantile) {
        return Err(invalid_argument(
            "quantile must be finite and between 0 and 1",
        ));
    }

    let mut finite = collect_finite_values(values, "quantile")?;
    finite.sort_by(f64::total_cmp);
    let rank = quantile * (finite.len() - 1) as f64;
    let lower = rank.floor() as usize;
    let upper = rank.ceil() as usize;
    if lower == upper {
        return Ok(finite[lower]);
    }
    let fraction = rank - lower as f64;
    Ok(finite[lower] + (finite[upper] - finite[lower]) * fraction)
}

/// Returns quartiles.
pub fn quartiles(values: &[f64]) -> Result<QuartileSummary> {
    let q1 = quantile(values, 0.25)?;
    let median = quantile(values, 0.5)?;
    let q3 = quantile(values, 0.75)?;
    Ok(QuartileSummary {
        q1,
        median,
        q3,
        iqr: q3 - q1,
    })
}

/// Returns histogram.
pub fn histogram(values: &[f64], config: HistogramConfig) -> Result<Histogram> {
    config.validate()?;

    let finite = collect_finite_values(values, "histogram")?;
    let range = config.range.unwrap_or(derive_range(&finite));
    let mut bins = build_histogram_bins(range, config.bins);

    for value in finite {
        if value < range.min || value > range.max {
            continue;
        }
        let bin_index = histogram_bin_index(value, range, config.bins);
        bins[bin_index.min(config.bins - 1)].count += 1;
    }

    Ok(Histogram {
        count: bins.iter().map(|bin| bin.count).sum(),
        bins,
        min: range.min,
        max: range.max,
    })
}

fn invalid_argument(message: impl Into<String>) -> DetectError {
    DetectError::InvalidArgument(message.into())
}

fn merge_lower_bound(left: Option<f64>, right: Option<f64>) -> Option<f64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.min(right)),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

fn merge_upper_bound(left: Option<f64>, right: Option<f64>) -> Option<f64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

fn collect_finite_values(values: &[f64], operation: &str) -> Result<Vec<f64>> {
    let finite = values
        .iter()
        .copied()
        .filter(|value| value.is_finite())
        .collect::<Vec<_>>();
    if finite.is_empty() {
        return Err(invalid_argument(format!(
            "{operation} requires at least one finite numeric value"
        )));
    }
    Ok(finite)
}

fn derive_range(values: &[f64]) -> NumberRange {
    NumberRange {
        min: *values
            .iter()
            .min_by(|left, right| left.total_cmp(right))
            .expect("finite values exist"),
        max: *values
            .iter()
            .max_by(|left, right| left.total_cmp(right))
            .expect("finite values exist"),
    }
}

fn build_histogram_bins(range: NumberRange, bins: usize) -> Vec<HistogramBin> {
    if range.min == range.max {
        return (0..bins)
            .map(|_| HistogramBin {
                start: range.min,
                end: range.max,
                count: 0,
            })
            .collect();
    }

    let width = (range.max - range.min) / bins as f64;
    (0..bins)
        .map(|index| {
            let start = range.min + index as f64 * width;
            let end = if index + 1 == bins {
                range.max
            } else {
                start + width
            };
            HistogramBin {
                start,
                end,
                count: 0,
            }
        })
        .collect()
}

fn histogram_bin_index(value: f64, range: NumberRange, bins: usize) -> usize {
    if range.min == range.max || value == range.max {
        bins - 1
    } else {
        (((value - range.min) / (range.max - range.min)) * bins as f64).floor() as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(left: f64, right: f64) {
        assert!((left - right).abs() < 1.0e-12, "{left} != {right}");
    }

    #[test]
    fn running_stats_track_finite_and_non_finite_values() {
        let mut stats = RunningStats::new();
        stats.push(1.0);
        stats.push(f64::INFINITY);
        stats.push_weighted(3.0, 2.0).unwrap();

        let summary = stats.summary();
        assert_eq!(summary.count, 3);
        assert_eq!(summary.finite_count, 2);
        assert_eq!(summary.non_finite_count, 1);
        assert_eq!(summary.weight_sum, 3.0);
        assert_eq!(summary.sum, Some(7.0));
        assert_close(summary.mean.unwrap(), 7.0 / 3.0);
    }

    #[test]
    fn running_stats_merge_preserves_weighted_summary() {
        let mut left = RunningStats::new();
        left.push(1.0);
        left.push_weighted(5.0, 2.0).unwrap();

        let mut right = RunningStats::new();
        right.push(f64::NAN);
        right.push_weighted(3.0, 3.0).unwrap();

        left.merge(&right);
        let summary = left.summary();
        assert_eq!(summary.count, 4);
        assert_eq!(summary.finite_count, 3);
        assert_eq!(summary.non_finite_count, 1);
        assert_eq!(summary.weight_sum, 6.0);
        assert_close(summary.mean.unwrap(), 20.0 / 6.0);
    }

    #[test]
    fn quantile_interpolates_even_sized_inputs() {
        assert_eq!(quantile(&[1.0, 2.0, 3.0, 4.0], 0.5).unwrap(), 2.5);
        assert_eq!(quartiles(&[1.0, 2.0, 3.0, 4.0]).unwrap().q1, 1.75);
    }

    #[test]
    fn histogram_places_max_value_in_last_bin() {
        let histogram = histogram(&[0.0, 0.5, 1.0], HistogramConfig::new(2).unwrap()).unwrap();
        assert_eq!(histogram.count, 3);
        assert_eq!(histogram.bins[0].count, 1);
        assert_eq!(histogram.bins[1].count, 2);
    }

    #[test]
    fn histogram_supports_degenerate_ranges() {
        let range = NumberRange::new(3.0, 3.0).unwrap();
        let histogram = histogram(
            &[3.0, 3.0, 3.0],
            HistogramConfig::new(4).unwrap().with_range(range),
        )
        .unwrap();
        assert_eq!(histogram.bins[3].count, 3);
    }

    #[test]
    fn rejects_invalid_inputs() {
        assert!(NumberRange::new(2.0, 1.0).is_err());
        assert!(HistogramConfig::new(0).is_err());
        assert!(quantile(&[], 0.5).is_err());
        assert!(quantile(&[1.0], 2.0).is_err());
        assert!(RunningStats::new().push_weighted(1.0, 0.0).is_err());
    }
}
