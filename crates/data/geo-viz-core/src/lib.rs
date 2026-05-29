#![doc = include_str!("../README.md")]

pub mod surface;

use std::collections::{BTreeMap, BTreeSet, HashMap};

use geo_data::Coordinate;
use geojson024::{feature::Id, Feature, Geometry, JsonObject, Value};
use serde::{Deserialize, Serialize};
use serde_json::json;
use supercluster::{CoordinateSystem, Supercluster};
use video_analysis_core::{DetectError, Result};

/// Numeric metric bag attached to points and aggregated features.
pub type GeoVizMetricRecord = BTreeMap<String, f64>;

/// Geographic bounding box in `[west, south, east, north]` order.
pub type GeoVizBounds = [f64; 4];

fn invalid_argument(message: impl Into<String>) -> DetectError {
    DetectError::InvalidArgument(message.into())
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
/// Input point for map visualization indexes.
pub struct GeoVizPoint {
    /// Caller-owned optional identifier.
    pub id: Option<String>,
    /// Optional human-readable label.
    pub label: Option<String>,
    /// Longitude in degrees.
    pub longitude: f64,
    /// Latitude in degrees.
    pub latitude: f64,
    /// Finite numeric metrics.
    #[serde(default)]
    pub metrics: GeoVizMetricRecord,
    /// Caller-owned JSON properties.
    #[serde(default)]
    pub properties: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
/// Indexed, normalized map point.
pub struct GeoVizIndexedPoint {
    /// Stable point id.
    pub id: String,
    /// Source index from the input array.
    pub source_index: usize,
    /// Label, defaulting to an empty string.
    pub label: String,
    /// Longitude in degrees.
    pub longitude: f64,
    /// Latitude in degrees.
    pub latitude: f64,
    /// Finite numeric metrics.
    pub metrics: GeoVizMetricRecord,
    /// Caller-owned JSON properties.
    pub properties: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
/// Query for a geographic viewport.
pub struct GeoVizViewportQuery {
    /// Viewport bounds in `[west, south, east, north]` order.
    pub bounds: GeoVizBounds,
    /// Map zoom level.
    pub zoom: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
/// Point aggregation configuration.
pub struct GeoVizAggregationOptions {
    /// Cluster radius in pixels.
    pub radius: Option<f64>,
    /// Tile extent used by supercluster.
    pub extent: Option<f64>,
    /// Minimum clustering zoom.
    pub min_zoom: Option<u8>,
    /// Maximum clustering zoom.
    pub max_zoom: Option<u8>,
}

impl Default for GeoVizAggregationOptions {
    fn default() -> Self {
        Self {
            radius: Some(72.0),
            extent: Some(512.0),
            min_zoom: Some(0),
            max_zoom: Some(16),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
/// Aggregated viewport feature.
pub enum GeoVizAggregationFeature {
    /// Individual visible point.
    Point {
        /// `[longitude, latitude]`.
        coordinates: [f64; 2],
        /// Aggregated metrics.
        metrics: GeoVizMetricRecord,
        /// Original point.
        point: GeoVizIndexedPoint,
    },
    /// Visible cluster.
    Cluster {
        /// Supercluster cluster id.
        cluster_id: usize,
        /// `[longitude, latitude]`.
        coordinates: [f64; 2],
        /// Zoom where this cluster expands.
        expansion_zoom: usize,
        /// Aggregated metrics.
        metrics: GeoVizMetricRecord,
        /// Number of source points represented.
        point_count: usize,
        /// Compact count label.
        point_count_abbreviated: String,
    },
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
/// Summary for visible aggregated features.
pub struct GeoVizAggregationSummary {
    /// Queried bounds.
    pub bounds: GeoVizBounds,
    /// Queried zoom.
    pub zoom: f64,
    /// Aggregated visible metrics.
    pub metrics: GeoVizMetricRecord,
    /// Source point count represented by visible features.
    pub visible_point_count: usize,
    /// Visible cluster count.
    pub visible_cluster_count: usize,
    /// Visible unclustered point count.
    pub visible_unclustered_count: usize,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
/// Aggregation result for one viewport.
pub struct GeoVizAggregation {
    /// Visible features.
    pub features: Vec<GeoVizAggregationFeature>,
    /// Visible summary.
    pub summary: GeoVizAggregationSummary,
}

/// Geographic point aggregation index.
#[derive(Debug, Clone)]
pub struct GeoPointIndex {
    points: Vec<GeoVizIndexedPoint>,
    point_lookup: HashMap<String, GeoVizIndexedPoint>,
    metric_keys: Vec<String>,
    tree: Supercluster,
}

impl GeoPointIndex {
    /// Builds a new point index.
    pub fn new(
        points: impl IntoIterator<Item = GeoVizPoint>,
        options: GeoVizAggregationOptions,
    ) -> Result<Self> {
        let normalized = points
            .into_iter()
            .enumerate()
            .map(|(index, point)| normalize_point(point, index))
            .collect::<Result<Vec<_>>>()?;
        let metric_keys = collect_metric_keys(&normalized);
        let point_lookup = normalized
            .iter()
            .cloned()
            .map(|point| (point.id.clone(), point))
            .collect::<HashMap<_, _>>();
        let mut tree = Supercluster::new(
            Supercluster::builder()
                .radius(options.radius.unwrap_or(72.0))
                .extent(options.extent.unwrap_or(512.0))
                .min_zoom(options.min_zoom.unwrap_or(0))
                .max_zoom(options.max_zoom.unwrap_or(16))
                .coordinate_system(CoordinateSystem::LatLng)
                .build(),
        );

        tree.load(normalized.iter().map(point_to_feature).collect::<Vec<_>>())
            .map_err(|error| invalid_argument(error.to_string()))?;

        Ok(Self {
            points: normalized,
            point_lookup,
            metric_keys,
            tree,
        })
    }

    /// Returns bounds for all indexed points.
    pub fn get_bounds(&self) -> Option<GeoVizBounds> {
        bounds_for_points(&self.points)
    }

    /// Returns one point by id.
    pub fn get_point_by_id(&self, point_id: &str) -> Option<GeoVizIndexedPoint> {
        self.point_lookup.get(point_id).cloned()
    }

    /// Returns visible features for a viewport.
    pub fn get_viewport_aggregation(
        &self,
        query: GeoVizViewportQuery,
    ) -> Result<GeoVizAggregation> {
        validate_bounds(query.bounds)?;
        let zoom = query.zoom.round().clamp(0.0, u8::MAX as f64) as u8;
        let raw_features = self
            .tree
            .get_clusters(query.bounds, zoom)
            .map_err(|error| invalid_argument(error.to_string()))?;
        let mut seen = BTreeSet::new();
        let features = raw_features
            .into_iter()
            .filter_map(|feature| self.to_aggregation_feature(feature).transpose())
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .filter(|feature| {
                let key = feature_key(feature);
                if seen.contains(&key) {
                    return false;
                }
                seen.insert(key);
                true
            })
            .collect::<Vec<_>>();

        Ok(GeoVizAggregation {
            summary: summarize_features(query, &features, &self.metric_keys),
            features,
        })
    }

    /// Returns the zoom where a cluster expands.
    pub fn get_cluster_expansion_zoom(&self, cluster_id: usize) -> usize {
        self.tree.get_cluster_expansion_zoom(cluster_id)
    }

    /// Returns source leaves for a cluster.
    pub fn get_cluster_leaves(
        &self,
        cluster_id: usize,
        limit: usize,
        offset: usize,
    ) -> Vec<GeoVizIndexedPoint> {
        self.tree
            .get_leaves(cluster_id, limit, offset)
            .into_iter()
            .filter_map(|feature| point_id_from_feature(&feature))
            .filter_map(|point_id| self.point_lookup.get(&point_id).cloned())
            .collect()
    }

    fn to_aggregation_feature(&self, feature: Feature) -> Result<Option<GeoVizAggregationFeature>> {
        let Some(coordinates) = coordinates_from_feature(&feature) else {
            return Ok(None);
        };

        if feature
            .property("cluster")
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
        {
            let cluster_id = read_usize_property(&feature, "cluster_id")?;
            let point_count = read_usize_property(&feature, "point_count")?;
            let point_count_abbreviated = feature
                .property("point_count_abbreviated")
                .and_then(|value| value.as_str())
                .map(str::to_string)
                .unwrap_or_else(|| abbreviate_count(point_count));
            let leaves = self.get_cluster_leaves(cluster_id, point_count, 0);
            let metrics = sum_metrics(leaves.iter().map(|point| &point.metrics), &self.metric_keys);

            return Ok(Some(GeoVizAggregationFeature::Cluster {
                cluster_id,
                coordinates,
                expansion_zoom: self.get_cluster_expansion_zoom(cluster_id),
                metrics,
                point_count,
                point_count_abbreviated,
            }));
        }

        let Some(point_id) = point_id_from_feature(&feature) else {
            return Ok(None);
        };
        let Some(point) = self.point_lookup.get(&point_id).cloned() else {
            return Ok(None);
        };

        Ok(Some(GeoVizAggregationFeature::Point {
            coordinates,
            metrics: point.metrics.clone(),
            point,
        }))
    }
}

fn normalize_point(point: GeoVizPoint, source_index: usize) -> Result<GeoVizIndexedPoint> {
    Coordinate::new(point.longitude, point.latitude)?.validate_geographic()?;
    Ok(GeoVizIndexedPoint {
        id: point.id.unwrap_or_else(|| source_index.to_string()),
        source_index,
        label: point.label.unwrap_or_default(),
        longitude: point.longitude,
        latitude: point.latitude,
        metrics: point
            .metrics
            .into_iter()
            .filter(|(_, value)| value.is_finite())
            .collect(),
        properties: point.properties,
    })
}

fn point_to_feature(point: &GeoVizIndexedPoint) -> Feature {
    let mut properties = JsonObject::new();
    properties.insert("pointId".to_string(), json!(point.id));
    for (metric_key, value) in &point.metrics {
        properties.insert(metric_key.clone(), json!(value));
    }

    Feature {
        bbox: None,
        foreign_members: None,
        geometry: Some(Geometry::new(Value::Point(vec![
            point.longitude,
            point.latitude,
        ]))),
        id: Some(Id::String(point.id.clone())),
        properties: Some(properties),
    }
}

fn collect_metric_keys(points: &[GeoVizIndexedPoint]) -> Vec<String> {
    let mut keys = BTreeSet::new();
    for point in points {
        for key in point.metrics.keys() {
            keys.insert(key.clone());
        }
    }
    keys.into_iter().collect()
}

fn bounds_for_points(points: &[GeoVizIndexedPoint]) -> Option<GeoVizBounds> {
    let first = points.first()?;
    let mut west = first.longitude;
    let mut south = first.latitude;
    let mut east = first.longitude;
    let mut north = first.latitude;

    for point in points.iter().skip(1) {
        west = west.min(point.longitude);
        south = south.min(point.latitude);
        east = east.max(point.longitude);
        north = north.max(point.latitude);
    }

    Some([west, south, east, north])
}

fn validate_bounds(bounds: GeoVizBounds) -> Result<()> {
    if bounds.iter().any(|value| !value.is_finite()) {
        return Err(invalid_argument("viewport bounds must be finite"));
    }
    if bounds[1] > bounds[3] {
        return Err(invalid_argument("viewport south must be <= north"));
    }
    if bounds[1] < -90.0 || bounds[3] > 90.0 {
        return Err(invalid_argument(
            "viewport latitude bounds must stay between -90 and 90",
        ));
    }
    Ok(())
}

fn coordinates_from_feature(feature: &Feature) -> Option<[f64; 2]> {
    let coordinates = match feature.geometry.as_ref()?.value {
        Value::Point(ref coordinates) => coordinates,
        _ => return None,
    };

    Some([*coordinates.first()?, *coordinates.get(1)?])
}

fn point_id_from_feature(feature: &Feature) -> Option<String> {
    feature
        .property("pointId")
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .or_else(|| match &feature.id {
            Some(Id::String(value)) => Some(value.clone()),
            Some(Id::Number(value)) => Some(value.to_string()),
            None => None,
        })
}

fn read_usize_property(feature: &Feature, key: &str) -> Result<usize> {
    feature
        .property(key)
        .and_then(|value| value.as_u64())
        .map(|value| value as usize)
        .ok_or_else(|| invalid_argument(format!("cluster feature missing `{key}`")))
}

fn sum_metrics<'a>(
    records: impl IntoIterator<Item = &'a GeoVizMetricRecord>,
    metric_keys: &[String],
) -> GeoVizMetricRecord {
    let mut metrics = metric_keys
        .iter()
        .map(|key| (key.clone(), 0.0))
        .collect::<GeoVizMetricRecord>();

    for record in records {
        for key in metric_keys {
            *metrics.entry(key.clone()).or_insert(0.0) += record.get(key).copied().unwrap_or(0.0);
        }
    }

    metrics
}

fn summarize_features(
    query: GeoVizViewportQuery,
    features: &[GeoVizAggregationFeature],
    metric_keys: &[String],
) -> GeoVizAggregationSummary {
    let mut metrics = metric_keys
        .iter()
        .map(|key| (key.clone(), 0.0))
        .collect::<GeoVizMetricRecord>();
    let mut visible_point_count = 0;
    let mut visible_cluster_count = 0;
    let mut visible_unclustered_count = 0;

    for feature in features {
        let (point_count, feature_metrics) = match feature {
            GeoVizAggregationFeature::Point { metrics, .. } => {
                visible_unclustered_count += 1;
                (1, metrics)
            }
            GeoVizAggregationFeature::Cluster {
                metrics,
                point_count,
                ..
            } => {
                visible_cluster_count += 1;
                (*point_count, metrics)
            }
        };
        visible_point_count += point_count;
        for key in metric_keys {
            *metrics.entry(key.clone()).or_insert(0.0) +=
                feature_metrics.get(key).copied().unwrap_or(0.0);
        }
    }

    GeoVizAggregationSummary {
        bounds: query.bounds,
        zoom: query.zoom,
        metrics,
        visible_point_count,
        visible_cluster_count,
        visible_unclustered_count,
    }
}

fn feature_key(feature: &GeoVizAggregationFeature) -> String {
    match feature {
        GeoVizAggregationFeature::Point { point, .. } => format!("point:{}", point.id),
        GeoVizAggregationFeature::Cluster { cluster_id, .. } => format!("cluster:{cluster_id}"),
    }
}

fn abbreviate_count(count: usize) -> String {
    if count >= 10_000 {
        format!("{}k", count / 1_000)
    } else if count >= 1_000 {
        format!("{:.1}k", count as f64 / 1_000.0)
    } else {
        count.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(id: &str, longitude: f64, latitude: f64, value: f64) -> GeoVizPoint {
        GeoVizPoint {
            id: Some(id.to_string()),
            label: Some(id.to_string()),
            longitude,
            latitude,
            metrics: BTreeMap::from([("value".to_string(), value)]),
            properties: json!({"id": id}),
        }
    }

    #[test]
    fn reports_bounds_and_lookup() {
        let index = GeoPointIndex::new(
            [point("a", 13.0, 52.0, 2.0), point("b", 14.0, 53.0, 3.0)],
            GeoVizAggregationOptions::default(),
        )
        .expect("index");

        assert_eq!(index.get_bounds(), Some([13.0, 52.0, 14.0, 53.0]));
        assert_eq!(index.get_point_by_id("a").unwrap().metrics["value"], 2.0);
    }

    #[test]
    fn rejects_invalid_coordinates() {
        let error = GeoPointIndex::new(
            [point("bad", 181.0, 52.0, 1.0)],
            GeoVizAggregationOptions::default(),
        )
        .expect_err("invalid longitude");
        assert!(error.to_string().contains("longitude"));
    }

    #[test]
    fn aggregates_cluster_metrics_and_leaves() {
        let index = GeoPointIndex::new(
            [
                point("a", 13.0, 52.0, 2.0),
                point("b", 13.0001, 52.0001, 3.0),
                point("c", 13.0002, 52.0002, 5.0),
            ],
            GeoVizAggregationOptions {
                radius: Some(80.0),
                ..GeoVizAggregationOptions::default()
            },
        )
        .expect("index");
        let aggregation = index
            .get_viewport_aggregation(GeoVizViewportQuery {
                bounds: [12.9, 51.9, 13.1, 52.1],
                zoom: 1.0,
            })
            .expect("aggregation");
        let cluster = aggregation
            .features
            .iter()
            .find_map(|feature| match feature {
                GeoVizAggregationFeature::Cluster {
                    cluster_id,
                    metrics,
                    point_count,
                    ..
                } => Some((*cluster_id, metrics.clone(), *point_count)),
                _ => None,
            })
            .expect("cluster");

        assert_eq!(cluster.1["value"], 10.0);
        assert_eq!(cluster.2, 3);
        assert_eq!(index.get_cluster_leaves(cluster.0, 2, 1).len(), 2);
        assert!(index.get_cluster_expansion_zoom(cluster.0) >= 1);
    }

    #[test]
    fn supports_antimeridian_bounds() {
        let index = GeoPointIndex::new(
            [
                point("west", -179.8, 10.0, 2.0),
                point("east", 179.8, 10.0, 3.0),
            ],
            GeoVizAggregationOptions::default(),
        )
        .expect("index");
        let aggregation = index
            .get_viewport_aggregation(GeoVizViewportQuery {
                bounds: [179.0, 0.0, -179.0, 20.0],
                zoom: 8.0,
            })
            .expect("aggregation");

        assert_eq!(aggregation.summary.visible_point_count, 2);
    }
}
