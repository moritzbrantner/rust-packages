//! Media-neutral annotation envelopes, storage, and temporal queries.
//!
//! Domain-specific findings remain owned by their audio, text, image, vision,
//! and video crates. This module provides the small interoperability layer that
//! lets those findings coexist on one exact media timeline.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

use crate::{AnalysisEvent, MediaRange, Timebase, Timestamp};

/// Current serialized annotation-dataset schema version.
pub const ANNOTATION_SCHEMA_VERSION: u32 = 1;

/// Errors produced by annotation validation, storage, and timing operations.
#[derive(Debug, Error)]
pub enum AnnotationError {
    /// Annotation data violates the neutral contract.
    #[error("invalid annotation data: {0}")]
    Invalid(String),
    /// Shared media timing validation failed.
    #[error(transparent)]
    Media(#[from] crate::DetectError),
    /// An I/O operation failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// JSON parsing or formatting failed.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

/// Result type for media annotation operations.
pub type Result<T> = std::result::Result<T, AnnotationError>;

/// Identifies the media source or stream to which an annotation belongs.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaSourceRef {
    /// Stable source identifier, when one exists.
    #[serde(default)]
    pub source_id: Option<String>,
    /// Stable stream identifier within a source, when one exists.
    #[serde(default)]
    pub stream_id: Option<String>,
    /// Source URI or external locator.
    #[serde(default)]
    pub uri: Option<String>,
    /// Caller-defined source kind such as `video`, `audio`, or `transcript`.
    #[serde(default)]
    pub source_kind: Option<String>,
    /// Source-specific metadata that does not belong in the neutral contract.
    #[serde(default)]
    pub attributes: BTreeMap<String, String>,
}

impl MediaSourceRef {
    /// Creates a source reference with a source id.
    pub fn source(source_id: impl Into<String>) -> Self {
        Self {
            source_id: Some(source_id.into()),
            ..Self::default()
        }
    }

    /// Creates a source reference with a stream id.
    pub fn stream(stream_id: impl Into<String>) -> Self {
        Self {
            stream_id: Some(stream_id.into()),
            ..Self::default()
        }
    }

    /// Adds a URI.
    pub fn uri(mut self, uri: impl Into<String>) -> Self {
        self.uri = Some(uri.into());
        self
    }

    /// Adds a source kind.
    pub fn source_kind(mut self, source_kind: impl Into<String>) -> Self {
        self.source_kind = Some(source_kind.into());
        self
    }

    /// Adds source-specific metadata.
    pub fn attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }

    /// Validates source identifiers and attribute keys.
    pub fn validate(&self) -> Result<()> {
        validate_optional_non_empty(self.source_id.as_deref(), "source id")?;
        validate_optional_non_empty(self.stream_id.as_deref(), "stream id")?;
        validate_optional_non_empty(self.uri.as_deref(), "source uri")?;
        validate_optional_non_empty(self.source_kind.as_deref(), "source kind")?;
        validate_attribute_keys(&self.attributes, "source attribute")
    }
}

/// Provenance describing how an annotation was produced.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnotationProvenance {
    /// Analyzer or producer name.
    #[serde(default)]
    pub analyzer: Option<String>,
    /// Operation or pipeline stage name.
    #[serde(default)]
    pub operation: Option<String>,
    /// Model identifier, when a model produced the result.
    #[serde(default)]
    pub model_id: Option<String>,
    /// Runtime or backend identifier.
    #[serde(default)]
    pub runtime: Option<String>,
    /// Producer or model version.
    #[serde(default)]
    pub version: Option<String>,
}

impl AnnotationProvenance {
    /// Creates provenance for an analyzer.
    pub fn analyzer(analyzer: impl Into<String>) -> Self {
        Self {
            analyzer: Some(analyzer.into()),
            ..Self::default()
        }
    }

