#![doc = include_str!("../README.md")]

use std::collections::BTreeMap;

use math_linear::{F32Matrix, MatrixShape};
use math_statistics::{CovarianceMatrix, PrincipalComponents, RunningCovariance};
use numbers_core::{NumberSummary, RunningStats};
use video_analysis_core::{DetectError, Result};

#[derive(Debug, Clone, PartialEq)]
pub struct DensePoint {
    pub id: Option<String>,
    pub coordinates: Vec<f64>,
    pub weight: f64,
    pub value: Option<f64>,
}

impl DensePoint {
    pub fn new(coordinates: impl Into<Vec<f64>>) -> Result<Self> {
        let point = Self {
            id: None,
            coordinates: coordinates.into(),
            weight: 1.0,
            value: None,
        };
        point.validate()?;
        Ok(point)
    }

    pub fn named(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn weighted(mut self, weight: f64) -> Result<Self> {
        self.weight = weight;
        self.validate()?;
        Ok(self)
    }

    pub fn valued(mut self, value: f64) -> Result<Self> {
        self.value = Some(value);
        self.validate()?;
        Ok(self)
    }

    pub fn dimensions(&self) -> usize {
        self.coordinates.len()
    }

    pub fn validate(&self) -> Result<()> {
        validate_coordinates(&self.coordinates, "point coordinates")?;
        if self.weight <= 0.0 || !self.weight.is_finite() {
            return Err(invalid_argument("point weight must be finite and positive"));
        }
        if self.value.is_some_and(|value| !value.is_finite()) {
            return Err(invalid_argument("point value must be finite"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DenseDataset {
    dimensions: Option<usize>,
    points: Vec<DensePoint>,
}

impl DenseDataset {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_points(points: impl IntoIterator<Item = DensePoint>) -> Result<Self> {
        let mut dataset = Self::new();
        dataset.extend(points)?;
        Ok(dataset)
    }

    pub fn push(&mut self, point: DensePoint) -> Result<()> {
        point.validate()?;
        match self.dimensions {
            Some(dimensions) if dimensions != point.dimensions() => {
                return Err(invalid_argument(
                    "dense dataset points must have the same dimensions",
                ));
            }
            None => self.dimensions = Some(point.dimensions()),
            _ => {}
        }
        self.points.push(point);
        Ok(())
    }

    pub fn extend(&mut self, points: impl IntoIterator<Item = DensePoint>) -> Result<()> {
        for point in points {
            self.push(point)?;
        }
        Ok(())
    }

    pub fn dimensions(&self) -> Option<usize> {
        self.dimensions
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    pub fn len(&self) -> usize {
        self.points.len()
    }

    pub fn points(&self) -> &[DensePoint] {
        &self.points
    }

    pub fn averages(&self) -> Result<DenseAverages> {
        dense_averages(&self.points)
    }

    pub fn summary(&self) -> Result<DenseSummary> {
        dense_summary(&self.points)
    }

    pub fn bounds(&self) -> Result<DenseBounds> {
        dense_bounds(&self.points)
    }

    pub fn buckets(&self, grid: &BucketGrid) -> Result<Vec<DenseBucket>> {
        bucket_points(&self.points, grid)
    }

    pub fn k_means(&self, config: KMeansConfig) -> Result<ClusterResult> {
        k_means(&self.points, config)
    }

    pub fn matrix(&self) -> Result<F32Matrix> {
        let dimensions = self
            .dimensions
            .ok_or_else(|| invalid_argument("dense dataset must not be empty"))?;
        let mut values = Vec::with_capacity(self.points.len() * dimensions);
        for point in &self.points {
            values.extend(point.coordinates.iter().map(|value| *value as f32));
        }
        F32Matrix::new(MatrixShape::new(self.points.len(), dimensions)?, values)
    }

    pub fn covariance_matrix(&self) -> Result<CovarianceMatrix> {
        let matrix = self.matrix()?;
        RunningCovariance::from_matrix(&matrix.as_view())?.covariance_matrix()
    }

    pub fn principal_components(&self, component_count: usize) -> Result<PrincipalComponents> {
        let matrix = self.matrix()?;
        PrincipalComponents::fit(&matrix.as_view(), component_count)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DenseAverages {
    pub count: u64,
    pub weight_sum: f64,
    pub coordinates: Vec<f64>,
    pub value_count: u64,
    pub value_weight_sum: f64,
    pub value: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DenseSummary {
    pub count: u64,
    pub dimensions: usize,
    pub weight_sum: f64,
    pub coordinate_stats: Vec<NumberSummary>,
    pub value_stats: Option<NumberSummary>,
    pub bounds: DenseBounds,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DenseBounds {
    pub min: Vec<f64>,
    pub max: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BucketGrid {
    pub origin: Vec<f64>,
    pub widths: Vec<f64>,
}

impl BucketGrid {
    pub fn new(origin: impl Into<Vec<f64>>, widths: impl Into<Vec<f64>>) -> Result<Self> {
        let grid = Self {
            origin: origin.into(),
            widths: widths.into(),
        };
        grid.validate()?;
        Ok(grid)
    }

    pub fn uniform(dimensions: usize, width: f64) -> Result<Self> {
        if dimensions == 0 {
            return Err(invalid_argument("bucket grid dimensions must be positive"));
        }
        Self::new(vec![0.0; dimensions], vec![width; dimensions])
    }

    pub fn dimensions(&self) -> usize {
        self.origin.len()
    }

    pub fn key_for(&self, point: &DensePoint) -> Result<BucketKey> {
        self.validate()?;
        point.validate()?;
        if point.dimensions() != self.dimensions() {
            return Err(invalid_argument(
                "point dimensions must match bucket grid dimensions",
            ));
        }

        let mut indices = Vec::with_capacity(point.dimensions());
        for ((coordinate, origin), width) in
            point.coordinates.iter().zip(&self.origin).zip(&self.widths)
        {
            let index = ((*coordinate - *origin) / *width).floor();
            if index < i64::MIN as f64 || index > i64::MAX as f64 {
                return Err(invalid_argument("bucket index is out of range"));
            }
            indices.push(index as i64);
        }
        Ok(BucketKey { indices })
    }

    pub fn validate(&self) -> Result<()> {
        validate_coordinates(&self.origin, "bucket grid origin")?;
        if self.widths.is_empty() {
            return Err(invalid_argument("bucket grid widths must not be empty"));
        }
        if self.origin.len() != self.widths.len() {
            return Err(invalid_argument(
                "bucket grid origin and widths must have the same dimensions",
            ));
        }
        if self
            .widths
            .iter()
            .any(|width| *width <= 0.0 || !width.is_finite())
        {
            return Err(invalid_argument(
                "bucket grid widths must be finite and positive",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BucketKey {
    pub indices: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DenseBucket {
    pub key: BucketKey,
    pub count: u64,
    pub weight_sum: f64,
    pub averages: DenseAverages,
    pub bounds: DenseBounds,
    pub point_indices: Vec<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KMeansConfig {
    pub clusters: usize,
    pub max_iterations: usize,
    pub tolerance: f64,
}

impl KMeansConfig {
    pub fn new(clusters: usize) -> Result<Self> {
        let config = Self {
            clusters,
            ..Self::default()
        };
        config.validate()?;
        Ok(config)
    }

    pub fn validate(self) -> Result<()> {
        if self.clusters == 0 {
            return Err(invalid_argument("cluster count must be positive"));
        }
        if self.max_iterations == 0 {
            return Err(invalid_argument("max iterations must be positive"));
        }
        if self.tolerance < 0.0 || !self.tolerance.is_finite() {
            return Err(invalid_argument(
                "cluster tolerance must be finite and non-negative",
            ));
        }
        Ok(())
    }
}

impl Default for KMeansConfig {
    fn default() -> Self {
        Self {
            clusters: 8,
            max_iterations: 100,
            tolerance: 0.0001,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DenseCluster {
    pub cluster_index: usize,
    pub centroid: Vec<f64>,
    pub count: u64,
    pub weight_sum: f64,
    pub averages: Option<DenseAverages>,
    pub bounds: Option<DenseBounds>,
    pub point_indices: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClusterResult {
    pub iterations: usize,
    pub clusters: Vec<DenseCluster>,
}

pub fn dense_summary(points: &[DensePoint]) -> Result<DenseSummary> {
    SummaryAccumulator::from_points(points)?.summary()
}

pub fn dense_averages(points: &[DensePoint]) -> Result<DenseAverages> {
    SummaryAccumulator::from_points(points)?.averages()
}

pub fn dense_bounds(points: &[DensePoint]) -> Result<DenseBounds> {
    SummaryAccumulator::from_points(points)?.bounds()
}

pub fn bucket_points(points: &[DensePoint], grid: &BucketGrid) -> Result<Vec<DenseBucket>> {
    validate_point_set(points)?;
    grid.validate()?;
    let mut buckets = BTreeMap::<BucketKey, BucketAccumulator>::new();
    for (index, point) in points.iter().enumerate() {
        let key = grid.key_for(point)?;
        buckets
            .entry(key.clone())
            .or_insert_with(|| BucketAccumulator::new(key))
            .push(index, point);
    }
    buckets
        .into_values()
        .map(BucketAccumulator::finish)
        .collect()
}

pub fn k_means(points: &[DensePoint], config: KMeansConfig) -> Result<ClusterResult> {
    config.validate()?;
    let dimensions = validate_point_set(points)?;
    if config.clusters > points.len() {
        return Err(invalid_argument(
            "cluster count must not exceed the number of points",
        ));
    }

    let mut centroids = initial_centroids(points, config.clusters);
    let mut assignments = vec![usize::MAX; points.len()];
    let mut iterations = 0;

    for iteration in 1..=config.max_iterations {
        iterations = iteration;
        let changed = assign_points(points, &centroids, &mut assignments);
        let next_centroids = recompute_centroids(points, &assignments, &centroids, dimensions);
        let shift = centroid_shift(&centroids, &next_centroids);
        centroids = next_centroids;
        if !changed || shift <= config.tolerance {
            break;
        }
    }

    let mut accumulators = centroids
        .iter()
        .cloned()
        .enumerate()
        .map(|(index, centroid)| ClusterAccumulator::new(index, centroid))
        .collect::<Vec<_>>();
    for (point_index, point) in points.iter().enumerate() {
        accumulators[assignments[point_index]].push(point_index, point);
    }

    Ok(ClusterResult {
        iterations,
        clusters: accumulators
            .into_iter()
            .map(ClusterAccumulator::finish)
            .collect(),
    })
}

#[derive(Debug, Clone)]
struct BucketAccumulator {
    key: BucketKey,
    count: u64,
    point_indices: Vec<usize>,
    summary: SummaryAccumulator,
}

impl BucketAccumulator {
    fn new(key: BucketKey) -> Self {
        Self {
            key,
            count: 0,
            point_indices: Vec::new(),
            summary: SummaryAccumulator::new(),
        }
    }

    fn push(&mut self, point_index: usize, point: &DensePoint) {
        self.count += 1;
        self.point_indices.push(point_index);
        self.summary.push(point);
    }

    fn finish(self) -> Result<DenseBucket> {
        Ok(DenseBucket {
            key: self.key,
            count: self.count,
            weight_sum: self.summary.weight_sum,
            averages: self.summary.averages()?,
            bounds: self.summary.bounds()?,
            point_indices: self.point_indices,
        })
    }
}

#[derive(Debug, Clone)]
struct ClusterAccumulator {
    cluster_index: usize,
    centroid: Vec<f64>,
    count: u64,
    point_indices: Vec<usize>,
    summary: SummaryAccumulator,
}

impl ClusterAccumulator {
    fn new(cluster_index: usize, centroid: Vec<f64>) -> Self {
        Self {
            cluster_index,
            centroid,
            count: 0,
            point_indices: Vec::new(),
            summary: SummaryAccumulator::new(),
        }
    }

    fn push(&mut self, point_index: usize, point: &DensePoint) {
        self.count += 1;
        self.point_indices.push(point_index);
        self.summary.push(point);
    }

    fn finish(self) -> DenseCluster {
        DenseCluster {
            cluster_index: self.cluster_index,
            centroid: self.centroid,
            count: self.count,
            weight_sum: self.summary.weight_sum,
            averages: self.summary.averages().ok(),
            bounds: self.summary.bounds().ok(),
            point_indices: self.point_indices,
        }
    }
}

#[derive(Debug, Clone)]
struct SummaryAccumulator {
    count: u64,
    weight_sum: f64,
    coordinate_stats: Vec<RunningStats>,
    value_stats: RunningStats,
    has_values: bool,
    min: Vec<f64>,
    max: Vec<f64>,
}

impl SummaryAccumulator {
    fn new() -> Self {
        Self {
            count: 0,
            weight_sum: 0.0,
            coordinate_stats: Vec::new(),
            value_stats: RunningStats::new(),
            has_values: false,
            min: Vec::new(),
            max: Vec::new(),
        }
    }

    fn from_points(points: &[DensePoint]) -> Result<Self> {
        validate_point_set(points)?;
        let mut summary = Self::new();
        for point in points {
            summary.push(point);
        }
        Ok(summary)
    }

    fn push(&mut self, point: &DensePoint) {
        if self.coordinate_stats.is_empty() {
            self.coordinate_stats
                .resize_with(point.dimensions(), RunningStats::new);
            self.min.resize(point.dimensions(), f64::INFINITY);
            self.max.resize(point.dimensions(), f64::NEG_INFINITY);
        }

        self.count += 1;
        self.weight_sum += point.weight;
        for (index, (stats, coordinate)) in self
            .coordinate_stats
            .iter_mut()
            .zip(&point.coordinates)
            .enumerate()
        {
            stats
                .push_weighted(*coordinate, point.weight)
                .expect("validated points have valid coordinate weights");
            self.min[index] = self.min[index].min(*coordinate);
            self.max[index] = self.max[index].max(*coordinate);
        }
        if let Some(point_value) = point.value {
            self.value_stats
                .push_weighted(point_value, point.weight)
                .expect("validated points have valid scalar weights");
            self.has_values = true;
        }
    }

    fn averages(&self) -> Result<DenseAverages> {
        if self.count == 0 {
            return Err(invalid_argument("dense point set must not be empty"));
        }
        let coordinates = self
            .coordinate_stats
            .iter()
            .map(|stats| {
                stats
                    .summary()
                    .mean
                    .expect("coordinate summary has a weighted mean")
            })
            .collect();
        let value_summary = self.has_values.then(|| self.value_stats.summary());
        Ok(DenseAverages {
            count: self.count,
            weight_sum: self.weight_sum,
            coordinates,
            value_count: value_summary.as_ref().map_or(0, |summary| summary.count),
            value_weight_sum: value_summary
                .as_ref()
                .map_or(0.0, |summary| summary.weight_sum),
            value: value_summary.and_then(|summary| summary.mean),
        })
    }

    fn bounds(&self) -> Result<DenseBounds> {
        if self.count == 0 {
            return Err(invalid_argument("dense point set must not be empty"));
        }
        Ok(DenseBounds {
            min: self.min.clone(),
            max: self.max.clone(),
        })
    }

    fn summary(&self) -> Result<DenseSummary> {
        Ok(DenseSummary {
            count: self.count,
            dimensions: self.coordinate_stats.len(),
            weight_sum: self.weight_sum,
            coordinate_stats: self
                .coordinate_stats
                .iter()
                .map(RunningStats::summary)
                .collect(),
            value_stats: self.has_values.then(|| self.value_stats.summary()),
            bounds: self.bounds()?,
        })
    }
}

fn initial_centroids(points: &[DensePoint], clusters: usize) -> Vec<Vec<f64>> {
    if clusters == 1 {
        return vec![points[0].coordinates.clone()];
    }
    let last_index = points.len() - 1;
    (0..clusters)
        .map(|cluster| {
            let index = (cluster * last_index + (clusters - 1) / 2) / (clusters - 1);
            points[index].coordinates.clone()
        })
        .collect()
}

fn assign_points(points: &[DensePoint], centroids: &[Vec<f64>], assignments: &mut [usize]) -> bool {
    let mut changed = false;
    for (point_index, point) in points.iter().enumerate() {
        let mut best_cluster = 0;
        let mut best_distance = f64::INFINITY;
        for (cluster_index, centroid) in centroids.iter().enumerate() {
            let distance = squared_distance(&point.coordinates, centroid);
            if distance < best_distance {
                best_distance = distance;
                best_cluster = cluster_index;
            }
        }
        if assignments[point_index] != best_cluster {
            assignments[point_index] = best_cluster;
            changed = true;
        }
    }
    changed
}

fn recompute_centroids(
    points: &[DensePoint],
    assignments: &[usize],
    centroids: &[Vec<f64>],
    dimensions: usize,
) -> Vec<Vec<f64>> {
    let mut next = vec![vec![0.0; dimensions]; centroids.len()];
    let mut weights = vec![0.0; centroids.len()];

    for (point, cluster) in points.iter().zip(assignments) {
        weights[*cluster] += point.weight;
        for (value, coordinate) in next[*cluster].iter_mut().zip(&point.coordinates) {
            *value += point.weight * coordinate;
        }
    }

    for (cluster, centroid) in next.iter_mut().enumerate() {
        if weights[cluster] == 0.0 {
            *centroid = centroids[cluster].clone();
            continue;
        }
        for value in centroid {
            *value /= weights[cluster];
        }
    }
    next
}

fn centroid_shift(left: &[Vec<f64>], right: &[Vec<f64>]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(left, right)| squared_distance(left, right).sqrt())
        .fold(0.0, f64::max)
}

fn squared_distance(left: &[f64], right: &[f64]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(left, right)| {
            let delta = left - right;
            delta * delta
        })
        .sum()
}

fn validate_point_set(points: &[DensePoint]) -> Result<usize> {
    if points.is_empty() {
        return Err(invalid_argument("dense point set must not be empty"));
    }
    let dimensions = points[0].dimensions();
    for point in points {
        point.validate()?;
        if point.dimensions() != dimensions {
            return Err(invalid_argument(
                "dense point set coordinates must have the same dimensions",
            ));
        }
    }
    Ok(dimensions)
}

fn validate_coordinates(coordinates: &[f64], name: &str) -> Result<()> {
    if coordinates.is_empty() {
        return Err(invalid_argument(format!("{name} must not be empty")));
    }
    if coordinates.iter().any(|coordinate| !coordinate.is_finite()) {
        return Err(invalid_argument(format!("{name} must be finite")));
    }
    Ok(())
}

fn invalid_argument(message: impl Into<String>) -> DetectError {
    DetectError::InvalidArgument(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(coordinates: impl Into<Vec<f64>>) -> DensePoint {
        DensePoint::new(coordinates).unwrap()
    }

    fn assert_close(left: f64, right: f64) {
        assert!((left - right).abs() < 1.0e-12, "{left} != {right}");
    }

    #[test]
    fn weighted_averages_include_coordinates_and_values() {
        let points = [
            point([0.0, 2.0])
                .weighted(1.0)
                .unwrap()
                .valued(10.0)
                .unwrap(),
            point([10.0, 6.0])
                .weighted(3.0)
                .unwrap()
                .valued(20.0)
                .unwrap(),
        ];

        let averages = dense_averages(&points).unwrap();

        assert_eq!(averages.count, 2);
        assert_eq!(averages.weight_sum, 4.0);
        assert_eq!(averages.coordinates, vec![7.5, 5.0]);
        assert_eq!(averages.value, Some(17.5));
    }

    #[test]
    fn dense_summary_matches_weighted_averages_and_bounds() {
        let points = [
            point([0.0, 2.0])
                .weighted(1.0)
                .unwrap()
                .valued(10.0)
                .unwrap(),
            point([10.0, 6.0])
                .weighted(3.0)
                .unwrap()
                .valued(20.0)
                .unwrap(),
        ];

        let summary = dense_summary(&points).unwrap();

        assert_eq!(summary.count, 2);
        assert_eq!(summary.dimensions, 2);
        assert_eq!(summary.weight_sum, 4.0);
        assert_eq!(summary.coordinate_stats.len(), 2);
        assert_eq!(summary.coordinate_stats[0].mean, Some(7.5));
        assert_eq!(summary.coordinate_stats[1].mean, Some(5.0));
        assert_eq!(summary.coordinate_stats[0].min, Some(0.0));
        assert_eq!(summary.coordinate_stats[0].max, Some(10.0));
        assert_eq!(summary.bounds.min, vec![0.0, 2.0]);
        assert_eq!(summary.bounds.max, vec![10.0, 6.0]);
        assert_eq!(
            summary.value_stats.as_ref().and_then(|stats| stats.mean),
            Some(17.5)
        );
        assert_eq!(
            summary.value_stats.as_ref().map(|stats| stats.weight_sum),
            Some(4.0)
        );
    }

    #[test]
    fn dataset_summary_wraps_point_summary() {
        let dataset = DenseDataset::from_points([
            point([1.0, 2.0]),
            point([3.0, 4.0]).weighted(2.0).unwrap(),
        ])
        .unwrap();

        let summary = dataset.summary().unwrap();

        assert_eq!(summary.count, 2);
        assert_close(summary.coordinate_stats[0].mean.unwrap(), 7.0 / 3.0);
        assert_close(summary.coordinate_stats[1].mean.unwrap(), 10.0 / 3.0);
    }

    #[test]
    fn dataset_exposes_covariance_and_pca_helpers() {
        let dataset =
            DenseDataset::from_points([point([1.0, 1.0]), point([2.0, 2.0]), point([3.0, 3.0])])
                .unwrap();
        let covariance = dataset.covariance_matrix().unwrap();
        let pca = dataset.principal_components(1).unwrap();
        assert_eq!(covariance.matrix.shape().rows, 2);
        assert_eq!(pca.components().shape().rows, 1);
    }

    #[test]
    fn dense_summary_omits_value_stats_when_points_have_no_values() {
        let summary = dense_summary(&[point([1.0]), point([3.0])]).unwrap();
        assert!(summary.value_stats.is_none());
    }

    #[test]
    fn dense_summary_rejects_empty_point_sets() {
        assert!(dense_summary(&[]).is_err());
    }

    #[test]
    fn grid_buckets_floor_coordinates_by_dimension() {
        let points = vec![point([0.2, 1.9]), point([1.1, 1.2]), point([-0.1, 0.2])];
        let grid = BucketGrid::uniform(2, 1.0).unwrap();

        let buckets = bucket_points(&points, &grid).unwrap();

        assert_eq!(buckets.len(), 3);
        assert_eq!(buckets[0].key.indices, vec![-1, 0]);
        assert_eq!(buckets[0].point_indices, vec![2]);
        assert_eq!(buckets[1].key.indices, vec![0, 1]);
        assert_eq!(buckets[2].key.indices, vec![1, 1]);
    }

    #[test]
    fn dataset_rejects_mismatched_dimensions() {
        let mut dataset = DenseDataset::new();
        dataset.push(point([1.0, 2.0])).unwrap();

        let err = dataset.push(point([1.0])).unwrap_err();

        assert!(err.to_string().contains("same dimensions"));
    }

    #[test]
    fn k_means_splits_dense_groups_deterministically() {
        let points = vec![
            point([0.0, 0.0]),
            point([0.2, 0.1]),
            point([9.0, 9.0]),
            point([10.0, 10.0]),
        ];

        let result = k_means(
            &points,
            KMeansConfig {
                clusters: 2,
                max_iterations: 20,
                tolerance: 0.0,
            },
        )
        .unwrap();

        assert_eq!(result.clusters.len(), 2);
        assert_eq!(result.clusters[0].point_indices, vec![0, 1]);
        assert_eq!(result.clusters[1].point_indices, vec![2, 3]);
        assert_eq!(result.clusters[0].centroid, vec![0.1, 0.05]);
        assert_eq!(result.clusters[1].centroid, vec![9.5, 9.5]);
    }

    #[test]
    fn invalid_points_are_rejected() {
        assert!(DensePoint::new([f64::NAN]).is_err());
        assert!(point([1.0]).weighted(0.0).is_err());
        assert!(point([1.0]).valued(f64::INFINITY).is_err());
    }
}
