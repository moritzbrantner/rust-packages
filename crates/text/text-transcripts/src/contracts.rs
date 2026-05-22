use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use text_core::{AsTextSegmentContract, TextSegmentContract, TimestampContract};
use video_analysis_core::{Timebase, Timestamp};

use crate::{TranscriptSegment, TranscriptWord, TranscriptionError, TranscriptionResult};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptWordContract {
    pub text: String,
    #[serde(default)]
    pub start_seconds: Option<f64>,
    #[serde(default)]
    pub end_seconds: Option<f64>,
    #[serde(default)]
    pub confidence: Option<f32>,
}

impl From<TranscriptWord> for TranscriptWordContract {
    fn from(value: TranscriptWord) -> Self {
        Self {
            text: value.text,
            start_seconds: value.start_seconds,
            end_seconds: value.end_seconds,
            confidence: sanitize_confidence(value.confidence),
        }
    }
}

impl From<TranscriptWordContract> for TranscriptWord {
    fn from(value: TranscriptWordContract) -> Self {
        Self {
            text: value.text,
            start_seconds: value.start_seconds,
            end_seconds: value.end_seconds,
            confidence: sanitize_confidence(value.confidence),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptSegmentContract {
    pub index: u64,
    #[serde(default)]
    pub start_seconds: Option<f64>,
    #[serde(default)]
    pub end_seconds: Option<f64>,
    pub text: String,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub speaker: Option<String>,
    #[serde(default)]
    pub confidence: Option<f32>,
    pub is_final: bool,
    #[serde(default)]
    pub words: Vec<TranscriptWordContract>,
    #[serde(default)]
    pub attributes: BTreeMap<String, String>,
}

impl TranscriptSegmentContract {
    pub fn new(index: u64, text: impl Into<String>) -> Self {
        Self {
            index,
            start_seconds: None,
            end_seconds: None,
            text: text.into(),
            language: None,
            speaker: None,
            confidence: None,
            is_final: true,
            words: Vec::new(),
            attributes: BTreeMap::new(),
        }
    }

    pub fn validate(&self) -> crate::Result<()> {
        validate_seconds_range(self.start_seconds, self.end_seconds)?;
        if self
            .confidence
            .is_some_and(|confidence| !confidence.is_finite())
        {
            return Err(TranscriptionError::InvalidTranscript(
                "transcript segment confidence must be finite".to_string(),
            ));
        }
        for word in &self.words {
            validate_seconds_range(word.start_seconds, word.end_seconds)?;
            if word
                .confidence
                .is_some_and(|confidence| !confidence.is_finite())
            {
                return Err(TranscriptionError::InvalidTranscript(
                    "transcript word confidence must be finite".to_string(),
                ));
            }
        }
        Ok(())
    }

    pub fn validated(mut self) -> crate::Result<Self> {
        self.confidence = sanitize_confidence(self.confidence);
        for word in &mut self.words {
            word.confidence = sanitize_confidence(word.confidence);
        }
        self.validate()?;
        Ok(self)
    }

    pub fn duration_seconds(&self) -> Option<f64> {
        Some((self.end_seconds? - self.start_seconds?).max(0.0))
    }
}

impl AsTextSegmentContract for TranscriptSegmentContract {
    fn as_text_segment_contract(&self) -> TextSegmentContract {
        let mut attributes = self.attributes.clone();
        insert_optional(&mut attributes, "speaker", self.speaker.as_deref());
        insert_optional_display(&mut attributes, "confidence", self.confidence);

        TextSegmentContract {
            stream_id: None,
            segment_index: self.index,
            text: self.text.clone(),
            language: self.language.clone(),
            timestamp: self.start_seconds.map(seconds_to_timestamp_contract),
            duration_seconds: self.duration_seconds(),
            is_final: self.is_final,
            attributes,
        }
    }
}

impl From<TranscriptSegment> for TranscriptSegmentContract {
    fn from(value: TranscriptSegment) -> Self {
        Self {
            index: value.index,
            start_seconds: value.start_seconds,
            end_seconds: value.end_seconds,
            text: value.text,
            language: value.language,
            speaker: value.speaker,
            confidence: sanitize_confidence(value.confidence),
            is_final: value.is_final,
            words: Vec::new(),
            attributes: BTreeMap::new(),
        }
    }
}

impl From<&TranscriptSegment> for TranscriptSegmentContract {
    fn from(value: &TranscriptSegment) -> Self {
        value.clone().into()
    }
}

impl From<TranscriptSegmentContract> for TranscriptSegment {
    fn from(value: TranscriptSegmentContract) -> Self {
        Self {
            index: value.index,
            start_seconds: value.start_seconds,
            end_seconds: value.end_seconds,
            text: value.text,
            language: value.language,
            speaker: value.speaker,
            confidence: sanitize_confidence(value.confidence),
            is_final: value.is_final,
        }
    }
}

impl From<TranscriptSegmentContract> for TextSegmentContract {
    fn from(value: TranscriptSegmentContract) -> Self {
        value.as_text_segment_contract()
    }
}

impl From<&TranscriptSegmentContract> for TextSegmentContract {
    fn from(value: &TranscriptSegmentContract) -> Self {
        value.as_text_segment_contract()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionContract {
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub segments: Vec<TranscriptSegmentContract>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub attributes: BTreeMap<String, String>,
}

impl TranscriptionContract {
    pub fn new(segments: Vec<TranscriptSegmentContract>) -> Self {
        Self {
            text: None,
            language: None,
            segments,
            source: None,
            attributes: BTreeMap::new(),
        }
    }

    pub fn validate(&self) -> crate::Result<()> {
        for segment in &self.segments {
            segment.validate()?;
        }
        Ok(())
    }

    pub fn joined_text(&self) -> String {
        self.segments
            .iter()
            .map(|segment| segment.text.trim())
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

impl From<TranscriptionResult> for TranscriptionContract {
    fn from(value: TranscriptionResult) -> Self {
        Self {
            text: value.text,
            language: value.language,
            segments: value.segments.into_iter().map(Into::into).collect(),
            source: value.source,
            attributes: BTreeMap::new(),
        }
    }
}

impl From<TranscriptionContract> for TranscriptionResult {
    fn from(value: TranscriptionContract) -> Self {
        Self {
            text: value.text,
            language: value.language,
            segments: value.segments.into_iter().map(Into::into).collect(),
            source: value.source,
        }
    }
}

fn seconds_to_timestamp_contract(seconds: f64) -> TimestampContract {
    Timestamp::new((seconds * 1_000.0).round() as i64, Timebase::new(1, 1_000)).into()
}

fn sanitize_confidence(value: Option<f32>) -> Option<f32> {
    value.and_then(|confidence| confidence.is_finite().then(|| confidence.clamp(0.0, 1.0)))
}

fn validate_seconds_range(start: Option<f64>, end: Option<f64>) -> crate::Result<()> {
    if start.is_some_and(|value| !value.is_finite()) || end.is_some_and(|value| !value.is_finite())
    {
        return Err(TranscriptionError::InvalidTranscript(
            "transcript timestamps must be finite".to_string(),
        ));
    }
    if let (Some(start), Some(end)) = (start, end) {
        if end < start {
            return Err(TranscriptionError::InvalidTranscript(
                "transcript segment end_seconds must be greater than or equal to start_seconds"
                    .to_string(),
            ));
        }
    }
    Ok(())
}

fn insert_optional(metadata: &mut BTreeMap<String, String>, key: &str, value: Option<&str>) {
    if let Some(value) = value {
        metadata.insert(key.to_string(), value.to_string());
    }
}

fn insert_optional_display<T: std::fmt::Display>(
    metadata: &mut BTreeMap<String, String>,
    key: &str,
    value: Option<T>,
) {
    if let Some(value) = value {
        metadata.insert(key.to_string(), value.to_string());
    }
}