    /// Validates all populated provenance fields.
    pub fn validate(&self) -> Result<()> {
        validate_optional_non_empty(self.analyzer.as_deref(), "provenance analyzer")?;
        validate_optional_non_empty(self.operation.as_deref(), "provenance operation")?;
        validate_optional_non_empty(self.model_id.as_deref(), "provenance model id")?;
        validate_optional_non_empty(self.runtime.as_deref(), "provenance runtime")?;
        validate_optional_non_empty(self.version.as_deref(), "provenance version")
    }
}

/// Domain-neutral selector locating an annotation within a media source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AnnotationSelector {
    /// A video-frame index.
    Frame {
        /// Zero-based frame index.
        frame_index: u64,
    },
    /// A text segment within a timed or untimed stream.
    TextSegment {
        /// Text stream identifier.
        stream_id: String,
        /// Stable segment index within the stream.
        segment_index: u64,
    },
    /// A half-open UTF-8 text span.
    TextSpan {
        /// Inclusive byte offset.
        byte_start: usize,
        /// Exclusive byte offset.
        byte_end: usize,
        /// Optional inclusive Unicode-scalar offset.
        #[serde(default)]
        char_start: Option<usize>,
        /// Optional exclusive Unicode-scalar offset.
        #[serde(default)]
        char_end: Option<usize>,
    },
    /// A two-dimensional region in a caller-declared coordinate space.
    Region2d {
        /// Horizontal origin.
        x: f64,
        /// Vertical origin.
        y: f64,
        /// Region width.
        width: f64,
        /// Region height.
        height: f64,
        /// Coordinate-space identifier such as `pixels` or `normalized`.
        #[serde(default)]
        coordinate_space: Option<String>,
    },
    /// A stable track identifier.
    Track {
        /// Track identifier.
        track_id: String,
    },
    /// A domain extension that remains opaque to the neutral layer.
    Custom {
        /// Selector kind owned by the domain adapter.
        selector_kind: String,
        /// Structured selector fields.
        #[serde(default)]
        fields: BTreeMap<String, serde_json::Value>,
    },
}

impl AnnotationSelector {
    /// Validates selector ranges, dimensions, and identifiers.
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Frame { .. } => Ok(()),
            Self::TextSegment { stream_id, .. } => {
                validate_non_empty(stream_id, "text segment stream id")
            }
            Self::TextSpan {
                byte_start,
                byte_end,
                char_start,
                char_end,
            } => {
                if byte_end < byte_start {
                    return Err(AnnotationError::Invalid(
                        "text span byte_end must not precede byte_start".to_string(),
                    ));
                }
                match (char_start, char_end) {
                    (Some(start), Some(end)) if end < start => Err(AnnotationError::Invalid(
                        "text span char_end must not precede char_start".to_string(),
                    )),
                    (Some(_), None) | (None, Some(_)) => Err(AnnotationError::Invalid(
                        "text span char_start and char_end must be provided together".to_string(),
                    )),
                    _ => Ok(()),
                }
            }
            Self::Region2d {
                x,
                y,
                width,
                height,
                coordinate_space,
            } => {
                if [*x, *y, *width, *height]
                    .into_iter()
                    .any(|value| !value.is_finite())
                    || *width <= 0.0
                    || *height <= 0.0
                {
                    return Err(AnnotationError::Invalid(
                        "2D regions must be finite with positive width and height".to_string(),
                    ));
                }
                validate_optional_non_empty(coordinate_space.as_deref(), "region coordinate space")
            }
            Self::Track { track_id } => validate_non_empty(track_id, "track id"),
            Self::Custom {
                selector_kind,
                fields,
            } => {
                validate_non_empty(selector_kind, "custom selector kind")?;
                for key in fields.keys() {
                    validate_non_empty(key, "custom selector field")?;
                }
                Ok(())
            }
        }
    }
}

/// Exact temporal anchor for an annotation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnnotationTiming {
    /// A finding at one exact media instant.
    Instant(Timestamp),
    /// A finding spanning a half-open media range.
    Range(MediaRange),
}

impl AnnotationTiming {
    /// Validates the underlying media timing.
    pub fn validate(self) -> Result<()> {
        match self {
            Self::Instant(timestamp) => timestamp.validate()?,
            Self::Range(range) => range.validate()?,
        }
        Ok(())
    }

