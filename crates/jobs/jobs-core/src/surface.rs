//! Library-owned runtime surface for `jobs-core`.

use serde::Deserialize;
use video_analysis_core::runtime::{
    OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation, SurfaceRequest,
    SurfaceResponse,
};

use std::collections::BTreeMap;

use crate::{JobArtifact, JobLogLevel, JobProgress, JobSpec, JobStatus, JobTracker};

/// Returns the package surface exposed by every transport wrapper.
pub fn package_surface() -> PackageSurface {
    PackageSurface {
        library: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: RuntimeCapabilities::pure_rust(),
        operations: vec![
            operation(
                "describe",
                "Describe package",
                "Reusable long-running job state, cancellation, progress, logs, and artifact primitives.",
                serde_json::json!({"includeOperations": true}),
            ),
            operation(
                "jobs.spec",
                "Validate job spec",
                "Validates a JobSpec input and returns normalized id, name, kind, and metadata.",
                serde_json::json!({"id": "job-1", "name": "Demo job", "kind": "demo", "metadata": {"owner": "surface"}}),
            ),
            operation(
                "jobs.progress",
                "Validate job progress",
                "Validates JobProgress input and returns fraction and percent when a total is known.",
                serde_json::json!({"completed": 2, "total": 4, "unit": "steps", "message": "halfway"}),
            ),
            operation(
                "jobs.lifecycle",
                "Script job lifecycle",
                "Creates an in-memory tracker, applies a short lifecycle script, and returns the final snapshot and events.",
                serde_json::json!({"spec": {"id": "job-1", "name": "Demo job"}, "script": ["running", "progress", "log", "artifact", "succeeded"]}),
            ),
        ],
    }
}

fn operation(
    id: &str,
    name: &str,
    description: &str,
    example_request: serde_json::Value,
) -> SurfaceOperation {
    SurfaceOperation {
        id: OperationId::new(id),
        name: name.to_string(),
        description: Some(description.to_string()),
        input_schema: serde_json::json!({"type": "object", "additionalProperties": true}),
        output_schema: serde_json::json!({"type": "object"}),
        example_request,
        wasm_supported: true,
        server_supported: true,
    }
}

/// Runs one library-owned operation.
pub fn run_surface_operation(request: SurfaceRequest) -> Result<SurfaceResponse, String> {
    let operation = request.operation.clone();
    let value = match request.operation.as_str() {
        "describe" => describe_value(request.input),
        "jobs.spec" => spec_value(parse_input(request.input)?)?,
        "jobs.progress" => progress_value(parse_input(request.input)?)?,
        "jobs.lifecycle" => lifecycle_value(parse_input(request.input)?)?,
        operation => {
            return Err(format!(
                "unsupported operation `{operation}` for {}",
                env!("CARGO_PKG_NAME")
            ));
        }
    };
    Ok(response(operation, value))
}

fn describe_value(input: serde_json::Value) -> serde_json::Value {
    let surface = package_surface();
    serde_json::json!({
        "library": surface.library,
        "version": surface.version,
        "operationCount": surface.operations.len(),
        "operations": surface.operations.iter().map(|operation| operation.id.as_str()).collect::<Vec<_>>(),
        "input": input
    })
}

