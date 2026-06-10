use runtime_core::{PackageSurface, SurfaceRequest, SurfaceResponse};

type SurfaceFn = fn() -> PackageSurface;
type RunFn = fn(SurfaceRequest) -> Result<SurfaceResponse, String>;

struct FoundationSurfaceCase {
    crate_name: &'static str,
    package_surface: SurfaceFn,
    run: RunFn,
}

#[test]
fn foundation_surfaces_expose_release_contracts_and_run_examples() {
    for case in foundation_surface_cases() {
        let surface = (case.package_surface)();

        for operation in &surface.operations {
            assert_release_schema(case.crate_name, operation);

            let response = (case.run)(SurfaceRequest {
                operation: operation.id.clone(),
                input: operation.example_request.clone(),
            })
            .unwrap_or_else(|error| {
                panic!(
                    "{} {} failed: {error}",
                    case.crate_name,
                    operation.id.as_str()
                )
            });

            assert_structured_response(case.crate_name, operation.id.as_str(), &response);
            assert!(
                response.artifacts.is_empty(),
                "{} {} emitted artifacts from a default package-surface example",
                case.crate_name,
                operation.id.as_str()
            );
        }
    }
}

#[test]
fn foundation_surfaces_return_typed_unknown_operation_errors() {
    for case in foundation_surface_cases() {
        let error = (case.run)(SurfaceRequest {
            operation: "missing.operation".into(),
            input: serde_json::json!({}),
        })
        .expect_err(case.crate_name);
        let parsed = runtime_core::parse_surface_error(&error)
            .unwrap_or_else(|| panic!("{} returned untyped error: {error}", case.crate_name));

        assert_eq!(parsed.code, "unsupported_operation");
        assert_eq!(parsed.operation.unwrap().as_str(), "missing.operation");
    }
}

#[test]
fn foundation_surface_resource_limits_are_typed() {
    let too_many_values = vec![0.0_f64; 100_001];
    let error = numbers_core::surface::run_surface_operation(SurfaceRequest {
        operation: "numbers.summary".into(),
        input: serde_json::json!({"values": too_many_values}),
    })
    .expect_err("numbers limit");
    assert_typed_error(&error, "resource_limit", "values");

    let error = tensor_data::surface::run_surface_operation(SurfaceRequest {
        operation: "tensor.summary".into(),
        input: serde_json::json!({"shape": [2], "values": [1.0, 2.0], "previewValues": 257}),
    })
    .expect_err("tensor preview limit");
    assert_typed_error(&error, "resource_limit", "previewValues");

    let too_many_vector_values = vec![1.0_f32; 100_001];
    let error = vector_analysis_core::surface::run_surface_operation(SurfaceRequest {
        operation: "vector.normalize".into(),
        input: serde_json::json!({"values": too_many_vector_values}),
    })
    .expect_err("vector limit");
    assert_typed_error(&error, "resource_limit", "values");

    let too_many_entries = (0..100_001)
        .map(|_| (0_usize, 0_usize, 1.0_f32))
        .collect::<Vec<_>>();
    let error = math_sparse_data::surface::run_surface_operation(SurfaceRequest {
        operation: "sparse.matrixSummary".into(),
        input: serde_json::json!({"format": "coo", "rows": 1, "cols": 1, "entries": too_many_entries}),
    })
    .expect_err("sparse entry limit");
    assert_typed_error(&error, "resource_limit", "entries");
}

#[test]
fn foundation_surface_unsupported_values_are_typed() {
    let error = jobs_core::surface::run_surface_operation(SurfaceRequest {
        operation: "jobs.lifecycle".into(),
        input: serde_json::json!({"spec": {"id": "job-1", "name": "Demo"}, "script": ["running", "teleport"]}),
    })
    .expect_err("unsupported lifecycle step");
    assert_typed_error(&error, "unsupported_value", "script");

    let error = vector_analysis_core::surface::run_surface_operation(SurfaceRequest {
        operation: "vector.distance".into(),
        input: serde_json::json!({"left": [1.0], "right": [1.0], "metric": "chebyshev"}),
    })
    .expect_err("unsupported metric");
    assert_typed_error(&error, "unsupported_value", "metric");

    let error = math_sparse_data::surface::run_surface_operation(SurfaceRequest {
        operation: "sparse.matrixSummary".into(),
        input: serde_json::json!({"format": "dense", "rows": 1, "cols": 1}),
    })
    .expect_err("unsupported sparse format");
    assert_typed_error(&error, "unsupported_value", "format");

    let error = video_analysis_core::surface::run_surface_operation(SurfaceRequest {
        operation: "video.core.frameSummary".into(),
        input: serde_json::json!({
            "frames": [{
                "frameIndex": 0,
                "timestampSeconds": 0.0,
                "width": 2,
                "height": 2,
                "pixelFormat": "yuv420p",
                "stride": 2,
                "bytes": 8
            }]
        }),
    })
    .expect_err("unsupported pixel format");
    assert_typed_error(&error, "unsupported_value", "pixelFormat");
}

