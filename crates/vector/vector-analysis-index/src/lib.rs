#![doc = include_str!("../README.md")]

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};
use vector_analysis_core::{metric_distance, DenseVector, VectorMetric};
use video_analysis_core::{DetectError, Result};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct VectorRecordId(String);

impl VectorRecordId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for VectorRecordId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for VectorRecordId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<String> for VectorRecordId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for VectorRecordId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VectorRecordMetadata {
    pub tags: Vec<String>,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VectorRecord {
    pub id: String,
    pub vector: DenseVector,
    pub payload: VectorRecordMetadata,
}

impl VectorRecord {
    pub fn new(id: impl Into<VectorRecordId>, vector: DenseVector) -> Self {
        Self::with_payload(id, vector, VectorRecordMetadata::default())
    }

    pub fn with_payload(
        id: impl Into<VectorRecordId>,
        vector: DenseVector,
        payload: VectorRecordMetadata,
    ) -> Self {
        let id = id.into();
        Self {
            id: id.into_string(),
            vector,
            payload,
        }
    }

    pub fn record_id(&self) -> VectorRecordId {
        VectorRecordId::from(self.id.clone())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VectorSearchFilter {
    pub required_tags: Vec<String>,
    pub metadata_equals: BTreeMap<String, String>,
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

#[derive(Debug, Clone, PartialEq)]
pub struct VectorHit {
    pub id: VectorRecordId,
    pub distance: f32,
    pub score: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SerializableVectorRecord {
    pub id: VectorRecordId,
    pub vector: Vec<f32>,
    pub payload: VectorRecordMetadata,
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

    pub fn clear(&mut self) {
        self.dimensions = None;
        self.records.clear();
    }

    pub fn add(&mut self, record: VectorRecord) -> Result<()> {
        record.vector.validate()?;
        if record.id.trim().is_empty() {
            return Err(invalid_argument("record id must not be empty"));
        }
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

    pub fn from_records(records: impl IntoIterator<Item = VectorRecord>) -> Result<Self> {
        let mut index = Self::new();
        index.extend(records)?;
        Ok(index)
    }

    pub fn export_records(&self) -> Vec<SerializableVectorRecord> {
        self.records
            .iter()
            .map(|record| SerializableVectorRecord {
                id: record.record_id(),
                vector: record.vector.as_slice().to_vec(),
                payload: record.payload.clone(),
            })
            .collect()
    }

    pub fn import_records(
        records: impl IntoIterator<Item = SerializableVectorRecord>,
    ) -> Result<Self> {
        let mut index = Self::new();
        for record in records {
            index.add(VectorRecord::with_payload(
                record.id,
                DenseVector::new(record.vector)?,
                record.payload,
            ))?;
        }
        Ok(index)
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

    pub fn search_filtered(
        &self,
        query: &[f32],
        top_k: usize,
        filter: Option<&VectorSearchFilter>,
    ) -> Result<Vec<VectorHit>> {
        if top_k == 0 {
            return Err(invalid_argument("search limit must be greater than zero"));
        }
        validate_query_slice(query)?;
        if let Some(dimensions) = self.dimensions {
            if query.len() != dimensions {
                return Err(invalid_argument("query dimensions must match the index"));
            }
        }

        let mut results = Vec::with_capacity(self.records.len());
        for record in &self.records {
            if let Some(filter) = filter {
                if !matches_filter(&record.payload, filter) {
                    continue;
                }
            }
            let distance = metric_distance(VectorMetric::Cosine, query, record.vector.as_slice())?;
            results.push(VectorHit {
                id: record.record_id(),
                distance,
                score: score_from_distance(VectorMetric::Cosine, distance),
            });
        }
        results.sort_by(|left, right| {
            left.distance
                .total_cmp(&right.distance)
                .then_with(|| left.id.cmp(&right.id))
        });
        results.truncate(top_k);
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

fn matches_filter(payload: &VectorRecordMetadata, filter: &VectorSearchFilter) -> bool {
    filter
        .required_tags
        .iter()
        .all(|tag| payload.tags.iter().any(|candidate| candidate == tag))
        && filter
            .metadata_equals
            .iter()
            .all(|(key, value)| payload.metadata.get(key) == Some(value))
}

fn validate_query_slice(query: &[f32]) -> Result<()> {
    if query.is_empty() {
        return Err(invalid_argument("query vector must not be empty"));
    }
    if query.iter().any(|value| !value.is_finite()) {
        return Err(invalid_argument("query vector components must be finite"));
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
    fn filters_search_results_by_tags_and_metadata() {
        let mut index = VectorSearchIndex::new();
        index
            .add(VectorRecord::with_payload(
                "alpha",
                DenseVector::new([1.0, 0.0]).unwrap(),
                VectorRecordMetadata {
                    tags: vec!["docs".to_string()],
                    metadata: BTreeMap::from([(String::from("lang"), String::from("en"))]),
                },
            ))
            .unwrap();
        index
            .add(VectorRecord::with_payload(
                "beta",
                DenseVector::new([1.0, 0.1]).unwrap(),
                VectorRecordMetadata {
                    tags: vec!["blog".to_string()],
                    metadata: BTreeMap::from([(String::from("lang"), String::from("de"))]),
                },
            ))
            .unwrap();

        let filter = VectorSearchFilter {
            required_tags: vec!["docs".to_string()],
            metadata_equals: BTreeMap::from([(String::from("lang"), String::from("en"))]),
        };
        let results = index
            .search_filtered(&[1.0, 0.0], 10, Some(&filter))
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id.as_str(), "alpha");
    }

    #[test]
    fn export_records_round_trip() {
        let mut index = VectorSearchIndex::new();
        index
            .add(VectorRecord::with_payload(
                "alpha",
                DenseVector::new([1.0, 0.0]).unwrap(),
                VectorRecordMetadata {
                    tags: vec!["docs".to_string()],
                    metadata: BTreeMap::from([(String::from("lang"), String::from("en"))]),
                },
            ))
            .unwrap();

        let exported = index.export_records();
        let imported = VectorSearchIndex::import_records(exported).unwrap();

        assert_eq!(imported.records(), index.records());
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
