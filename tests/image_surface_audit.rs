use runtime_core::{PackageSurface, SurfaceRequest, SurfaceResponse};

type SurfaceFn = fn() -> PackageSurface;
type RunFn = fn(SurfaceRequest) -> Result<SurfaceResponse, String>;

struct ImageSurfaceCase {
    crate_name: &'static str,
    package_surface: SurfaceFn,
    run: RunFn,
    operations: &'static [&'static str],
    workflow: &'static [&'static str],
    debug: &'static [&'static str],
    support: &'static [&'static str],
    invalid_operation: &'static str,
    invalid_input: serde_json::Value,
}

#[test]
fn image_surfaces_expose_expected_operations_and_run_examples() {
    for case in image_surface_cases() {
        let surface = (case.package_surface)();
        let actual = surface
            .operations
            .iter()
            .map(|operation| operation.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            actual, case.operations,
            "{} operation ids changed",
            case.crate_name
        );

        for operation in &surface.operations {
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
        }
    }
}

#[test]
fn image_surfaces_fail_clearly_on_invalid_input() {
    for case in image_surface_cases() {
        let error = (case.run)(SurfaceRequest {
            operation: case.invalid_operation.into(),
            input: case.invalid_input.clone(),
        })
        .expect_err(case.crate_name);
        assert!(
            error.contains("invalid request")
                || error.contains("unsupported")
                || error.contains("invalid")
                || error.contains("must")
                || error.contains("unknown")
                || error.contains("could not infer")
                || error.contains("extension"),
            "{} returned unclear invalid-input error: {error}",
            case.crate_name
        );
    }
}

#[test]
fn image_package_apps_define_audited_operation_groups() {
    for case in image_surface_cases() {
        let app = std::fs::read_to_string(format!("packages/{}-app/src/App.tsx", case.crate_name))
            .unwrap_or_else(|error| panic!("{} app config missing: {error}", case.crate_name));
        assert!(
            app.contains(&format!("defaultOperation: \"{}\"", case.workflow[0])),
            "{} app default operation is not the primary workflow",
            case.crate_name
        );
        assert!(
            app.contains("operationGroups:"),
            "{} app missing operation groups",
            case.crate_name
        );
        assert!(
            app.contains("label: \"Workflow\""),
            "{} app missing Workflow group",
            case.crate_name
        );
        assert!(
            app.contains("label: \"Debug\""),
            "{} app missing Debug group",
            case.crate_name
        );
        for operation in case.workflow {
            assert!(
                app.contains(operation),
                "{} app missing workflow operation {operation}",
                case.crate_name
            );
        }
        for operation in case.debug {
            assert!(
                app.contains(operation),
                "{} app missing debug operation {operation}",
                case.crate_name
            );
        }
        if case.support.is_empty() {
            assert!(
                !app.contains("label: \"Support\""),
                "{} app should not define a Support group",
                case.crate_name
            );
        } else {
            assert!(
                app.contains("label: \"Support\""),
                "{} app missing Support group",
                case.crate_name
            );
            for operation in case.support {
                assert!(
                    app.contains(operation),
                    "{} app missing support operation {operation}",
                    case.crate_name
                );
            }
        }
    }
}

fn assert_structured_response(crate_name: &str, operation: &str, response: &SurfaceResponse) {
    assert_eq!(response.operation.as_str(), operation);
    assert_eq!(
        response.value["operation"], operation,
        "{crate_name} {operation} missing operation field"
    );
    assert!(
        response.value["title"]
            .as_str()
            .is_some_and(|title| !title.is_empty()),
        "{crate_name} {operation} missing title"
    );
    assert!(
        response.value["message"]
            .as_str()
            .is_some_and(|message| !message.is_empty()),
        "{crate_name} {operation} missing message"
    );
    assert!(
        response.value["summary"].is_object(),
        "{crate_name} {operation} missing summary object"
    );
    assert!(
        !response.value["result"].is_null(),
        "{crate_name} {operation} missing nested result"
    );
}

