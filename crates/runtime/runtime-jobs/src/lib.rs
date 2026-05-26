#![doc = include_str!("../README.md")]

pub mod surface;
use runtime_artifacts::ArtifactRef;
use video_analysis_core::runtime::{Diagnostic, JobId, OperationId};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum JobStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

impl JobStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct JobProgress {
    pub completed: u64,
    pub total: Option<u64>,
    pub unit: String,
    pub message: Option<String>,
}

impl JobProgress {
    pub fn new(completed: u64, total: Option<u64>, unit: impl Into<String>) -> Self {
        Self {
            completed,
            total,
            unit: unit.into(),
            message: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct JobEvent {
    pub job_id: JobId,
    pub status: JobStatus,
    pub progress: Option<JobProgress>,
    pub diagnostics: Vec<Diagnostic>,
    pub message: Option<String>,
    pub occurred_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct JobManifest {
    pub job_id: JobId,
    pub operation_id: OperationId,
    pub status: JobStatus,
    pub progress: Option<JobProgress>,
    pub diagnostics: Vec<Diagnostic>,
    pub artifacts: Vec<ArtifactRef>,
    pub metadata: Value,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OperationResult<T> {
    pub value: Option<T>,
    pub diagnostics: Vec<Diagnostic>,
    pub artifacts: Vec<ArtifactRef>,
}

impl<T> OperationResult<T> {
    pub fn value(value: T) -> Self {
        Self {
            value: Some(value),
            diagnostics: Vec::new(),
            artifacts: Vec::new(),
        }
    }

    pub fn empty() -> Self {
        Self {
            value: None,
            diagnostics: Vec::new(),
            artifacts: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct JobResult<T> {
    pub job_id: JobId,
    pub status: JobStatus,
    pub result: OperationResult<T>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operation_result_serializes_contract_fields() {
        let result = OperationResult::value(42_u32);
        let json = serde_json::to_string(&result).expect("serialize result");

        assert!(json.contains("\"value\":42"));
        assert!(json.contains("\"diagnostics\":[]"));
        assert!(json.contains("\"artifacts\":[]"));
    }

    #[test]
    fn terminal_statuses_are_detected() {
        assert!(JobStatus::Succeeded.is_terminal());
        assert!(!JobStatus::Running.is_terminal());
    }
}