#[test]
fn runtime_job_surfaces_expose_execution_plan_metadata() {
    for (crate_name, surface) in [
        ("jobs-core", jobs_core::surface::package_surface()),
        ("model-runtime", model_runtime::surface::package_surface()),
        (
            "video-analysis-core",
            video_analysis_core::surface::package_surface(),
        ),
    ] {
        let planned = surface
            .operations
            .iter()
            .filter(|operation| operation.id.as_str() != "describe")
            .filter(|operation| !operation.input_schema["xExecutionPlan"].is_null())
            .count();
        assert!(
            planned > 0,
            "{crate_name} must expose xExecutionPlan metadata for runtime/job operations"
        );
    }
}

fn assert_release_schema(crate_name: &str, operation: &runtime_core::SurfaceOperation) {
    let id = operation.id.as_str();
    assert_eq!(
        operation.input_schema["type"], "object",
        "{crate_name} {id} input schema must be an object"
    );
    assert_eq!(
        operation.input_schema["additionalProperties"], false,
        "{crate_name} {id} must reject undeclared top-level input fields"
    );
    assert_eq!(
        operation.input_schema["xReleaseStability"], "stable",
        "{crate_name} {id} missing stable release marker"
    );
    assert_eq!(
        operation.input_schema["xContractPolicy"], "additiveOnly",
        "{crate_name} {id} missing additive-only contract policy"
    );
    assert!(
        operation.input_schema["xOperationCategory"].is_string(),
        "{crate_name} {id} missing operation category"
    );
    assert!(
        operation.input_schema["xErrorShape"].is_object(),
        "{crate_name} {id} missing typed error shape metadata"
    );
    assert_eq!(
        operation.output_schema["required"],
        serde_json::json!(["operation", "title", "message", "summary", "result"]),
        "{crate_name} {id} output schema must preserve the structured response shape"
    );
}

fn assert_structured_response(crate_name: &str, operation: &str, response: &SurfaceResponse) {
    assert_eq!(
        response.operation.as_str(),
        operation,
        "{crate_name} response operation mismatch"
    );
    for field in ["operation", "title", "message", "summary", "result"] {
        assert!(
            !response.value[field].is_null(),
            "{crate_name} {operation} missing structured field `{field}`"
        );
    }
    assert_eq!(response.value["operation"], operation);
}

fn assert_typed_error(error: &str, code: &str, field: &str) {
    let parsed = runtime_core::parse_surface_error(error)
        .unwrap_or_else(|| panic!("expected typed surface error: {error}"));
    assert_eq!(parsed.code, code);
    assert_eq!(parsed.details["field"], field);
}

fn foundation_surface_cases() -> Vec<FoundationSurfaceCase> {
    vec![
        FoundationSurfaceCase {
            crate_name: "jobs-core",
            package_surface: jobs_core::surface::package_surface,
            run: jobs_core::surface::run_surface_operation,
        },
        FoundationSurfaceCase {
            crate_name: "model-runtime",
            package_surface: model_runtime::surface::package_surface,
            run: model_runtime::surface::run_surface_operation,
        },
        FoundationSurfaceCase {
            crate_name: "video-analysis-core",
            package_surface: video_analysis_core::surface::package_surface,
            run: video_analysis_core::surface::run_surface_operation,
        },
        FoundationSurfaceCase {
            crate_name: "image-analysis-core",
            package_surface: image_analysis_core::surface::package_surface,
            run: image_analysis_core::surface::run_surface_operation,
        },
        FoundationSurfaceCase {
            crate_name: "audio-analysis-core",
            package_surface: audio_analysis_core::surface::package_surface,
            run: audio_analysis_core::surface::run_surface_operation,
        },
        FoundationSurfaceCase {
            crate_name: "text-core",
            package_surface: text_core::surface::package_surface,
            run: text_core::surface::run_surface_operation,
        },
        FoundationSurfaceCase {
            crate_name: "text-transcripts",
            package_surface: text_transcripts::surface::package_surface,
            run: text_transcripts::surface::run_surface_operation,
        },
        FoundationSurfaceCase {
            crate_name: "numbers-core",
            package_surface: numbers_core::surface::package_surface,
            run: numbers_core::surface::run_surface_operation,
        },
        FoundationSurfaceCase {
            crate_name: "tensor-data",
            package_surface: tensor_data::surface::package_surface,
            run: tensor_data::surface::run_surface_operation,
        },
        FoundationSurfaceCase {
            crate_name: "vector-analysis-core",
            package_surface: vector_analysis_core::surface::package_surface,
            run: vector_analysis_core::surface::run_surface_operation,
        },
        FoundationSurfaceCase {
            crate_name: "math-sparse-data",
            package_surface: math_sparse_data::surface::package_surface,
            run: math_sparse_data::surface::run_surface_operation,
        },
        FoundationSurfaceCase {
            crate_name: "dense-data",
            package_surface: dense_data::surface::package_surface,
            run: dense_data::surface::run_surface_operation,
        },
        FoundationSurfaceCase {
            crate_name: "math-geometry-2d",
            package_surface: math_geometry_2d::surface::package_surface,
            run: math_geometry_2d::surface::run_surface_operation,
        },
        FoundationSurfaceCase {
            crate_name: "math-signal-core",
            package_surface: math_signal_core::surface::package_surface,
            run: math_signal_core::surface::run_surface_operation,
        },
    ]
}
