use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use text_core::{AsTextSegmentContract, TextSegmentContract, TextSourceRef, TimestampContract};
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

    pub fn normalized(mut self) -> Self {
        self.text = self.text.trim().to_string();
        self.confidence = sanitize_confidence(self.confidence);
        self.words = self
            .words
            .into_iter()
            .filter_map(|mut word| {
                word.text = word.text.trim().to_string();
                word.confidence = sanitize_confidence(word.confidence);
                (!word.text.is_empty()).then_some(word)
            })
            .collect();
        self
    }

    pub fn duration_seconds(&self) -> Option<f64> {
        Some((self.end_seconds? - self.start_seconds?).max(0.0))
    }

    pub fn midpoint_seconds(&self) -> Option<f64> {
        Some((self.start_seconds? + self.end_seconds?) * 0.5)
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
            source: Some(TextSourceRef {
                source_id: None,
                source_kind: Some("transcript_segment".to_string()),
                uri: None,
                media_timestamp: self.start_seconds.map(seconds_to_timestamp_contract),
                duration_seconds: self.duration_seconds(),
            }),
            provenance: Vec::new(),
            annotations: Vec::new(),
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

    pub fn from_segments(
        source: Option<String>,
        language: Option<String>,
        segments: Vec<TranscriptSegmentContract>,
    ) -> crate::Result<Self> {
        let segments = segments
            .into_iter()
            .map(|mut segment| {
                if segment.language.is_none() {
                    segment.language = language.clone();
                }
                segment
            })
            .collect();
        Self {
            text: None,
            language,
            segments,
            source,
            attributes: BTreeMap::new(),
        }
        .normalized()
    }

    pub fn validate(&self) -> crate::Result<()> {
        for segment in &self.segments {
            segment.validate()?;
        }
        Ok(())
    }

    pub fn validate_strict(&self) -> crate::Result<()> {
        self.validate()?;
        let mut last_start_seconds = None;
        for segment in &self.segments {
            if segment.text.trim().is_empty() {
                return Err(TranscriptionError::InvalidTranscript(
                    "transcript segment text must not be empty".to_string(),
                ));
            }
            if let (Some(previous), Some(current)) = (last_start_seconds, segment.start_seconds) {
                if current < previous {
                    return Err(TranscriptionError::InvalidTranscript(
                        "transcript segment start_seconds must not move backward".to_string(),
                    ));
                }
            }
            if segment.start_seconds.is_some() {
                last_start_seconds = segment.start_seconds;
            }
            for word in &segment.words {
                validate_word_inside_segment(segment, word)?;
            }
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

    pub fn text_or_joined(&self) -> String {
        self.text
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| self.joined_text())
    }

    pub fn normalized(mut self) -> crate::Result<Self> {
        self.text = self
            .text
            .map(|text| text.trim().to_string())
            .filter(|text| !text.is_empty());
        self.segments = self
            .segments
            .into_iter()
            .map(TranscriptSegmentContract::normalized)
            .collect();
        if self.text.is_none() {
            let joined = self.joined_text();
            if !joined.is_empty() {
                self.text = Some(joined);
            }
        }
        self.validate()?;
        Ok(self)
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

fn validate_word_inside_segment(
    segment: &TranscriptSegmentContract,
    word: &TranscriptWordContract,
) -> crate::Result<()> {
    if let (Some(segment_start), Some(word_start)) = (segment.start_seconds, word.start_seconds) {
        if word_start < segment_start {
            return Err(TranscriptionError::InvalidTranscript(
                "transcript word start_seconds must be within its segment".to_string(),
            ));
        }
    }
    if let (Some(segment_end), Some(word_end)) = (segment.end_seconds, word.end_seconds) {
        if word_end > segment_end {
            return Err(TranscriptionError::InvalidTranscript(
                "transcript word end_seconds must be within its segment".to_string(),
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