    /// Returns the first timestamp represented by this timing.
    pub const fn start_timestamp(self) -> Timestamp {
        match self {
            Self::Instant(timestamp) => timestamp,
            Self::Range(range) => range.start,
        }
    }

    /// Returns whether this timing contains an exact timestamp.
    pub fn contains(self, timestamp: Timestamp) -> Result<bool> {
        match self {
            Self::Instant(instant) => Ok(instant.same_instant(timestamp)?),
            Self::Range(range) => Ok(range.contains(timestamp)?),
        }
    }

    /// Returns whether this timing intersects a query range.
    pub fn overlaps(self, range: MediaRange) -> Result<bool> {
        match self {
            Self::Instant(timestamp) => Ok(range.contains(timestamp)?),
            Self::Range(annotation_range) => Ok(annotation_range.overlaps(range)?),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct TimebaseWire {
    num: i32,
    den: i32,
}

impl From<Timebase> for TimebaseWire {
    fn from(value: Timebase) -> Self {
        Self {
            num: value.num,
            den: value.den,
        }
    }
}

impl TimebaseWire {
    fn into_timebase(self) -> crate::Result<Timebase> {
        Timebase::try_new(self.num, self.den)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct TimestampWire {
    pts: i64,
    timebase: TimebaseWire,
}

impl From<Timestamp> for TimestampWire {
    fn from(value: Timestamp) -> Self {
        Self {
            pts: value.pts,
            timebase: value.timebase.into(),
        }
    }
}

impl TimestampWire {
    fn into_timestamp(self) -> crate::Result<Timestamp> {
        Timestamp::try_new(self.pts, self.timebase.into_timebase()?)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum AnnotationTimingWire {
    Instant {
        timestamp: TimestampWire,
    },
    Range {
        start: TimestampWire,
        end: TimestampWire,
    },
}

impl From<AnnotationTiming> for AnnotationTimingWire {
    fn from(value: AnnotationTiming) -> Self {
        match value {
            AnnotationTiming::Instant(timestamp) => Self::Instant {
                timestamp: timestamp.into(),
            },
            AnnotationTiming::Range(range) => Self::Range {
                start: range.start.into(),
                end: range.end.into(),
            },
        }
    }
}

impl AnnotationTimingWire {
    fn into_timing(self) -> crate::Result<AnnotationTiming> {
        match self {
            Self::Instant { timestamp } => {
                Ok(AnnotationTiming::Instant(timestamp.into_timestamp()?))
            }
            Self::Range { start, end } => Ok(AnnotationTiming::Range(MediaRange::new(
                start.into_timestamp()?,
                end.into_timestamp()?,
            )?)),
        }
    }
}

impl Serialize for AnnotationTiming {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        AnnotationTimingWire::from(*self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for AnnotationTiming {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error as _;

        AnnotationTimingWire::deserialize(deserializer)?
            .into_timing()
            .map_err(D::Error::custom)
    }
}

/// Typed or structured value carried by an annotation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum AnnotationValue {
    /// Human-readable text.
    Text(String),
    /// Floating-point feature or measurement.
    Number(f64),
    /// Integer feature or measurement.
    Integer(i64),
    /// Boolean finding.
    Boolean(bool),
    /// Collection of labels or categories.
    Labels(Vec<String>),
    /// Domain-specific structured payload.
    Json(serde_json::Value),
}

impl AnnotationValue {
    /// Validates finite numeric values and non-empty labels.
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Number(value) if !value.is_finite() => Err(AnnotationError::Invalid(
                "annotation numeric values must be finite".to_string(),
            )),
            Self::Labels(labels) => {
                for label in labels {
                    validate_non_empty(label, "annotation value label")?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

/// A neutral finding that can coexist with annotations from any media domain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaAnnotation {
    /// Stable identifier within an annotation dataset.
    pub id: String,
    /// Stable machine-readable annotation kind.
    pub kind: String,
    /// Optional human-readable or detector-provided label.
    #[serde(default)]
    pub label: Option<String>,
    /// Optional exact temporal location.
    #[serde(default)]
    pub timing: Option<AnnotationTiming>,
    /// Optional source or stream identity.
    #[serde(default)]
    pub source: Option<MediaSourceRef>,
    /// Optional location within the source.
    #[serde(default)]
    pub selector: Option<AnnotationSelector>,
    /// Optional finite analyzer score. The neutral layer does not assume a 0..=1 scale.
    #[serde(default)]
    pub score: Option<f32>,
    /// Producer/model provenance entries.
    #[serde(default)]
    pub provenance: Vec<AnnotationProvenance>,
    /// Optional typed or structured result payload.
    #[serde(default)]
    pub value: Option<AnnotationValue>,
    /// Domain- or adapter-specific string metadata.
    #[serde(default)]
    pub attributes: BTreeMap<String, String>,
}

impl MediaAnnotation {
    /// Creates an untimed annotation with no optional metadata.
    pub fn new(id: impl Into<String>, kind: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind: kind.into(),
            label: None,
            timing: None,
            source: None,
            selector: None,
            score: None,
            provenance: Vec::new(),
            value: None,
            attributes: BTreeMap::new(),
        }
    }

    /// Adds a label.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Anchors the annotation at one instant.
    pub fn at(mut self, timestamp: Timestamp) -> Self {
        self.timing = Some(AnnotationTiming::Instant(timestamp));
        self
    }

    /// Anchors the annotation to a half-open range.
    pub fn during(mut self, range: MediaRange) -> Self {
        self.timing = Some(AnnotationTiming::Range(range));
        self
    }

    /// Adds source identity.
    pub fn source(mut self, source: MediaSourceRef) -> Self {
        self.source = Some(source);
        self
    }

    /// Adds a source selector.
    pub fn selector(mut self, selector: AnnotationSelector) -> Self {
        self.selector = Some(selector);
        self
    }

    /// Adds an analyzer score.
    pub fn score(mut self, score: f32) -> Self {
        self.score = Some(score);
        self
    }

    /// Adds producer provenance.
    pub fn provenance(mut self, provenance: AnnotationProvenance) -> Self {
        self.provenance.push(provenance);
        self
    }

    /// Adds a typed or structured value.
    pub fn value(mut self, value: AnnotationValue) -> Self {
        self.value = Some(value);
        self
    }

    /// Adds adapter-specific metadata.
    pub fn attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }

    /// Returns the annotation's first timestamp, when timed.
    pub fn start_timestamp(&self) -> Option<Timestamp> {
        self.timing.map(AnnotationTiming::start_timestamp)
    }

    /// Validates the complete annotation envelope.
    pub fn validate(&self) -> Result<()> {
        validate_non_empty(&self.id, "annotation id")?;
        validate_non_empty(&self.kind, "annotation kind")?;
        validate_optional_non_empty(self.label.as_deref(), "annotation label")?;
        if let Some(timing) = self.timing {
            timing.validate()?;
        }
        if let Some(source) = &self.source {
            source.validate()?;
        }
        if let Some(selector) = &self.selector {
            selector.validate()?;
        }
        if self.score.is_some_and(|score| !score.is_finite()) {
            return Err(AnnotationError::Invalid(
                "annotation score must be finite".to_string(),
            ));
        }
        for provenance in &self.provenance {
            provenance.validate()?;
        }
        if let Some(value) = &self.value {
            value.validate()?;
        }
        validate_attribute_keys(&self.attributes, "annotation attribute")
    }

    /// Validates and returns this annotation.
    pub fn validated(self) -> Result<Self> {
        self.validate()?;
        Ok(self)
    }

    /// Converts the legacy neutral `AnalysisEvent` into the canonical annotation envelope.
    pub fn from_analysis_event(id: impl Into<String>, event: AnalysisEvent) -> Result<Self> {
        let AnalysisEvent {
            timestamp,
            analyzer,
            label,
            score,
        } = event;
        let mut annotation = Self::new(id, "analysis_event")
            .label(label)
            .provenance(AnnotationProvenance::analyzer(analyzer));
        if let Some(timestamp) = timestamp {
            annotation = annotation.at(timestamp);
        }
        if let Some(score) = score {
            annotation = annotation.score(score);
        }
        annotation.validated()
    }
}

/// A validated collection of annotations from one or more media analyzers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnotationDataset {
    /// Serialized schema version.
    pub schema_version: u32,
    /// Optional human-readable dataset name.
    #[serde(default)]
    pub name: Option<String>,
    /// Optional source shared by the dataset.
    #[serde(default)]
    pub source: Option<MediaSourceRef>,
    /// Dataset-level metadata.
    #[serde(default)]
    pub attributes: BTreeMap<String, String>,
    /// Timed and untimed annotations.
    #[serde(default)]
    pub annotations: Vec<MediaAnnotation>,
}

impl Default for AnnotationDataset {
    fn default() -> Self {
        Self {
            schema_version: ANNOTATION_SCHEMA_VERSION,
            name: None,
            source: None,
            attributes: BTreeMap::new(),
            annotations: Vec::new(),
        }
    }
}

impl AnnotationDataset {
    /// Creates an empty dataset using the current schema.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds one annotation after validating it and its id uniqueness.
    pub fn push(&mut self, annotation: MediaAnnotation) -> Result<()> {
        annotation.validate()?;
        if self
            .annotations
            .iter()
            .any(|existing| existing.id == annotation.id)
        {
            return Err(AnnotationError::Invalid(format!(
                "duplicate annotation id `{}`",
                annotation.id
            )));
        }
        self.annotations.push(annotation);
        Ok(())
    }

    /// Adds several annotations with the same validation as [`Self::push`].
    pub fn extend(&mut self, annotations: impl IntoIterator<Item = MediaAnnotation>) -> Result<()> {
        for annotation in annotations {
            self.push(annotation)?;
        }
        Ok(())
    }

    /// Validates schema version, metadata, annotation ids, and all records.
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != ANNOTATION_SCHEMA_VERSION {
            return Err(AnnotationError::Invalid(format!(
                "unsupported annotation schema version {}",
                self.schema_version
            )));
        }
        validate_optional_non_empty(self.name.as_deref(), "dataset name")?;
        if let Some(source) = &self.source {
            source.validate()?;
        }
        validate_attribute_keys(&self.attributes, "dataset attribute")?;

        let mut ids = BTreeSet::new();
        for annotation in &self.annotations {
            annotation.validate()?;
            if !ids.insert(annotation.id.as_str()) {
                return Err(AnnotationError::Invalid(format!(
                    "duplicate annotation id `{}`",
                    annotation.id
                )));
            }
        }
        Ok(())
    }

    /// Sorts timed annotations chronologically and leaves untimed annotations last.
    pub fn sort_by_time(&mut self) -> Result<()> {
        self.validate()?;
        self.annotations.sort_by(|left, right| {
            match (left.start_timestamp(), right.start_timestamp()) {
                (Some(left_time), Some(right_time)) => left_time
                    .chronological_cmp(right_time)
                    .unwrap_or(Ordering::Equal)
                    .then_with(|| left.id.cmp(&right.id)),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => left.id.cmp(&right.id),
            }
        });
        Ok(())
    }

    /// Returns annotations whose timing contains the supplied instant.
    pub fn at(&self, timestamp: Timestamp) -> Result<Vec<&MediaAnnotation>> {
        self.validate()?;
        timestamp.validate()?;
        let mut matches = Vec::new();
        for annotation in &self.annotations {
            if let Some(timing) = annotation.timing {
                if timing.contains(timestamp)? {
                    matches.push(annotation);
                }
            }
        }
        Ok(matches)
    }

    /// Returns annotations whose timing intersects the supplied half-open range.
    pub fn overlapping(&self, range: MediaRange) -> Result<Vec<&MediaAnnotation>> {
        self.validate()?;
        range.validate()?;
        let mut matches = Vec::new();
        for annotation in &self.annotations {
            if let Some(timing) = annotation.timing {
                if timing.overlaps(range)? {
                    matches.push(annotation);
                }
            }
        }
        Ok(matches)
    }

    /// Returns annotations of one exact machine-readable kind.
    pub fn by_kind<'a>(&'a self, kind: &'a str) -> impl Iterator<Item = &'a MediaAnnotation> + 'a {
        self.annotations
            .iter()
            .filter(move |annotation| annotation.kind == kind)
    }

    /// Merges datasets without silently resolving duplicate annotation ids.
    pub fn merge(datasets: &[Self]) -> Result<Self> {
        for dataset in datasets {
            dataset.validate()?;
        }

        let mut merged = Self::new();
        if let Some(first) = datasets.first() {
            merged.name = first.name.clone();
            if datasets
                .iter()
                .all(|dataset| dataset.source == first.source)
            {
                merged.source = first.source.clone();
            }
        }
        for dataset in datasets {
            for annotation in &dataset.annotations {
                merged.push(annotation.clone())?;
            }
        }
        Ok(merged)
    }
}

/// Writes one complete annotation dataset as pretty JSON.
pub fn write_json(mut writer: impl Write, dataset: &AnnotationDataset) -> Result<()> {
    dataset.validate()?;
    serde_json::to_writer_pretty(&mut writer, dataset)?;
    writer.write_all(b"\n")?;
    Ok(())
}

/// Reads and validates one complete annotation dataset from JSON.
pub fn read_json(reader: impl Read) -> Result<AnnotationDataset> {
    let dataset = serde_json::from_reader::<_, AnnotationDataset>(reader)?;
    dataset.validate()?;
    Ok(dataset)
}

/// Writes annotation records as JSON Lines.
///
/// Dataset-level metadata is intentionally omitted from this streaming format.
pub fn write_jsonl(mut writer: impl Write, dataset: &AnnotationDataset) -> Result<()> {
    dataset.validate()?;
    for annotation in &dataset.annotations {
        serde_json::to_writer(&mut writer, annotation)?;
        writer.write_all(b"\n")?;
    }
    Ok(())
}

/// Reads annotation records from JSON Lines into a dataset with default metadata.
pub fn read_jsonl(reader: impl Read) -> Result<AnnotationDataset> {
    let reader = BufReader::new(reader);
    let mut dataset = AnnotationDataset::new();
    for (line_index, line) in reader.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let annotation = serde_json::from_str::<MediaAnnotation>(&line).map_err(|error| {
            AnnotationError::Invalid(format!(
                "invalid annotation JSONL at line {}: {error}",
                line_index + 1
            ))
        })?;
        dataset.push(annotation)?;
    }
    Ok(dataset)
}

/// Writes a complete annotation dataset to a JSON file.
pub fn write_json_file(path: impl AsRef<Path>, dataset: &AnnotationDataset) -> Result<()> {
    write_json(File::create(path)?, dataset)
}

/// Reads and validates a complete annotation dataset from a JSON file.
pub fn read_json_file(path: impl AsRef<Path>) -> Result<AnnotationDataset> {
    read_json(File::open(path)?)
}

/// Writes annotation records to a JSONL file.
pub fn write_jsonl_file(path: impl AsRef<Path>, dataset: &AnnotationDataset) -> Result<()> {
    write_jsonl(File::create(path)?, dataset)
}

/// Reads annotation records from a JSONL file.
pub fn read_jsonl_file(path: impl AsRef<Path>) -> Result<AnnotationDataset> {
    read_jsonl(File::open(path)?)
}

fn validate_non_empty(value: &str, field: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(AnnotationError::Invalid(format!(
            "{field} must not be empty"
        )))
    } else {
        Ok(())
    }
}

fn validate_optional_non_empty(value: Option<&str>, field: &str) -> Result<()> {
    if let Some(value) = value {
        validate_non_empty(value, field)?;
    }
    Ok(())
}

fn validate_attribute_keys(attributes: &BTreeMap<String, String>, field: &str) -> Result<()> {
    for key in attributes.keys() {
        validate_non_empty(key, field)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timestamp(pts: i64, num: i32, den: i32) -> Timestamp {
        Timestamp::try_new(pts, Timebase::try_new(num, den).unwrap()).unwrap()
    }

    #[test]
    fn json_round_trip_preserves_exact_range_and_selector() {
        let mut dataset = AnnotationDataset::new();
        dataset.name = Some("fixture".to_string());
        dataset
            .push(
                MediaAnnotation::new("face-1", "face")
                    .label("Alice")
                    .during(MediaRange::new(timestamp(500, 1, 1_000), timestamp(2, 1, 1)).unwrap())
                    .source(MediaSourceRef::stream("video-0").source_kind("video"))
                    .selector(AnnotationSelector::Region2d {
                        x: 10.0,
                        y: 20.0,
                        width: 100.0,
                        height: 120.0,
                        coordinate_space: Some("pixels".to_string()),
                    })
                    .score(0.95)
                    .provenance(AnnotationProvenance::analyzer("face-detector"))
                    .value(AnnotationValue::Text("Alice".to_string())),
            )
            .unwrap();

        let mut encoded = Vec::new();
        write_json(&mut encoded, &dataset).unwrap();
        let decoded = read_json(encoded.as_slice()).unwrap();

        assert_eq!(decoded, dataset);
    }

    #[test]
    fn temporal_queries_compare_different_timebases_exactly() {
        let mut dataset = AnnotationDataset::new();
        dataset
            .push(MediaAnnotation::new("instant", "beat").at(timestamp(1, 1, 1)))
            .unwrap();
        dataset
            .push(MediaAnnotation::new("range", "speech").during(
                MediaRange::new(timestamp(1_500, 1, 1_000), timestamp(2_500, 1, 1_000)).unwrap(),
            ))
            .unwrap();

        let query = MediaRange::new(timestamp(900, 1, 1_000), timestamp(2, 1, 1)).unwrap();
        let ids = dataset
            .overlapping(query)
            .unwrap()
            .into_iter()
            .map(|annotation| annotation.id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(ids, vec!["instant", "range"]);
        assert_eq!(dataset.at(timestamp(2, 1, 1)).unwrap().len(), 1);
    }

    #[test]
    fn analysis_events_adapt_without_losing_timing_or_score() {
        let event = AnalysisEvent::new("rhythm", "beat")
            .at_timestamp(timestamp(24, 1, 24))
            .score(0.8);
        let annotation = MediaAnnotation::from_analysis_event("beat-24", event).unwrap();

        assert_eq!(annotation.kind, "analysis_event");
        assert_eq!(annotation.label.as_deref(), Some("beat"));
        assert_eq!(annotation.score, Some(0.8));
        assert!(annotation
            .start_timestamp()
            .unwrap()
            .same_instant(timestamp(1, 1, 1))
            .unwrap());
        assert_eq!(annotation.provenance[0].analyzer.as_deref(), Some("rhythm"));
    }

    #[test]
    fn jsonl_round_trip_preserves_annotations_but_not_dataset_metadata() {
        let mut dataset = AnnotationDataset::new();
        dataset.name = Some("not-streamed".to_string());
        dataset
            .push(MediaAnnotation::new("a", "marker").at(timestamp(5, 1, 10)))
            .unwrap();

        let mut encoded = Vec::new();
        write_jsonl(&mut encoded, &dataset).unwrap();
        let decoded = read_jsonl(encoded.as_slice()).unwrap();

        assert_eq!(decoded.name, None);
        assert_eq!(decoded.annotations, dataset.annotations);
    }

    #[test]
    fn duplicate_ids_are_rejected() {
        let mut dataset = AnnotationDataset::new();
        dataset.push(MediaAnnotation::new("same", "one")).unwrap();
        assert!(dataset.push(MediaAnnotation::new("same", "two")).is_err());
    }
}