fn image_surface_cases() -> Vec<ImageSurfaceCase> {
    #[allow(unused_mut)]
    let mut cases = vec![
        ImageSurfaceCase {
            crate_name: "image-analysis-core",
            package_surface: image_analysis_core::surface::package_surface,
            run: image_analysis_core::surface::run_surface_operation,
            operations: &[
                "describe",
                "image.core.summary",
                "image.core.lumaHistogram",
                "image.core.maskTensorSummary",
            ],
            workflow: &[
                "image.core.summary",
                "image.core.lumaHistogram",
                "image.core.maskTensorSummary",
            ],
            debug: &["describe"],
            support: &[],
            invalid_operation: "image.core.lumaHistogram",
            invalid_input: serde_json::json!({"image": sample_image_json(), "bins": 0}),
        },
        ImageSurfaceCase {
            crate_name: "image-analysis-captioning",
            package_surface: image_analysis_captioning::surface::package_surface,
            run: image_analysis_captioning::surface::run_surface_operation,
            operations: &[
                "describe",
                "image.captioning.caption",
                "image.captioning.models",
                "image.captioning.schema",
                "image.captioning.imported",
                "image.captioning.rankCaptions",
                "image.captioning.captionReport",
            ],
            workflow: &[
                "image.captioning.caption",
                "image.captioning.imported",
                "image.captioning.rankCaptions",
                "image.captioning.captionReport",
            ],
            debug: &[
                "image.captioning.models",
                "image.captioning.schema",
                "describe",
            ],
            support: &[],
            invalid_operation: "image.captioning.imported",
            invalid_input: serde_json::json!({"captions": [{"text": ""}]}),
        },
        ImageSurfaceCase {
            crate_name: "image-analysis-classification",
            package_surface: image_analysis_classification::surface::package_surface,
            run: image_analysis_classification::surface::run_surface_operation,
            operations: &[
                "describe",
                "image.classification.classify",
                "image.classification.models",
                "image.classification.schema",
                "image.classification.imported",
                "image.classification.topLabels",
                "image.classification.thresholdLabels",
            ],
            workflow: &[
                "image.classification.classify",
                "image.classification.imported",
                "image.classification.topLabels",
                "image.classification.thresholdLabels",
            ],
            debug: &[
                "image.classification.models",
                "image.classification.schema",
                "describe",
            ],
            support: &[],
            invalid_operation: "image.classification.models",
            invalid_input: serde_json::json!({"task": "missing"}),
        },
        ImageSurfaceCase {
            crate_name: "image-analysis-comfyui",
            package_surface: image_analysis_comfyui::surface::package_surface,
            run: image_analysis_comfyui::surface::run_surface_operation,
            operations: &[
                "describe",
                "image.comfyui.workflowSummary",
                "image.comfyui.promptPlan",
                "image.comfyui.assetMap",
            ],
            workflow: &["image.comfyui.promptPlan"],
            debug: &[
                "image.comfyui.workflowSummary",
                "image.comfyui.assetMap",
                "describe",
            ],
            support: &[],
            invalid_operation: "image.comfyui.unknown",
            invalid_input: serde_json::json!({}),
        },
        ImageSurfaceCase {
            crate_name: "image-analysis-detection",
            package_surface: image_analysis_detection::surface::package_surface,
            run: image_analysis_detection::surface::run_surface_operation,
            operations: &[
                "describe",
                "image.detection.colorBlob",
                "image.detection.models",
                "image.detection.boxSummary",
            ],
            workflow: &["image.detection.colorBlob"],
            debug: &[
                "image.detection.models",
                "image.detection.boxSummary",
                "describe",
            ],
            support: &[],
            invalid_operation: "image.detection.boxSummary",
            invalid_input: serde_json::json!({"detections": [{"label": "", "score": 2.0, "region": {"x": 0, "y": 0, "width": 0, "height": 1}}]}),
        },
        ImageSurfaceCase {
            crate_name: "image-analysis-embeddings",
            package_surface: image_analysis_embeddings::surface::package_surface,
            run: image_analysis_embeddings::surface::run_surface_operation,
            operations: &[
                "describe",
                "image.embeddings.models",
                "image.embeddings.schema",
                "image.embeddings.validate",
            ],
            workflow: &["image.embeddings.validate"],
            debug: &[
                "image.embeddings.models",
                "image.embeddings.schema",
                "describe",
            ],
            support: &[],
            invalid_operation: "image.embeddings.validate",
            invalid_input: serde_json::json!({"kind": "missing", "vector": []}),
        },
        ImageSurfaceCase {
            crate_name: "image-analysis-io",
            package_surface: image_analysis_io::surface::package_surface,
            run: image_analysis_io::surface::run_surface_operation,
            operations: &[
                "describe",
                "image.io.supportedFormats",
                "image.io.inferFormat",
                "image.io.plan",
            ],
            workflow: &["image.io.plan"],
            debug: &[
                "image.io.supportedFormats",
                "image.io.inferFormat",
                "describe",
            ],
            support: &[],
            invalid_operation: "image.io.inferFormat",
            invalid_input: serde_json::json!({"path": "image"}),
        },
        ImageSurfaceCase {
            crate_name: "image-analysis-ocr",
            package_surface: image_analysis_ocr::surface::package_surface,
            run: image_analysis_ocr::surface::run_surface_operation,
            operations: &[
                "describe",
                "image.ocr.presets",
                "image.ocr.requestSummary",
                "image.ocr.documentSummary",
            ],
            workflow: &["image.ocr.documentSummary"],
            debug: &["image.ocr.presets", "image.ocr.requestSummary", "describe"],
            support: &[],
            invalid_operation: "image.ocr.documentSummary",
            invalid_input: serde_json::json!({"width": 1, "height": 1}),
        },
        ImageSurfaceCase {
            crate_name: "image-analysis-processing",
            package_surface: image_analysis_processing::surface::package_surface,
            run: image_analysis_processing::surface::run_surface_operation,
            operations: &[
                "describe",
                "image.processing.apply",
                "image.processing.pipeline",
                "image.processing.composite",
                "image.processing.hash",
            ],
            workflow: &[
                "image.processing.apply",
                "image.processing.pipeline",
                "image.processing.composite",
                "image.processing.hash",
            ],
            debug: &["describe"],
            support: &[],
            invalid_operation: "image.processing.hash",
            invalid_input: serde_json::json!({"image": sample_image_json(), "hashSize": 0}),
        },
        ImageSurfaceCase {
            crate_name: "image-analysis-segmentation",
            package_surface: image_analysis_segmentation::surface::package_surface,
            run: image_analysis_segmentation::surface::run_surface_operation,
            operations: &[
                "describe",
                "image.segmentation.model",
                "image.segmentation.promptSummary",
                "image.segmentation.maskSummary",
            ],
            workflow: &["image.segmentation.maskSummary"],
            debug: &[
                "image.segmentation.model",
                "image.segmentation.promptSummary",
                "describe",
            ],
            support: &[],
            invalid_operation: "image.segmentation.maskSummary",
            invalid_input: serde_json::json!({"width": 1}),
        },
        ImageSurfaceCase {
            crate_name: "image-analysis-synthesis",
            package_surface: image_analysis_synthesis::surface::package_surface,
            run: image_analysis_synthesis::surface::run_surface_operation,
            operations: &[
                "describe",
                "image.synthesis.solid",
                "image.synthesis.gradient",
                "image.synthesis.histogram",
            ],
            workflow: &[
                "image.synthesis.solid",
                "image.synthesis.gradient",
                "image.synthesis.histogram",
            ],
            debug: &["describe"],
            support: &[],
            invalid_operation: "image.synthesis.solid",
            invalid_input: serde_json::json!({"width": 0, "height": 1, "color": {"red": 0, "green": 0, "blue": 0}}),
        },
    ];

    cases
}

fn sample_image_json() -> serde_json::Value {
    serde_json::json!({
        "width": 2,
        "height": 2,
        "pixelFormat": "rgb24",
        "stride": null,
        "data": [255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255]
    })
}
