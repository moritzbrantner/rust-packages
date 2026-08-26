//! Migration from retained video-analysis records to neutral media annotations.
//!
//! This adapter is intentionally video-owned: `media-core` stays independent
//! of video contracts while existing datasets can move onto the common media
//! annotation envelope without a flag-day rewrite.

use media_core::annotations::{
    AnnotationDataset, AnnotationProvenance, AnnotationSelector, AnnotationTiming,
    AnnotationValue, MediaAnnotation, MediaSourceRef, Result,
};
use media_core::{MediaRange, Timebase, Timestamp};
use video_analysis_dataset::{
    AnalysisDataset, BoundingBoxRecord, DatasetRecord, TimestampRecord,
};

/// Converts an existing retained video-analysis dataset into neutral annotations.
///
/// Common timing, source, selector, label, score, and analyzer information is
/// promoted into the neutral envelope. The complete legacy record is also
/// retained as structured JSON in [`AnnotationValue::Json`], so conversion is
/// lossless even when a field remains domain-specific.
pub fn annotation_dataset_from_video_dataset(
    dataset: &AnalysisDataset,
) -> Result<AnnotationDataset> {
    let mut annotations = AnnotationDataset::new();
    annotations.name = dataset.metadata.name.clone();
    annotations.attributes = dataset.metadata.attributes.clone();
    annotations.attributes.insert(
        "legacySchema".to_string(),
        "video-analysis-dataset".to_string(),
    );
    annotations.attributes.insert(
        "legacySchemaVersion".to_string(),
        dataset.metadata.schema_version.to_string(),
    );
    if let Some(created_at) = &dataset.metadata.created_at {
        annotations
            .attributes
            .insert("legacyCreatedAt".to_string(), created_at.clone());
    }
    if let Some(source) = &dataset.metadata.source {
        annotations.source = Some(
            MediaSourceRef::default()
                .uri(source.clone())
                .source_kind("legacy_video_analysis_dataset"),
        );
    }

    for (index, record) in dataset.records.iter().enumerate() {
        let mut annotation = MediaAnnotation::new(
            format!("legacy:{index}:{}", record.kind()),
            record.kind(),
        );

        if let Some(label) = record_label(record) {
            annotation = annotation.label(label);
        }
        if let Some(timing) = record_timing(record)? {
            annotation.timing = Some(timing);
        }
        if let Some(source) = record_source(record) {
            annotation = annotation.source(source);
        }
        if let Some(selector) = record_selector(record) {
            annotation = annotation.selector(selector);
        }
        if let Some(score) = record_score(record) {
            annotation = annotation.score(score);
        }
        if let Some(provenance) = record_provenance(record) {
            annotation = annotation.provenance(provenance);
        }
        annotation.attributes = record_attributes(record);
        annotation.attributes.insert(
            "legacyRecordKind".to_string(),
            record.kind().to_string(),
        );
        annotation = annotation.value(AnnotationValue::Json(serde_json::to_value(record)?));
        annotations.push(annotation)?;
    }

    annotations.validate()?;
    Ok(annotations)
}

