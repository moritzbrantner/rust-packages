# jobs-core

Reusable long-running job state and generic artifact handling for Rust
applications.

## Highlights

- Stable job ids and metadata for cross-process APIs
- Status transitions for queued, running, cancelling, succeeded, failed, and cancelled jobs
- Cooperative cancellation token that can be shared with worker code
- Progress snapshots with optional totals and percent calculation
- Structured logs and artifact records
- Generic `OperationResult<T>`, `JobResult<T>`, and `JobManifest` envelopes
- Generic `ArtifactRef` records, memory/local artifact stores, SHA-256
  validation, and downloader/validator traits
- In-memory tracker and std-thread runner for small services, CLIs, and tests

## Example

```rust,no_run
use jobs_core::{BackgroundJobRunner, JobArtifact, JobProgress, JobSpec, Result};

fn main() -> Result<()> {
    let runner = BackgroundJobRunner::default();
    let spec = JobSpec::new("render-001", "Render preview")?.with_kind("render")?;

    let mut handle = runner.spawn(spec, |ctx| {
        ctx.info("started render")?;
        ctx.progress(JobProgress::new(1, Some(2))?.message("drawing"))?;
        ctx.check_cancelled()?;
        ctx.artifact(JobArtifact::new("preview", "Preview image").path("out/preview.png"))?;
        Ok(())
    })?;

    handle.join()?;
    let snapshot = runner.tracker().snapshot(handle.id())?.unwrap();
    assert!(snapshot.status.is_terminal());
    Ok(())
}
```

## Related crates

- `video-analysis-core`
- `video-analysis-storage`
