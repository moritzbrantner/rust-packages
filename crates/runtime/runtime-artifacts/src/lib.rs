#![doc = include_str!("../README.md")]

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use runtime_contracts::{ArtifactId, JobId};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const DEFAULT_WORKFLOW_OUTPUT_DIR: &str = ".workflow-output";

pub type Result<T> = std::result::Result<T, ArtifactError>;

#[derive(Debug)]
pub enum ArtifactError {
    Io(std::io::Error),
    NotFound(String),
    InvalidUri(String),
}

impl fmt::Display for ArtifactError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "{error}"),
            Self::NotFound(message) => write!(formatter, "artifact not found: {message}"),
            Self::InvalidUri(uri) => write!(formatter, "invalid artifact uri: {uri}"),
        }
    }
}

impl std::error::Error for ArtifactError {}

impl From<std::io::Error> for ArtifactError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactRef {
    pub id: String,
    pub kind: ArtifactKind,
    pub media_type: String,
    pub uri: String,
    pub size_bytes: Option<u64>,
    pub sha256: Option<String>,
    pub created_at: Option<String>,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ArtifactKind {
    File,
    Directory,
    Image,
    Audio,
    Video,
    Text,
    Json,
    Log,
    Model,
    Other,
}

#[derive(Debug, Clone)]
pub struct PutArtifactRequest {
    pub job_id: JobId,
    pub artifact_id: ArtifactId,
    pub kind: ArtifactKind,
    pub media_type: String,
    pub file_name: String,
    pub bytes: Vec<u8>,
    pub metadata: Value,
}

impl PutArtifactRequest {
    pub fn new(
        job_id: impl Into<JobId>,
        artifact_id: impl Into<ArtifactId>,
        file_name: impl Into<String>,
        bytes: impl Into<Vec<u8>>,
    ) -> Self {
        Self {
            job_id: job_id.into(),
            artifact_id: artifact_id.into(),
            kind: ArtifactKind::File,
            media_type: "application/octet-stream".to_string(),
            file_name: file_name.into(),
            bytes: bytes.into(),
            metadata: Value::Object(Default::default()),
        }
    }
}

pub trait ArtifactStore {
    fn put(&mut self, request: PutArtifactRequest) -> Result<ArtifactRef>;
    fn read(&self, artifact: &ArtifactRef) -> Result<Vec<u8>>;
    fn list(&self, job_id: &JobId) -> Result<Vec<ArtifactRef>>;
}

#[derive(Debug, Default, Clone)]
pub struct MemoryArtifactStore {
    artifacts: BTreeMap<String, Vec<StoredArtifact>>,
}

#[derive(Debug, Clone)]
struct StoredArtifact {
    artifact: ArtifactRef,
    bytes: Vec<u8>,
}

impl ArtifactStore for MemoryArtifactStore {
    fn put(&mut self, request: PutArtifactRequest) -> Result<ArtifactRef> {
        let artifact = artifact_ref(
            &request.artifact_id,
            request.kind,
            request.media_type,
            format!("memory://{}/{}", request.job_id.as_str(), request.file_name),
            request.bytes.len() as u64,
            request.metadata,
        );
        self.artifacts
            .entry(request.job_id.as_str().to_string())
            .or_default()
            .push(StoredArtifact {
                artifact: artifact.clone(),
                bytes: request.bytes,
            });
        Ok(artifact)
    }

    fn read(&self, artifact: &ArtifactRef) -> Result<Vec<u8>> {
        self.artifacts
            .values()
            .flatten()
            .find(|stored| stored.artifact.id == artifact.id && stored.artifact.uri == artifact.uri)
            .map(|stored| stored.bytes.clone())
            .ok_or_else(|| ArtifactError::NotFound(artifact.id.clone()))
    }

    fn list(&self, job_id: &JobId) -> Result<Vec<ArtifactRef>> {
        Ok(self
            .artifacts
            .get(job_id.as_str())
            .map(|artifacts| {
                artifacts
                    .iter()
                    .map(|stored| stored.artifact.clone())
                    .collect()
            })
            .unwrap_or_default())
    }
}

#[derive(Debug, Clone)]
pub struct LocalFileArtifactStore {
    root: PathBuf,
}

impl LocalFileArtifactStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn workflow_output(root: impl AsRef<Path>) -> Self {
        Self::new(root.as_ref().join(DEFAULT_WORKFLOW_OUTPUT_DIR))
    }

    pub fn job_dir(&self, job_id: &JobId) -> PathBuf {
        self.root.join("jobs").join(job_id.as_str())
    }