fn record_timing(record: &DatasetRecord) -> Result<Option<AnnotationTiming>> {
    let timing = match record {
        DatasetRecord::VideoFrame(record) => Some(AnnotationTiming::Instant(timestamp(
            record.position.timestamp,
        )?)),
        DatasetRecord::AudioFrame(record) => {
            Some(AnnotationTiming::Instant(timestamp(record.timestamp)?))
        }
        DatasetRecord::TextSegment(record) => record
            .timestamp
            .map(timestamp)
            .transpose()?
            .map(AnnotationTiming::Instant),
        DatasetRecord::Scene(record) => Some(AnnotationTiming::Range(MediaRange::new(
            timestamp(record.start.timestamp)?,
            timestamp(record.end.timestamp)?,
        )?)),
        DatasetRecord::Cut(record) => Some(AnnotationTiming::Instant(timestamp(
            record.position.timestamp,
        )?)),
        DatasetRecord::Observation(record) => record
            .timestamp
            .or_else(|| record.frame.map(|frame| frame.timestamp))
            .map(timestamp)
            .transpose()?
            .map(AnnotationTiming::Instant),
        DatasetRecord::Event(record) => record
            .timestamp
            .map(timestamp)
            .transpose()?
            .map(AnnotationTiming::Instant),
        DatasetRecord::Metric(_) => None,
        DatasetRecord::Feature(record) => record
            .timestamp
            .map(timestamp)
            .transpose()?
            .map(AnnotationTiming::Instant),
        DatasetRecord::Track(record) => record
            .first_timestamp
            .map(timestamp)
            .transpose()?
            .map(AnnotationTiming::Instant),
        DatasetRecord::Pose2d(record) => record
            .frame
            .map(|frame| frame.timestamp)
            .map(timestamp)
            .transpose()?
            .map(AnnotationTiming::Instant),
        DatasetRecord::Pose3d(record) => record
            .frame
            .map(|frame| frame.timestamp)
            .map(timestamp)
            .transpose()?
            .map(AnnotationTiming::Instant),
    };
    Ok(timing)
}

fn record_source(record: &DatasetRecord) -> Option<MediaSourceRef> {
    match record {
        DatasetRecord::VideoFrame(record) => {
            Some(MediaSourceRef::stream(record.stream_id.clone()).source_kind("video"))
        }
        DatasetRecord::AudioFrame(record) => {
            Some(MediaSourceRef::stream(record.stream_id.clone()).source_kind("audio"))
        }
        DatasetRecord::TextSegment(record) => {
            Some(MediaSourceRef::stream(record.stream_id.clone()).source_kind("text"))
        }
        _ => None,
    }
}

fn record_selector(record: &DatasetRecord) -> Option<AnnotationSelector> {
    match record {
        DatasetRecord::VideoFrame(record) => Some(AnnotationSelector::Frame {
            frame_index: record.position.frame_index,
        }),
        DatasetRecord::AudioFrame(_) => None,
        DatasetRecord::TextSegment(record) => Some(AnnotationSelector::TextSegment {
            stream_id: record.stream_id.clone(),
            segment_index: record.segment_index,
        }),
        DatasetRecord::Scene(record) => Some(AnnotationSelector::Frame {
            frame_index: record.start.frame_index,
        }),
        DatasetRecord::Cut(record) => Some(AnnotationSelector::Frame {
            frame_index: record.position.frame_index,
        }),
        DatasetRecord::Observation(record) => record
            .region
            .map(region_selector)
            .or_else(|| {
                record
                    .track_id
                    .as_ref()
                    .map(|track_id| AnnotationSelector::Track {
                        track_id: track_id.clone(),
                    })
            })
            .or_else(|| {
                record.frame.map(|frame| AnnotationSelector::Frame {
                    frame_index: frame.frame_index,
                })
            }),
        DatasetRecord::Event(_) => None,
        DatasetRecord::Metric(record) => Some(AnnotationSelector::Frame {
            frame_index: record.frame_index,
        }),
        DatasetRecord::Feature(record) => record
            .track_id
            .as_ref()
            .map(|track_id| AnnotationSelector::Track {
                track_id: track_id.clone(),
            })
            .or_else(|| {
                record
                    .frame_index
                    .map(|frame_index| AnnotationSelector::Frame { frame_index })
            }),
        DatasetRecord::Track(record) => Some(AnnotationSelector::Track {
            track_id: record.track_id.clone(),
        }),
        DatasetRecord::Pose2d(record) => record
            .region
            .map(region_selector)
            .or_else(|| {
                record.frame.map(|frame| AnnotationSelector::Frame {
                    frame_index: frame.frame_index,
                })
            }),
        DatasetRecord::Pose3d(record) => record.frame.map(|frame| AnnotationSelector::Frame {
            frame_index: frame.frame_index,
        }),
    }
}

