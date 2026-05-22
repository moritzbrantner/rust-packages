use std::sync::{Arc, Mutex};

use jobs_core::{BackgroundJobRunner, JobArtifact, JobError, JobProgress, JobSpec};

use crate::{ModelBundle, ModelBundleStore, ModelRuntimeBackend, ModelSpec};

/// Long-running model operation kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelJobKind {
    /// Download remote model files.
    Download,
    /// Materialize a local model bundle.
    MaterializeBundle,
    /// Validate an existing model bundle.
    ValidateBundle,
    /// Warm a runtime/session.
    Warmup,
    /// Run one inference request.
    Inference,
    /// Run batch inference.
    BatchInference,
    /// Run an external model command.
    ExternalCommand,
}

impl ModelJobKind {
    /// Returns the stable job kind string.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Download => "model-download",
            Self::MaterializeBundle => "model-materialize-bundle",
            Self::ValidateBundle => "model-validate-bundle",
            Self::Warmup => "model-warmup",
            Self::Inference => "model-inference",
            Self::BatchInference => "model-batch-inference",
            Self::ExternalCommand => "model-external-command",
        }
    }
}

/// Builds a standard model job spec with model/runtime metadata.
pub fn model_job_spec(
    id: impl Into<String>,
    kind: ModelJobKind,
    spec: &ModelSpec,
    backend: ModelRuntimeBackend,
) -> jobs_core::Result<JobSpec> {
    let mut job = JobSpec::new(id, format!("{} {}", kind.as_str(), spec.name))?
        .with_kind(kind.as_str())?
        .with_metadata("model.name", spec.name.clone())?
        .with_metadata("model.task", spec.task.as_protocol_str().to_string())?
        .with_metadata("model.source", spec.source.kind().to_string())?
        .with_metadata("model.runtime", backend.as_str().to_string())?;
    if let Some(revision) = spec.revision_value() {
        job = job.with_metadata("model.revision", revision.to_string())?;
    }
    Ok(job)
}

/// Join handle for model jobs that produce a typed value.
#[derive(Debug)]
pub struct ModelJobJoinHandle<T> {
    inner: jobs_core::JobJoinHandle,
    value: Arc<Mutex<Option<T>>>,
}

impl<T: Clone> ModelJobJoinHandle<T> {
    /// Returns the underlying tracked job.
    pub fn job(&self) -> &jobs_core::JobHandle {
        self.inner.job()
    }

    /// Returns the job id.
    pub fn id(&self) -> &jobs_core::JobId {
        self.inner.id()
    }

    /// Requests cancellation.
    pub fn request_cancel(&self) -> jobs_core::Result<()> {
        self.inner.request_cancel()
    }

    /// Waits for completion and returns the produced value.
    pub fn join_result(&mut self) -> jobs_core::Result<T> {
        self.inner.join()?;
        self.value
            .lock()
            .map_err(|_| JobError::StateUnavailable("model job result lock poisoned".to_string()))?
            .clone()
            .ok_or_else(|| JobError::Failed("model job did not produce a result".to_string()))
    }
}

/// Spawns a model download/materialization job.
pub fn spawn_model_download_job(
    runner: &BackgroundJobRunner,
    spec: ModelSpec,
    store: ModelBundleStore,
) -> jobs_core::Result<ModelJobJoinHandle<ModelBundle>> {
    let value = Arc::new(Mutex::new(None));
    let value_for_job = Arc::clone(&value);
    let job_spec = model_job_spec(
        format!(
            "model-download-{}-{}",
            spec.safe_name(),
            spec.revision_value().unwrap_or("local")
        ),
        ModelJobKind::Download,
        &spec,
        ModelRuntimeBackend::External,
    )?;
    let inner = runner.spawn(job_spec, move |context| {
        context.info(format!("materializing model bundle `{}`", spec.name))?;
        context.progress(
            JobProgress::new(0, Some(2))?
                .unit("steps")?
                .message("starting model download"),
        )?;
        context.check_cancelled()?;
        let bundle = store
            .download(&spec)
            .map_err(|err| JobError::Failed(err.to_string()))?;
        context.progress(
            JobProgress::new(1, Some(2))?
                .unit("steps")?
                .message("model files materialized"),
        )?;
        context.artifact(
            JobArtifact::new("manifest", "Model bundle manifest")
                .kind("model-bundle")
                .path(bundle.manifest_path()),
        )?;
        *value_for_job.lock().map_err(|_| {
            JobError::StateUnavailable("model job result lock poisoned".to_string())
        })? = Some(bundle);
        context.progress(
            JobProgress::new(2, Some(2))?
                .unit("steps")?
                .message("model bundle ready"),
        )?;
        Ok(())
    })?;
    Ok(ModelJobJoinHandle { inner, value })
}
