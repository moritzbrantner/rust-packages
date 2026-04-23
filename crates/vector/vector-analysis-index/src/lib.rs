#![doc = include_str!("../README.md")]

use vector_analysis_core::{metric_distance, DenseVector, VectorMetric};
use video_analysis_core::{DetectError, Result};

#[derive(Debug, Clone, PartialEq)]
pub struct VectorRecord {
    pub id: String,
    pub vector: DenseVector,
}

impl VectorRecord {
    pub fn new(id: impl Into<String>, vector: DenseVector) -> Self {
        Self {
            id: id.into(),
            vector,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchConfig {
    pub metric: VectorMetric,
    pub limit: usize,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            metric: VectorMetric::Cosine,
            limit: 10,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchResult {
    pub id: String,
    pub distance: f32,
    pub score: f32,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct VectorSearchIndex {
    dimensions: Option<usize>,
    records: Vec<VectorRecord>,
}

impl VectorSearchIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn dimensions(&self) -> Option<usize> {
        self.dimensions
    }

    pub fn records(&self) -> &[VectorRecord] {
        &self.records
    }

    pub fn add(&mut self, record: VectorRecord) -> Result<()> {
        record.vector.validate()?;
        match self.dimensions {
            Some(dimensions) if dimensions != record.vector.dimensions() => {
                return Err(invalid_argument(
                    "indexed vectors must have the same dimensions",
                ));
            }
            None => self.dimensions = Some(record.vector.dimensions()),
            _ => {}
        }
        self.records.push(record);
        Ok(())
    }

    pub fn extend(&mut self, records: impl IntoIterator<Item = VectorRecord>) -> Result<()> {
        for record in records {
            self.add(record)?;
        }
        Ok(())
    }

    pub fn search(&self, query: &DenseVector, config: SearchConfig) -> Result<Vec<SearchResult>> {
        if config.limit == 0 {
            return Err(invalid_argument("search limit must be greater than zero"));
        }
        if let Some(dimensions) = self.dimensions {
            if query.dimensions() != dimensions {
                return Err(invalid_argument("query dimensions must match the index"));
            }
        }
        let mut results = Vec::with_capacity(self.records.len());
        for record in &self.records {
            let distance =
                metric_distance(config.metric, query.as_slice(), record.vector.as_slice())?;
            results.push(SearchResult {
                id: record.id.clone(),
                distance,
                score: score_from_distance(config.metric, distance),
            });
        }
        results.sort_by(|left, right| {
            left.distance
                .total_cmp(&right.distance)
                .then_with(|| left.id.cmp(&right.id))
        });
        results.truncate(config.limit);
        Ok(results)
    }
}

pub fn assign_nearest_centroids(
    vectors: &[DenseVector],
    centroids: &[DenseVector],
    metric: VectorMetric,
) -> Result<Vec<usize>> {
    if centroids.is_empty() {
        return Err(invalid_argument("centroids must not be empty"));
    }
    let mut assignments = Vec::with_capacity(vectors.len());
    for vector in vectors {
        let mut best_index = 0;
        let mut best_distance = f32::INFINITY;
        for (index, centroid) in centroids.iter().enumerate() {
            let distance = metric_distance(metric, vector.as_slice(), centroid.as_slice())?;
            if distance < best_distance {
                best_distance = distance;
                best_index = index;
            }
        }
        assignments.push(best_index);
    }
    Ok(assignments)
}

fn score_from_distance(metric: VectorMetric, distance: f32) -> f32 {
    match metric {
        VectorMetric::Cosine => 1.0 - distance,
        VectorMetric::Dot => -distance,
        VectorMetric::Euclidean | VectorMetric::Manhattan => 1.0 / (1.0 + distance),
    }
}

fn invalid_argument(message: impl Into<String>) -> DetectError {
    DetectError::InvalidArgument(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn searches_nearest_vector() {
        let mut index = VectorSearchIndex::new();
        index
            .add(VectorRecord::new(
                "x",
                DenseVector::new([1.0, 0.0]).unwrap(),
            ))
            .unwrap();
        index
            .add(VectorRecord::new(
                "y",
                DenseVector::new([0.0, 1.0]).unwrap(),
            ))
            .unwrap();
        let results = index
            .search(
                &DenseVector::new([0.9, 0.1]).unwrap(),
                SearchConfig::default(),
            )
            .unwrap();
        assert_eq!(results[0].id, "x");
    }

    #[test]
    fn assigns_nearest_centroid() {
        let assignments = assign_nearest_centroids(
            &[DenseVector::new([9.0, 0.0]).unwrap()],
            &[
                DenseVector::new([0.0, 0.0]).unwrap(),
                DenseVector::new([10.0, 0.0]).unwrap(),
            ],
            VectorMetric::Euclidean,
        )
        .unwrap();
        assert_eq!(assignments, vec![1]);
    }
}
