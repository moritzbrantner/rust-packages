use runtime_core::{PackageSurface, SurfaceRequest, SurfaceResponse};

type SurfaceFn = fn() -> PackageSurface;
type RunFn = fn(SurfaceRequest) -> Result<SurfaceResponse, String>;

struct AudioSurfaceCase {
    crate_name: &'static str,
    package_surface: SurfaceFn,
    run: RunFn,
    operations: &'static [&'static str],
    workflow: &'static [&'static str],
    debug: &'static [&'static str],
    invalid_operation: &'static str,
    invalid_input: serde_json::Value,
}

#[test]
fn audio_surfaces_expose_expected_operations_and_run_examples() {
    for case in audio_surface_cases() {
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
fn audio_surfaces_declare_complete_operation_metadata() {
    for case in audio_surface_cases() {
        let surface = (case.package_surface)();
        for operation in &surface.operations {
            let id = operation.id.as_str();
            assert!(
                !operation.name.trim().is_empty(),
                "{} {id} missing name",
                case.crate_name
            );
            assert!(
                operation
                    .description
                    .as_deref()
                    .is_some_and(|description| !description.trim().is_empty()),
                "{} {id} missing description",
                case.crate_name
            );
            assert!(
                operation.example_request.is_object(),
                "{} {id} example request must be an object",
                case.crate_name
            );
            assert!(
                operation.input_schema.is_object(),
                "{} {id} input schema must be an object",
                case.crate_name
            );
            assert!(
                operation.output_schema.is_object(),
                "{} {id} output schema must be an object",
                case.crate_name
            );
        }
    }
}

#[test]
fn audio_surfaces_fail_clearly_on_invalid_input() {
    for case in audio_surface_cases() {
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
                || error.contains("missing")
                || error.contains("unknown")
                || error.contains("empty"),
            "{} returned unclear invalid-input error: {error}",
            case.crate_name
        );
    }
}

#[test]
fn audio_surfaces_return_unknown_operation_errors() {
    for case in audio_surface_cases() {
        let error = (case.run)(SurfaceRequest {
            operation: "missing.operation".into(),
            input: serde_json::json!({}),
        })
        .expect_err(case.crate_name);
        assert!(
            error.contains("unsupported operation"),
            "{} returned unclear unknown-operation error: {error}",
            case.crate_name
        );
    }
}

#[test]
fn audio_package_apps_define_complete_operation_groups() {
    for case in audio_surface_cases() {
        let app = app_source(case.crate_name);
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

        let workflow_group = group_source(&app, "workflow")
            .unwrap_or_else(|| panic!("{} app missing workflow group", case.crate_name));
        let debug_group = group_source(&app, "debug")
            .unwrap_or_else(|| panic!("{} app missing debug group", case.crate_name));

        for operation in case.workflow {
            assert!(
                workflow_group.contains(operation),
                "{} app Workflow group missing {operation}",
                case.crate_name
            );
            assert!(
                !debug_group.contains(operation),
                "{} app Debug group must not include workflow operation {operation}",
                case.crate_name
            );
        }
        for operation in case.debug {
            assert!(
                debug_group.contains(operation),
                "{} app Debug group missing {operation}",
                case.crate_name
            );
            assert!(
                !workflow_group.contains(operation),
                "{} app Workflow group must not include debug operation {operation}",
                case.crate_name
            );
        }
        for operation in case.operations {
            if *operation == "describe" {
                continue;
            }
            assert!(
                workflow_group.contains(operation) || debug_group.contains(operation),
                "{} app leaves {operation} outside Workflow and Debug",
                case.crate_name
            );
        }
        assert!(
            !app.contains("label: \"Support\""),
            "{} app should not define a Support group",
            case.crate_name
        );
    }
}

#[test]
fn demucs_execution_surface_is_server_only_and_non_executing_by_default() {
    let surface = audio_analysis_separation::surface::package_surface();
    let operation = surface
        .operations
        .iter()
        .find(|operation| operation.id.as_str() == "audio.separation.runDemucs")
        .expect("runDemucs operation");
    assert!(!operation.wasm_supported);
    assert!(operation.server_supported);

    let response = audio_analysis_separation::surface::run_surface_operation(SurfaceRequest {
        operation: "audio.separation.runDemucs".into(),
        input: operation.example_request.clone(),
    })
    .expect("runDemucs default response");
    assert_eq!(response.value["result"]["executed"], false);
    assert_eq!(response.value["result"]["wasmSupported"], false);
    assert_eq!(response.value["result"]["serverSupported"], true);
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
    assert!(
        response.value["result"].is_object()
            || response.value["result"].is_array()
            || response.value["summary"]
                .as_object()
                .is_some_and(|summary| !summary.is_empty()),
        "{crate_name} {operation} looks like an empty placeholder response"
    );
}

fn app_source(crate_name: &str) -> String {
    std::fs::read_to_string(format!("packages/{crate_name}-app/src/App.tsx"))
        .unwrap_or_else(|error| panic!("{crate_name} app config missing: {error}"))
}

fn group_source<'a>(app: &'a str, group_id: &str) -> Option<&'a str> {
    let start = app.find(&format!("id: \"{group_id}\""))?;
    let remainder = &app[start..];
    let end = remainder
        .find("\n    },")
        .or_else(|| remainder.find("\n  ],"))
        .unwrap_or(remainder.len());
    Some(&remainder[..end])
}