fn response(operation: OperationId, value: serde_json::Value) -> SurfaceResponse {
    SurfaceResponse {
        operation,
        value,
        diagnostics: Vec::new(),
        artifacts: Vec::new(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LifecycleRequest {
    spec: JobSpecInput,
    #[serde(default = "default_lifecycle_script")]
    script: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JobSpecInput {
    id: String,
    name: String,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    metadata: BTreeMap<String, String>,
}

fn spec_value(spec: JobSpecInput) -> Result<serde_json::Value, String> {
    let normalized = build_spec(spec)?;
    Ok(serde_json::json!({
        "id": normalized.id.as_str(),
        "name": normalized.name,
        "kind": normalized.kind,
        "metadata": normalized.metadata
    }))
}

fn progress_value(progress: JobProgress) -> Result<serde_json::Value, String> {
    progress.validate().map_err(|error| error.to_string())?;
    Ok(serde_json::json!({
        "completed": progress.completed,
        "total": progress.total,
        "unit": progress.unit,
        "message": progress.message,
        "fraction": progress.fraction(),
        "percent": progress.percent()
    }))
}

fn lifecycle_value(request: LifecycleRequest) -> Result<serde_json::Value, String> {
    let tracker = JobTracker::new();
    let handle = tracker
        .create(build_spec(request.spec)?)
        .map_err(|error| error.to_string())?;
    let context = handle.context();
    for step in request.script {
        match step.as_str() {
            "create" | "queued" => {}
            "running" => handle.mark_running().map_err(|error| error.to_string())?,
            "progress" => context
                .progress(JobProgress::new(1, Some(2)).map_err(|error| error.to_string())?)
                .map_err(|error| error.to_string())?,
            "log" => context
                .log(JobLogLevel::Info, "surface lifecycle log")
                .map_err(|error| error.to_string())?,
            "artifact" => context
                .artifact(
                    JobArtifact::new("artifact-1", "surface artifact")
                        .kind("json")
                        .uri("memory://artifact-1")
                        .content_type("application/json")
                        .bytes(2),
                )
                .map_err(|error| error.to_string())?,
            "succeeded" => handle.mark_succeeded().map_err(|error| error.to_string())?,
            "failed" => handle
                .mark_failed("surface lifecycle failure")
                .map_err(|error| error.to_string())?,
            "cancelled" => handle
                .mark_cancelled(Some("surface lifecycle cancelled".to_string()))
                .map_err(|error| error.to_string())?,
            other => return Err(format!("unsupported lifecycle step `{other}`")),
        }
    }
    let snapshot = tracker
        .snapshot(handle.id())
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "lifecycle job snapshot missing".to_string())?;
    let events = tracker
        .events(handle.id())
        .map_err(|error| error.to_string())?;
    Ok(serde_json::json!({
        "snapshot": snapshot,
        "events": events,
        "terminal": snapshot.status.is_terminal(),
        "status": status_name(snapshot.status)
    }))
}

fn build_spec(spec: JobSpecInput) -> Result<JobSpec, String> {
    let normalized = JobSpec::new(spec.id, spec.name).map_err(|error| error.to_string())?;
    let normalized = if let Some(kind) = spec.kind {
        normalized
            .with_kind(kind)
            .map_err(|error| error.to_string())?
    } else {
        normalized
    };
    spec.metadata
        .into_iter()
        .try_fold(normalized, |spec, (key, value)| {
            spec.with_metadata(key, value)
                .map_err(|error| error.to_string())
        })
}

fn status_name(status: JobStatus) -> &'static str {
    match status {
        JobStatus::Queued => "queued",
        JobStatus::Running => "running",
        JobStatus::Cancelling => "cancelling",
        JobStatus::Succeeded => "succeeded",
        JobStatus::Failed => "failed",
        JobStatus::Cancelled => "cancelled",
    }
}

fn parse_input<T: for<'de> Deserialize<'de>>(input: serde_json::Value) -> Result<T, String> {
    serde_json::from_value(input).map_err(|error| format!("invalid request: {error}"))
}

fn default_lifecycle_script() -> Vec<String> {
    vec![
        "running".to_string(),
        "progress".to_string(),
        "log".to_string(),
        "artifact".to_string(),
        "succeeded".to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_surface_lists_job_operations() {
        let ids = package_surface()
            .operations
            .into_iter()
            .map(|operation| operation.id.0)
            .collect::<Vec<_>>();
        assert!(ids.contains(&"jobs.spec".to_string()));
        assert!(ids.contains(&"jobs.progress".to_string()));
        assert!(ids.contains(&"jobs.lifecycle".to_string()));
    }

    #[test]
    fn progress_operation_returns_percent() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("jobs.progress"),
            input: serde_json::json!({"completed": 2, "total": 4, "unit": "steps"}),
        })
        .expect("progress");
        assert_eq!(response.value["percent"], 50.0);
    }

    #[test]
    fn lifecycle_operation_returns_succeeded_snapshot() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("jobs.lifecycle"),
            input: serde_json::json!({"spec": {"id": "job-1", "name": "Demo"}}),
        })
        .expect("lifecycle");
        assert_eq!(response.value["status"], "succeeded");
        assert!(response.value["events"].as_array().unwrap().len() >= 5);
    }

    #[test]
    fn lifecycle_rejects_unknown_step() {
        let error = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("jobs.lifecycle"),
            input: serde_json::json!({"spec": {"id": "job-1", "name": "Demo"}, "script": ["wat"]}),
        })
        .expect_err("bad step");
        assert!(error.contains("unsupported lifecycle step"));
    }
}
