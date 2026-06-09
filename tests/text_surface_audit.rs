use runtime_core::{PackageSurface, SurfaceRequest, SurfaceResponse};

type SurfaceFn = fn() -> PackageSurface;
type RunFn = fn(SurfaceRequest) -> Result<SurfaceResponse, String>;

struct TextSurfaceCase {
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
fn text_surfaces_expose_expected_operations_and_run_examples() {
    for case in text_surface_cases() {
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
fn text_surfaces_declare_release_contracts() {
    for case in text_surface_cases() {
        let surface = (case.package_surface)();
        for operation in &surface.operations {
            let id = operation.id.as_str();
            assert_eq!(
                operation.input_schema["type"], "object",
                "{} {id} input schema must be an object",
                case.crate_name
            );
            assert_eq!(
                operation.input_schema["additionalProperties"], false,
                "{} {id} input schema must reject undeclared top-level fields",
                case.crate_name
            );
            assert_eq!(
                operation.input_schema["xReleaseStability"], "stable",
                "{} {id} missing stable release marker",
                case.crate_name
            );
            assert_eq!(
                operation.input_schema["xContractPolicy"], "additiveOnly",
                "{} {id} missing additive-only contract policy",
                case.crate_name
            );
            assert_eq!(
                operation.input_schema["xOperationCategory"],
                expected_category(&case, id),
                "{} {id} has the wrong operation category",
                case.crate_name
            );
            assert!(
                operation.input_schema["properties"].is_object(),
                "{} {id} missing input properties",
                case.crate_name
            );
            assert!(
                operation.input_schema["required"].is_array(),
                "{} {id} missing required-field list",
                case.crate_name
            );
            assert_eq!(
                operation.output_schema["required"],
                serde_json::json!(["operation", "title", "message", "summary", "result"]),
                "{} {id} output schema must preserve the structured response shape",
                case.crate_name
            );

            let roundtrip = serde_json::from_value::<runtime_core::SurfaceOperation>(
                serde_json::to_value(operation).expect("serialize operation"),
            )
            .expect("deserialize operation");
            assert_eq!(roundtrip.id.as_str(), id);
        }
    }
}

#[test]
fn text_surfaces_fail_clearly_on_invalid_input() {
    for case in text_surface_cases() {
        let error = (case.run)(SurfaceRequest {
            operation: case.invalid_operation.into(),
            input: case.invalid_input.clone(),
        })
        .expect_err(case.crate_name);
        assert!(
            error.contains("invalid request")
                || error.contains("unsupported")
                || error.contains("must")
                || error.contains("unknown"),
            "{} returned unclear invalid-input error: {error}",
            case.crate_name
        );
    }
}

#[test]
fn text_surfaces_return_typed_unknown_operation_errors() {
    for case in text_surface_cases() {
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
fn text_package_apps_define_audited_operation_groups() {
    for case in text_surface_cases() {
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

#[test]
fn new_release_text_operations_have_app_presets_and_benchmarks() {
    let generation = app_source("text-generation");
    assert_operation_has_app_preset(&generation, "generation.perplexity");
    assert_operation_has_benchmark(&generation, "generation.perplexity");

    let classification = app_source("text-classification");
    assert_operation_has_app_preset(&classification, "classification.schema");

    let embeddings = app_source("text-embeddings");
    assert_operation_has_app_preset(&embeddings, "embeddings.backends");

    let lexical = app_source("text-lexical");
    assert_operation_has_app_preset(&lexical, "lexical.corpusStats");
    assert_operation_has_benchmark(&lexical, "lexical.corpusStats");

    let linguistics = app_source("text-linguistics");
    assert_operation_has_app_preset(&linguistics, "linguistics.language");
    assert_operation_has_benchmark(&linguistics, "linguistics.language");

    let retrieval = app_source("text-retrieval");
    assert_operation_has_app_preset(&retrieval, "retrieval.snapshotPlan");
    assert_operation_has_benchmark(&retrieval, "retrieval.snapshotPlan");

    let transcripts = app_source("text-transcripts");
    assert_operation_has_app_preset(&transcripts, "transcripts.formatWebVtt");
    assert_operation_has_app_preset(&transcripts, "transcripts.toTextSegments");
    assert_operation_has_benchmark(&transcripts, "transcripts.formatWebVtt");
    assert_operation_has_benchmark(&transcripts, "transcripts.toTextSegments");
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

fn app_source(crate_name: &str) -> String {
    std::fs::read_to_string(format!("packages/{crate_name}-app/src/App.tsx"))
        .unwrap_or_else(|error| panic!("{crate_name} app config missing: {error}"))
}

fn assert_operation_has_app_preset(app: &str, operation: &str) {
    assert!(
        app.contains(&format!("operation: \"{operation}\"")),
        "app missing preset for {operation}"
    );
}

fn assert_operation_has_benchmark(app: &str, operation: &str) {
    let benchmark_start = app
        .find("benchmarkScenarios:")
        .expect("app missing benchmark scenarios");
    assert!(
        app[benchmark_start..].contains(&format!("operation: \"{operation}\"")),
        "app missing benchmark scenario for {operation}"
    );
}

fn expected_category(case: &TextSurfaceCase, operation: &str) -> &'static str {
    if case.debug.contains(&operation) {
        "debug"
    } else if case.support.contains(&operation) {
        "support"
    } else {
        "workflow"
    }
}

fn text_surface_cases() -> Vec<TextSurfaceCase> {
    vec![
        TextSurfaceCase {
            crate_name: "text-core",
            package_surface: text_core::surface::package_surface,
            run: text_core::surface::run_surface_operation,
            operations: &[
                "describe",
                "text.statistics",
                "text.normalize",
                "text.tokenize",
                "text.boundaries",
            ],
            workflow: &[
                "text.tokenize",
                "text.statistics",
                "text.normalize",
                "text.boundaries",
            ],
            debug: &["describe"],
            support: &[],
            invalid_operation: "text.statistics",
            invalid_input: serde_json::json!({"missing": true}),
        },
        TextSurfaceCase {
            crate_name: "text-lexical",
            package_surface: text_lexical::surface::package_surface,
            run: text_lexical::surface::run_surface_operation,
            operations: &[
                "describe",
                "lexical.analyze",
                "lexical.keywords",
                "lexical.corpusSearch",
                "lexical.corpusStats",
            ],
            workflow: &[
                "lexical.analyze",
                "lexical.keywords",
                "lexical.corpusSearch",
                "lexical.corpusStats",
            ],
            debug: &["describe"],
            support: &[],
            invalid_operation: "lexical.corpusSearch",
            invalid_input: serde_json::json!({"documents": [], "query": "x", "mode": "missing"}),
        },
        TextSurfaceCase {
            crate_name: "text-linguistics",
            package_surface: text_linguistics::surface::package_surface,
            run: text_linguistics::surface::run_surface_operation,
            operations: &[
                "describe",
                "linguistics.analyze",
                "linguistics.entities",
                "linguistics.language",
            ],
            workflow: &[
                "linguistics.analyze",
                "linguistics.entities",
                "linguistics.language",
            ],
            debug: &["describe"],
            support: &[],
            invalid_operation: "linguistics.analyze",
            invalid_input: serde_json::json!({"text": "hello", "profile": "missing"}),
        },
        TextSurfaceCase {
            crate_name: "text-embeddings",
            package_surface: text_embeddings::surface::package_surface,
            run: text_embeddings::surface::run_surface_operation,
            operations: &[
                "describe",
                "embeddings.backends",
                "embeddings.embed",
                "embeddings.similarity",
                "embeddings.semanticSearch",
                "embeddings.relatedTerms",
            ],
            workflow: &[
                "embeddings.embed",
                "embeddings.similarity",
                "embeddings.semanticSearch",
                "embeddings.relatedTerms",
            ],
            debug: &["embeddings.backends", "describe"],
            support: &[],
            invalid_operation: "embeddings.embed",
            invalid_input: serde_json::json!({"texts": []}),
        },
        TextSurfaceCase {
            crate_name: "text-retrieval",
            package_surface: text_retrieval::surface::package_surface,
            run: text_retrieval::surface::run_surface_operation,
            operations: &[
                "describe",
                "retrieval.chunk",
                "retrieval.search",
                "retrieval.rerank",
                "retrieval.snapshotPlan",
            ],
            workflow: &[
                "retrieval.search",
                "retrieval.chunk",
                "retrieval.rerank",
                "retrieval.snapshotPlan",
            ],
            debug: &["describe"],
            support: &[],
            invalid_operation: "retrieval.search",
            invalid_input: serde_json::json!({"documents": []}),
        },
        TextSurfaceCase {
            crate_name: "text-analysis",
            package_surface: text_analysis::surface::package_surface,
            run: text_analysis::surface::run_surface_operation,
            operations: &[
                "describe",
                "analysis.describe",
                "analysis.document",
                "analysis.corpus",
                "analysis.similarity",
            ],
            workflow: &[
                "analysis.document",
                "analysis.corpus",
                "analysis.similarity",
            ],
            debug: &["analysis.describe", "describe"],
            support: &[],
            invalid_operation: "analysis.similarity",
            invalid_input: serde_json::json!({"left": "a", "right": "b", "mode": "missing"}),
        },
        TextSurfaceCase {
            crate_name: "text-classification",
            package_surface: text_classification::surface::package_surface,
            run: text_classification::surface::run_surface_operation,
            operations: &[
                "describe",
                "classification.models",
                "classification.schema",
                "classification.classify",
                "classification.sentiment",
                "classification.zeroShot",
            ],
            workflow: &[
                "classification.classify",
                "classification.sentiment",
                "classification.zeroShot",
            ],
            debug: &["classification.models", "classification.schema", "describe"],
            support: &[],
            invalid_operation: "classification.models",
            invalid_input: serde_json::json!({"task": "missing"}),
        },
        TextSurfaceCase {
            crate_name: "text-question-answering",
            package_surface: text_question_answering::surface::package_surface,
            run: text_question_answering::surface::run_surface_operation,
            operations: &[
                "describe",
                "qa.models",
                "qa.answer",
                "qa.answerWithRetrieval",
                "qa.answerBatch",
            ],
            workflow: &["qa.answer", "qa.answerWithRetrieval", "qa.answerBatch"],
            debug: &["qa.models", "describe"],
            support: &[],
            invalid_operation: "qa.answer",
            invalid_input: serde_json::json!({"question": "missing context"}),
        },
        TextSurfaceCase {
            crate_name: "text-generation",
            package_surface: text_generation::surface::package_surface,
            run: text_generation::surface::run_surface_operation,
            operations: &[
                "describe",
                "generation.markovPredict",
                "generation.markovGenerate",
                "generation.perplexity",
                "generation.synthesizeTerms",
            ],
            workflow: &[
                "generation.markovGenerate",
                "generation.markovPredict",
                "generation.perplexity",
                "generation.synthesizeTerms",
            ],
            debug: &["describe"],
            support: &[],
            invalid_operation: "generation.markovPredict",
            invalid_input: serde_json::json!({"trainingTexts": [], "context": ["rust"]}),
        },
        TextSurfaceCase {
            crate_name: "text-generation-linguistics",
            package_surface: text_generation_linguistics::surface::package_surface,
            run: text_generation_linguistics::surface::run_surface_operation,
            operations: &[
                "describe",
                "generationLinguistics.analysisTerms",
                "generationLinguistics.synthesizeFromAnalysis",
                "generationLinguistics.trainAnalysis",
            ],
            workflow: &[
                "generationLinguistics.synthesizeFromAnalysis",
                "generationLinguistics.analysisTerms",
                "generationLinguistics.trainAnalysis",
            ],
            debug: &["describe"],
            support: &[],
            invalid_operation: "generationLinguistics.trainAnalysis",
            invalid_input: serde_json::json!({"text": "hello", "mode": "missing"}),
        },
        TextSurfaceCase {
            crate_name: "text-model-runtime",
            package_surface: text_model_runtime::surface::package_surface,
            run: text_model_runtime::surface::run_surface_operation,
            operations: &[
                "describe",
                "runtime.tokenizeSummary",
                "runtime.bundleCheck",
                "runtime.downloadBundle",
                "runtime.onnxQaProbe",
                "runtime.tokenizerProbe",
                "runtime.softmax",
            ],
            workflow: &[
                "runtime.onnxQaProbe",
                "runtime.downloadBundle",
                "runtime.bundleCheck",
                "runtime.tokenizeSummary",
                "runtime.tokenizerProbe",
            ],
            debug: &["describe"],
            support: &["runtime.softmax"],
            invalid_operation: "runtime.softmax",
            invalid_input: serde_json::json!({"logits": "bad"}),
        },
        TextSurfaceCase {
            crate_name: "text-transcripts",
            package_surface: text_transcripts::surface::package_surface,
            run: text_transcripts::surface::run_surface_operation,
            operations: &[
                "describe",
                "transcripts.parse",
                "transcripts.normalize",
                "transcripts.importWhisperX",
                "transcripts.formatSrt",
                "transcripts.formatWebVtt",
                "transcripts.toTextSegments",
            ],
            workflow: &[
                "transcripts.parse",
                "transcripts.normalize",
                "transcripts.importWhisperX",
                "transcripts.formatSrt",
                "transcripts.formatWebVtt",
                "transcripts.toTextSegments",
            ],
            debug: &["describe"],
            support: &[],
            invalid_operation: "transcripts.parse",
            invalid_input: serde_json::json!({"format": "srt"}),
        },
    ]
}