fn audio_surface_cases() -> Vec<AudioSurfaceCase> {
    vec![
        AudioSurfaceCase {
            crate_name: "audio-analysis-core",
            package_surface: audio_analysis_core::surface::package_surface,
            run: audio_analysis_core::surface::run_surface_operation,
            operations: &[
                "describe",
                "audio.levels",
                "audio.frames",
                "audio.timestamps",
            ],
            workflow: &["audio.levels", "audio.frames"],
            debug: &["describe", "audio.timestamps"],
            invalid_operation: "audio.levels",
            invalid_input: serde_json::json!({"samples": [], "sampleRate": 48000, "channels": 1}),
        },
        AudioSurfaceCase {
            crate_name: "audio-analysis-fourier",
            package_surface: audio_analysis_fourier::surface::package_surface,
            run: audio_analysis_fourier::surface::run_surface_operation,
            operations: &[
                "describe",
                "audio.fourier.spectrum",
                "audio.fourier.spectrogram",
                "audio.fourier.features",
            ],
            workflow: &[
                "audio.fourier.spectrum",
                "audio.fourier.spectrogram",
                "audio.fourier.features",
            ],
            debug: &["describe"],
            invalid_operation: "audio.fourier.spectrum",
            invalid_input: serde_json::json!({"samples": [], "sampleRate": 48000, "fftSize": 1024}),
        },
        AudioSurfaceCase {
            crate_name: "audio-analysis-io",
            package_surface: audio_analysis_io::surface::package_surface,
            run: audio_analysis_io::surface::run_surface_operation,
            operations: &[
                "describe",
                "audio.io.inputPlan",
                "audio.io.waveformBatchSummary",
                "audio.io.wavSummary",
                "audio.io.probePlan",
                "audio.io.decodePlan",
                "audio.io.editPlan",
                "audio.io.splitPlan",
                "audio.io.joinPlan",
                "audio.io.ffmpegFilterPlan",
            ],
            workflow: &["audio.io.waveformBatchSummary", "audio.io.wavSummary"],
            debug: &[
                "describe",
                "audio.io.inputPlan",
                "audio.io.probePlan",
                "audio.io.decodePlan",
                "audio.io.editPlan",
                "audio.io.splitPlan",
                "audio.io.joinPlan",
                "audio.io.ffmpegFilterPlan",
            ],
            invalid_operation: "audio.io.waveformBatchSummary",
            invalid_input: serde_json::json!({"waveform": {"sampleRate": 48000, "channels": 1, "samples": []}}),
        },
        AudioSurfaceCase {
            crate_name: "audio-analysis-pitch",
            package_surface: audio_analysis_pitch::surface::package_surface,
            run: audio_analysis_pitch::surface::run_surface_operation,
            operations: &[
                "describe",
                "audio.pitch.estimate",
                "audio.pitch.track",
                "audio.pitch.noteName",
                "audio.pitch.chroma",
            ],
            workflow: &[
                "audio.pitch.estimate",
                "audio.pitch.track",
                "audio.pitch.chroma",
            ],
            debug: &["describe", "audio.pitch.noteName"],
            invalid_operation: "audio.pitch.estimate",
            invalid_input: serde_json::json!({"samples": [], "sampleRate": 48000}),
        },
        AudioSurfaceCase {
            crate_name: "audio-analysis-processing",
            package_surface: audio_analysis_processing::surface::package_surface,
            run: audio_analysis_processing::surface::run_surface_operation,
            operations: &[
                "describe",
                "audio.processing.apply",
                "audio.processing.effectsCatalog",
                "audio.processing.offlineEdit",
                "audio.processing.mixdown",
                "audio.processing.preset",
                "audio.processing.energy",
                "audio.processing.loudness",
                "audio.processing.chainSummary",
            ],
            workflow: &[
                "audio.processing.apply",
                "audio.processing.offlineEdit",
                "audio.processing.mixdown",
                "audio.processing.preset",
                "audio.processing.energy",
                "audio.processing.loudness",
            ],
            debug: &[
                "describe",
                "audio.processing.effectsCatalog",
                "audio.processing.chainSummary",
            ],
            invalid_operation: "audio.processing.apply",
            invalid_input: serde_json::json!({"samples": [], "sampleRate": 48000, "channels": 1}),
        },
        AudioSurfaceCase {
            crate_name: "audio-analysis-recognition",
            package_surface: audio_analysis_recognition::surface::package_surface,
            run: audio_analysis_recognition::surface::run_surface_operation,
            operations: &[
                "describe",
                "audio.recognition.embed",
                "audio.recognition.compare",
                "audio.recognition.search",
            ],
            workflow: &[
                "audio.recognition.embed",
                "audio.recognition.compare",
                "audio.recognition.search",
            ],
            debug: &["describe"],
            invalid_operation: "audio.recognition.embed",
            invalid_input: serde_json::json!({"samples": [], "sampleRate": 48000}),
        },
        AudioSurfaceCase {
            crate_name: "audio-analysis-rhythm",
            package_surface: audio_analysis_rhythm::surface::package_surface,
            run: audio_analysis_rhythm::surface::run_surface_operation,
            operations: &[
                "describe",
                "audio.rhythm.onsets",
                "audio.rhythm.tempo",
                "audio.rhythm.beatGrid",
            ],
            workflow: &[
                "audio.rhythm.onsets",
                "audio.rhythm.tempo",
                "audio.rhythm.beatGrid",
            ],
            debug: &["describe"],
            invalid_operation: "audio.rhythm.onsets",
            invalid_input: serde_json::json!({"samples": [], "sampleRate": 48000}),
        },
        AudioSurfaceCase {
            crate_name: "audio-analysis-separation",
            package_surface: audio_analysis_separation::surface::package_surface,
            run: audio_analysis_separation::surface::run_surface_operation,
            operations: &[
                "describe",
                "audio.separation.models",
                "audio.separation.plan",
                "audio.separation.expectedStems",
                "audio.separation.runDemucs",
            ],
            workflow: &[
                "audio.separation.expectedStems",
                "audio.separation.runDemucs",
            ],
            debug: &[
                "describe",
                "audio.separation.models",
                "audio.separation.plan",
            ],
            invalid_operation: "audio.separation.expectedStems",
            invalid_input: serde_json::json!({"format": "aac"}),
        },
        AudioSurfaceCase {
            crate_name: "audio-analysis-speakers",
            package_surface: audio_analysis_speakers::surface::package_surface,
            run: audio_analysis_speakers::surface::run_surface_operation,
            operations: &[
                "describe",
                "audio.speakers.embed",
                "audio.speakers.identify",
                "audio.speakers.assignTranscript",
                "audio.speakers.vad",
                "audio.speakers.diarize",
            ],
            workflow: &[
                "audio.speakers.vad",
                "audio.speakers.diarize",
                "audio.speakers.embed",
                "audio.speakers.identify",
                "audio.speakers.assignTranscript",
            ],
            debug: &["describe"],
            invalid_operation: "audio.speakers.vad",
            invalid_input: serde_json::json!({"samples": [], "sampleRate": 48000, "channels": 1}),
        },
        AudioSurfaceCase {
            crate_name: "audio-analysis-synthesis",
            package_surface: audio_analysis_synthesis::surface::package_surface,
            run: audio_analysis_synthesis::surface::run_surface_operation,
            operations: &[
                "describe",
                "audio.synthesis.tone",
                "audio.synthesis.timeline",
                "audio.synthesis.fromEvents",
                "audio.synthesis.clickTrack",
            ],
            workflow: &[
                "audio.synthesis.tone",
                "audio.synthesis.timeline",
                "audio.synthesis.fromEvents",
                "audio.synthesis.clickTrack",
            ],
            debug: &["describe"],
            invalid_operation: "audio.synthesis.tone",
            invalid_input: serde_json::json!({"frequencyHz": -1.0, "durationSeconds": 1.0}),
        },
        AudioSurfaceCase {
            crate_name: "audio-generation-midi",
            package_surface: audio_generation_midi::surface::package_surface,
            run: audio_generation_midi::surface::run_surface_operation,
            operations: &[
                "describe",
                "audio.midi.note",
                "audio.midi.encode",
                "audio.midi.render",
                "audio.midi.fromPitchTrack",
            ],
            workflow: &[
                "audio.midi.render",
                "audio.midi.encode",
                "audio.midi.fromPitchTrack",
            ],
            debug: &["describe", "audio.midi.note"],
            invalid_operation: "audio.midi.note",
            invalid_input: serde_json::json!({"name": "bad"}),
        },
    ]
}