fn record_label(record: &DatasetRecord) -> Option<String> {
    match record {
        DatasetRecord::VideoFrame(_) | DatasetRecord::AudioFrame(_) => None,
        DatasetRecord::TextSegment(record) => Some(record.text.clone()),
        DatasetRecord::Scene(record) => Some(format!("scene {}", record.scene_index)),
        DatasetRecord::Cut(record) => Some(record.detector.clone()),
        DatasetRecord::Observation(record) => record.label.clone().or_else(|| record.text.clone()),
        DatasetRecord::Event(record) => Some(record.label.clone()),
        DatasetRecord::Metric(record) => Some(record.key.clone()),
        DatasetRecord::Feature(record) => Some(record.name.clone()),
        DatasetRecord::Track(record) => record.label.clone(),
        DatasetRecord::Pose2d(record) => record.label.clone(),
        DatasetRecord::Pose3d(record) => record.label.clone(),
    }
}

fn record_score(record: &DatasetRecord) -> Option<f32> {
    match record {
        DatasetRecord::Cut(record) => record.score,
        DatasetRecord::Observation(record) => record.score,
        DatasetRecord::Event(record) => record.score,
        DatasetRecord::Pose2d(record) => record.score,
        DatasetRecord::Pose3d(record) => record.score,
        _ => None,
    }
}

fn record_provenance(record: &DatasetRecord) -> Option<AnnotationProvenance> {
    let analyzer = match record {
        DatasetRecord::Cut(record) => Some(record.detector.as_str()),
        DatasetRecord::Observation(record) => Some(record.analyzer.as_str()),
        DatasetRecord::Event(record) => Some(record.analyzer.as_str()),
        DatasetRecord::Pose2d(record) => Some(record.analyzer.as_str()),
        DatasetRecord::Pose3d(record) => Some(record.analyzer.as_str()),
        _ => None,
    }?;
    Some(AnnotationProvenance::analyzer(analyzer))
}

fn record_attributes(record: &DatasetRecord) -> std::collections::BTreeMap<String, String> {
    match record {
        DatasetRecord::Observation(record) => record.attributes.clone(),
        DatasetRecord::Feature(record) => record.attributes.clone(),
        DatasetRecord::Track(record) => record.attributes.clone(),
        DatasetRecord::Pose2d(record) => record.attributes.clone(),
        DatasetRecord::Pose3d(record) => record.attributes.clone(),
        _ => Default::default(),
    }
}

fn region_selector(region: BoundingBoxRecord) -> AnnotationSelector {
    AnnotationSelector::Region2d {
        x: region.x as f64,
        y: region.y as f64,
        width: region.width as f64,
        height: region.height as f64,
        coordinate_space: Some("pixels".to_string()),
    }
}

fn timestamp(record: TimestampRecord) -> Result<Timestamp> {
    let timebase = Timebase::try_new(record.timebase_num, record.timebase_den)?;
    Ok(Timestamp::try_new(record.pts, timebase)?)
}

#[cfg(test)]
mod tests {
    use media_core::annotations::{AnnotationTiming, AnnotationValue};
    use video_analysis_core::{AnalysisEvent, Timebase, Timestamp};

    use super::*;

    #[test]
    fn converts_legacy_events_losslessly_into_neutral_annotations() {
        let source_timestamp = Timestamp::new(24, Timebase::new(1, 24));
        let mut legacy = AnalysisDataset::empty();
        legacy.extend_events([
            AnalysisEvent::new("fixture", "marker")
                .at_timestamp(source_timestamp)
                .score(0.75),
        ]);

        let converted = annotation_dataset_from_video_dataset(&legacy).unwrap();
        assert_eq!(converted.annotations.len(), 1);
        let annotation = &converted.annotations[0];
        assert_eq!(annotation.kind, "event");
        assert_eq!(annotation.label.as_deref(), Some("marker"));
        assert_eq!(annotation.score, Some(0.75));
        assert!(matches!(
            annotation.timing,
            Some(AnnotationTiming::Instant(_))
        ));
        assert!(annotation
            .start_timestamp()
            .unwrap()
            .same_instant(source_timestamp)
            .unwrap());
        assert_eq!(
            annotation.provenance[0].analyzer.as_deref(),
            Some("fixture")
        );
        assert!(matches!(
            annotation.value,
            Some(AnnotationValue::Json(_))
        ));
    }
}
