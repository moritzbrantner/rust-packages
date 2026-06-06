use runtime_core::{OperationId, SurfaceRequest};

#[test]
fn data_inversion_jobs_and_model_runtime_surfaces_accept_valid_inputs() {
    let trace = data_inversion_core::surface::run_surface_operation(SurfaceRequest {
        operation: OperationId::new("inversion.trace"),
        input: serde_json::json!({"sourceType": "histogram", "targetType": "image", "fidelity": "heuristic"}),
    })
    .expect("inversion trace");
    assert_eq!(trace.value["fidelity"], "heuristic");

    let lifecycle = jobs_core::surface::run_surface_operation(SurfaceRequest {
        operation: OperationId::new("jobs.lifecycle"),
        input: serde_json::json!({"spec": {"id": "job-1", "name": "Surface job"}}),
    })
    .expect("jobs lifecycle");
    assert_eq!(lifecycle.value["status"], "succeeded");

    let bundle = model_runtime::surface::run_surface_operation(SurfaceRequest {
        operation: OperationId::new("model.bundlePlan"),
        input: serde_json::json!({
            "spec": {
                "name": "demo",
                "task": "text_embedding",
                "source": {"kind": "hugging_face", "repo_id": "demo/model", "revision": "main"},
                "files": [{"required": "config.json"}]
            },
            "localFiles": ["config.json"]
        }),
    })
    .expect("bundle plan");
    assert_eq!(bundle.value["downloadsRequired"], false);
    assert_eq!(bundle.value["manifest"]["files"][0]["presentLocally"], true);

    let execution_plan = model_runtime::surface::run_surface_operation(SurfaceRequest {
        operation: OperationId::new("model.executionPlan"),
        input: serde_json::json!({
            "id": "model-job-1",
            "kind": "Inference",
            "spec": {
                "name": "demo",
                "task": "text_embedding",
                "source": {"kind": "hugging_face", "repo_id": "demo/model", "revision": "main"},
                "files": [{"required": "config.json"}]
            },
            "backend": "heuristic",
            "inputs": [{"kind": "json", "value": {"text": "hello"}}]
        }),
    })
    .expect("model execution plan");
    assert_eq!(execution_plan.value["executionPlan"]["mode"], "plannedJob");

    let timecode = video_analysis_core::surface::run_surface_operation(SurfaceRequest {
        operation: OperationId::new("video.core.timecode"),
        input: serde_json::json!({
            "fps": {"numerator": 24, "denominator": 1},
            "frames": [48],
            "seconds": [1.5],
            "timecodes": ["00:00:02.000"]
        }),
    })
    .expect("video timecode");
    assert_eq!(timecode.value["frames"][0]["display"], "00:00:02.000");
}
