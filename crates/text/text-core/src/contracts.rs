use std::collections::BTreeMap;

use runtime_contracts::{MobileCapability, OperationId, OperationMetadata, RuntimeCapabilities};
use serde::{Deserialize, Serialize};
use video_analysis_core::{OwnedTextSegment, TextSegment, Timebase, Timestamp};

use crate::{segment_document_id, OwnedTextDocument, TextDocument};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TimebaseContract {
    pub num: i32,
    pub den: i32,
}

impl From<Timebase> for TimebaseContract {
    fn from(value: Timebase) -> Self {
        Self {
            num: value.num,
            den: value.den,
        }
    }
}

impl From<TimebaseContract> for Timebase {
    fn from(value: TimebaseContract) -> Self {
        Self::new(value.num, value.den)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TimestampContract {
    pub pts: i64,
    pub timebase: TimebaseContract,
}

impl TimestampContract {
    pub fn seconds(self) -> f64 {
        Timestamp::from(self).seconds()
    }
}

impl From<Timestamp> for TimestampContract {
    fn from(value: Timestamp) -> Self {
        Self {
            pts: value.pts,
            timebase: value.timebase.into(),
        }
    }
}

impl From<TimestampContract> for Timestamp {
    fn from(value: TimestampContract) -> Self {
        Self::new(value.pts, value.timebase.into())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TextDocumentContract {
    pub id: String,
    pub text: String,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub timestamp: Option<TimestampContract>,
    #[serde(default)]
    pub attributes: BTreeMap<String, String>,
}

impl TextDocumentContract {
    pub fn new(id: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            language: None,
            timestamp: None,
            attributes: BTreeMap::new(),
        }
    }
}

pub trait IntoTextDocumentContract {
    fn into_text_document_contract(self) -> TextDocumentContract;
}

impl IntoTextDocumentContract for TextDocument<'_> {
    fn into_text_document_contract(self) -> TextDocumentContract {
        TextDocumentContract {
            id: self.id.to_string(),
            text: self.text.to_string(),
            language: self.language.map(ToString::to_string),
            timestamp: self.timestamp.map(Into::into),
            attributes: BTreeMap::new(),
        }
    }
}

impl IntoTextDocumentContract for OwnedTextDocument {
    fn into_text_document_contract(self) -> TextDocumentContract {
        TextDocumentContract {
            id: self.id,
            text: self.text,
            language: self.language,
            timestamp: self.timestamp.map(Into::into),
            attributes: BTreeMap::new(),
        }
    }
}

impl IntoTextDocumentContract for &OwnedTextDocument {
    fn into_text_document_contract(self) -> TextDocumentContract {
        self.as_document().into_text_document_contract()
    }
}

impl From<TextDocument<'_>> for TextDocumentContract {
    fn from(value: TextDocument<'_>) -> Self {
        value.into_text_document_contract()
    }
}

impl From<OwnedTextDocument> for TextDocumentContract {
    fn from(value: OwnedTextDocument) -> Self {
        value.into_text_document_contract()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TextSegmentContract {
    #[serde(default)]
    pub stream_id: Option<String>,
    pub segment_index: u64,
    pub text: String,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub timestamp: Option<TimestampContract>,
    #[serde(default)]
    pub duration_seconds: Option<f64>,
    pub is_final: bool,
    #[serde(default)]
    pub attributes: BTreeMap<String, String>,
}

impl TextSegmentContract {
    pub fn new(segment_index: u64, text: impl Into<String>) -> Self {
        Self {
            stream_id: None,
            segment_index,
            text: text.into(),
            language: None,
            timestamp: None,
            duration_seconds: None,
            is_final: true,
            attributes: BTreeMap::new(),
        }
    }

    pub fn document_id(&self) -> Option<String> {
        self.stream_id
            .as_deref()
            .map(|stream_id| segment_document_id(stream_id, self.segment_index))
    }

    pub fn to_owned_text_segment(&self) -> OwnedTextSegment {
        let mut segment =
            OwnedTextSegment::new(self.segment_index, self.text.clone()).finality(self.is_final);
        if let Some(language) = &self.language {
            segment = segment.language(language.clone());
        }
        if let Some(timestamp) = self.timestamp {
            segment = segment.timestamp(timestamp.into());
        }
        segment
    }
}

pub trait AsTextSegmentContract {
    fn as_text_segment_contract(&self) -> TextSegmentContract;
}

impl AsTextSegmentContract for TextSegment<'_> {
    fn as_text_segment_contract(&self) -> TextSegmentContract {
        TextSegmentContract {
            stream_id: None,
            segment_index: self.segment_index,
            text: self.text.to_string(),
            language: self.language.map(ToString::to_string),
            timestamp: self.timestamp.map(Into::into),
            duration_seconds: None,
            is_final: self.is_final,
            attributes: BTreeMap::new(),
        }
    }
}

impl AsTextSegmentContract for OwnedTextSegment {
    fn as_text_segment_contract(&self) -> TextSegmentContract {
        self.as_segment().as_text_segment_contract()
    }
}

impl From<TextSegment<'_>> for TextSegmentContract {
    fn from(value: TextSegment<'_>) -> Self {
        value.as_text_segment_contract()
    }
}

impl From<OwnedTextSegment> for TextSegmentContract {
    fn from(value: OwnedTextSegment) -> Self {
        value.as_text_segment_contract()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TextStatisticsRequest {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TextStatisticsResult {
    pub byte_count: usize,
    pub character_count: usize,
    pub word_count: usize,
    pub line_count: usize,
    pub sentence_count: usize,
}

pub fn text_statistics_metadata() -> OperationMetadata {
    OperationMetadata {
        id: OperationId::new("text.statistics"),
        name: "Text statistics".to_string(),
        description: Some("Counts bytes, characters, words, lines, and sentences.".to_string()),
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: RuntimeCapabilities {
            native: true,
            server: true,
            wasm: true,
            mobile: MobileCapability::Wasm,
            requirements: Vec::new(),
            max_recommended_input_bytes: Some(1_000_000),
        },
    }
}
