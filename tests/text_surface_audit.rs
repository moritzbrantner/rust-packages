use video_analysis_core::runtime::{PackageSurface, SurfaceRequest, SurfaceResponse};

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
            ],
            workflow: &[
                "lexical.analyze",
                "lexical.keywords",
                "lexical.corpusSearch",
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
            operations: &["describe", "linguistics.analyze", "linguistics.entities"],
            workflow: &["linguistics.analyze", "linguistics.entities"],
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
            debug: &["describe"],
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
            ],
            workflow: &["retrieval.search", "retrieval.chunk", "retrieval.rerank"],
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
                "classification.classify",
                "classification.sentiment",
                "classification.zeroShot",
            ],
            workflow: &[
                "classification.classify",
                "classification.sentiment",
                "classification.zeroShot",
            ],
            debug: &["classification.models", "describe"],
            support: &[],
            invalid_operation: "classification.models",
            invalid_input: serde_json::json!({"task": "missing"}),
        },
        TextSurfaceCase {
            crate_name: "text-question-answering",
            package_surface: text_question_answering::surface::package_surface,
            run: text_question_answering::surface::run_surface_operation,
            operations: &["describe", "qa.models", "qa.answer"],
            workflow: &["qa.answer"],
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
                "generation.synthesizeTerms",
            ],
            workflow: &[
                "generation.markovGenerate",
                "generation.markovPredict",
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
            operations: &["describe", "runtime.tokenizeSummary", "runtime.softmax"],
            workflow: &["runtime.tokenizeSummary"],
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
                "transcripts.formatSrt",
            ],
            workflow: &[
                "transcripts.parse",
                "transcripts.normalize",
                "transcripts.formatSrt",
            ],
            debug: &["describe"],
            support: &[],
            invalid_operation: "transcripts.parse",
            invalid_input: serde_json::json!({"format": "srt"}),
        },
    ]
}
