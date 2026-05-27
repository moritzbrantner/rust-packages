use video_analysis_core::runtime::{OperationId, SurfaceRequest};

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
}

#[test]
fn test_support_surface_returns_minimal_fixture_report() {
    let response = video_analysis_test_support::surface::run_surface_operation(SurfaceRequest {
        operation: OperationId::new("testSupport.minimalReport"),
        input: serde_json::json!({}),
    })
    .expect("minimal report");
    assert_eq!(response.value["producesComplexVideoOr3dPayloads"], false);
    assert!(response.value["dataset"]["records"].as_u64().unwrap() > 0);
}