    pub fn artifacts_dir(&self, job_id: &JobId) -> PathBuf {
        self.job_dir(job_id).join("artifacts")
    }

    pub fn logs_dir(&self, job_id: &JobId) -> PathBuf {
        self.job_dir(job_id).join("logs")
    }
}

impl ArtifactStore for LocalFileArtifactStore {
    fn put(&mut self, request: PutArtifactRequest) -> Result<ArtifactRef> {
        let artifacts_dir = self.artifacts_dir(&request.job_id);
        fs::create_dir_all(&artifacts_dir)?;
        fs::create_dir_all(self.logs_dir(&request.job_id))?;
        let path = artifacts_dir.join(safe_file_name(&request.file_name));
        fs::write(&path, &request.bytes)?;
        let artifact = artifact_ref(
            &request.artifact_id,
            request.kind,
            request.media_type,
            file_uri(&path),
            request.bytes.len() as u64,
            request.metadata,
        );
        self.write_manifest(&request.job_id, std::slice::from_ref(&artifact))?;
        Ok(artifact)
    }

    fn read(&self, artifact: &ArtifactRef) -> Result<Vec<u8>> {
        let path = path_from_file_uri(&artifact.uri)?;
        fs::read(path).map_err(ArtifactError::Io)
    }

    fn list(&self, job_id: &JobId) -> Result<Vec<ArtifactRef>> {
        let manifest = self.job_dir(job_id).join("manifest.json");
        if !manifest.exists() {
            return Ok(Vec::new());
        }
        let bytes = fs::read(manifest)?;
        serde_json::from_slice(&bytes).map_err(|error| {
            ArtifactError::InvalidUri(format!("manifest for {}: {error}", job_id.as_str()))
        })
    }
}

impl LocalFileArtifactStore {
    fn write_manifest(&self, job_id: &JobId, new_artifacts: &[ArtifactRef]) -> Result<()> {
        let mut artifacts = self.list(job_id)?;
        artifacts.extend_from_slice(new_artifacts);
        let manifest = self.job_dir(job_id).join("manifest.json");
        let bytes = serde_json::to_vec_pretty(&artifacts)
            .map_err(|error| ArtifactError::InvalidUri(error.to_string()))?;
        fs::write(manifest, bytes)?;
        Ok(())
    }
}

fn artifact_ref(
    id: &ArtifactId,
    kind: ArtifactKind,
    media_type: String,
    uri: String,
    size_bytes: u64,
    metadata: Value,
) -> ArtifactRef {
    ArtifactRef {
        id: id.as_str().to_string(),
        kind,
        media_type,
        uri,
        size_bytes: Some(size_bytes),
        sha256: None,
        created_at: Some(now_string()),
        metadata,
    }
}

fn safe_file_name(file_name: &str) -> String {
    file_name
        .chars()
        .map(|character| match character {
            '/' | '\\' => '_',
            _ => character,
        })
        .collect()
}

fn file_uri(path: &Path) -> String {
    format!("file://{}", path.to_string_lossy())
}

fn path_from_file_uri(uri: &str) -> Result<PathBuf> {
    uri.strip_prefix("file://")
        .map(PathBuf::from)
        .ok_or_else(|| ArtifactError::InvalidUri(uri.to_string()))
}

fn now_string() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    format!("unix:{seconds}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_store_round_trips_artifact_bytes() {
        let mut store = MemoryArtifactStore::default();
        let request = PutArtifactRequest::new("job-1", "artifact-1", "result.txt", b"hello");

        let artifact = store.put(request).expect("put artifact");
        let bytes = store.read(&artifact).expect("read artifact");
        let artifacts = store.list(&JobId::new("job-1")).expect("list artifacts");

        assert_eq!(bytes, b"hello");
        assert_eq!(artifacts, vec![artifact]);
    }

    #[test]
    fn local_file_store_writes_manifest_and_reads_bytes() {
        let root = std::env::temp_dir().join(format!(
            "runtime-artifacts-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut store = LocalFileArtifactStore::new(&root);
        let request = PutArtifactRequest::new("job-1", "artifact-1", "result.txt", b"hello");

        let artifact = store.put(request).expect("put artifact");
        let bytes = store.read(&artifact).expect("read artifact");
        let artifacts = store.list(&JobId::new("job-1")).expect("list artifacts");

        assert_eq!(bytes, b"hello");
        assert_eq!(artifacts, vec![artifact]);

        let _ = fs::remove_dir_all(root);
    }
}
