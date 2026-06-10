use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn text_core_stays_independent_of_specialized_text_and_audio_crates() {
    let manifest = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/text/text-core/Cargo.toml"),
    )
    .expect("read text-core manifest");

    for forbidden in [
        "text-transcripts",
        "text-retrieval",
        "text-linguistics",
        "audio-analysis",
        "video-analysis-models",
    ] {
        assert!(
            !manifest.contains(forbidden),
            "text-core must not depend on specialized contract consumer `{forbidden}`"
        );
    }
}

#[test]
fn text_generation_core_stays_independent_of_linguistics() {
    let manifest = read_manifest("crates/text/text-generation/Cargo.toml");
    assert!(
        !manifest.contains("text-linguistics"),
        "text-generation must keep linguistic adapters in text-generation-linguistics"
    );
}

#[test]
fn text_retrieval_transcript_ingestion_is_feature_gated() {
    let manifest = read_manifest("crates/text/text-retrieval/Cargo.toml");
    assert!(
        manifest.contains("text-transcripts = { workspace = true, optional = true }"),
        "text-retrieval must only depend on text-transcripts as an optional adapter"
    );
    assert!(
        manifest.contains("transcripts = [\"dep:text-transcripts\"]"),
        "text-retrieval must expose transcript ingestion through an explicit feature"
    );
}

#[test]
fn text_analysis_model_backed_defaults_do_not_auto_download() {
    let options = text_analysis::DocumentAnalysisOptions::model_backed();

    match options.linguistic_depth {
        text_analysis::LinguisticDepth::LocalModel {
            auto_download,
            download_progress,
            ..
        } => {
            assert!(!auto_download);
            assert!(!download_progress);
        }
        other => panic!("expected local model depth, got {other:?}"),
    }
}

#[test]
fn native_text_model_runtime_dependencies_are_feature_gated() {
    for path in [
        "crates/text/text-analysis/Cargo.toml",
        "crates/text/text-embeddings/Cargo.toml",
        "crates/text/text-linguistics/Cargo.toml",
        "crates/text/text-model-runtime/Cargo.toml",
        "crates/text/text-retrieval/Cargo.toml",
        "crates/text/text-transcripts/Cargo.toml",
    ] {
        let manifest = read_manifest(path);
        assert!(
            manifest.contains("default = []"),
            "{path} must keep native/model runtime dependencies out of default builds"
        );
    }

    let linguistics_manifest = read_manifest("crates/text/text-linguistics/Cargo.toml");
    assert!(
        linguistics_manifest.contains("jobs-core = { workspace = true, optional = true }"),
        "text-linguistics must not require jobs-core in default builds"
    );
    assert!(
        linguistics_manifest.contains("text-transcripts = { workspace = true, optional = true }"),
        "text-linguistics transcript adapters must be feature-gated"
    );
    assert!(
        linguistics_manifest.contains("model-runtime = { workspace = true, optional = true"),
        "text-linguistics must use model-runtime for optional model bundle support"
    );

    let runtime_manifest = read_manifest("crates/text/text-model-runtime/Cargo.toml");
    assert!(
        runtime_manifest.contains("model-runtime = { workspace = true, optional = true"),
        "text-model-runtime must use model-runtime for optional model bundle support"
    );

    for (path, dependencies) in [
        (
            "crates/text/text-embeddings/Cargo.toml",
            &[
                "tokenizers",
                "candle-core",
                "candle-nn",
                "candle-transformers",
            ][..],
        ),
        (
            "crates/text/text-linguistics/Cargo.toml",
            &[
                "tokenizers",
                "candle-core",
                "candle-nn",
                "candle-transformers",
            ][..],
        ),
        (
            "crates/text/text-model-runtime/Cargo.toml",
            &[
                "tokenizers",
                "candle-core",
                "candle-nn",
                "candle-transformers",
            ][..],
        ),
    ] {
        let manifest = read_manifest(path);
        for dependency in dependencies {
            assert!(
                manifest.contains(&format!(
                    "{dependency} = {{ workspace = true, optional = true"
                )),
                "{path} must keep native/model dependency `{dependency}` optional"
            );
        }
    }

    for path in [
        "crates/text/text-embeddings/Cargo.toml",
        "crates/text/text-model-runtime/Cargo.toml",
    ] {
        let manifest = read_manifest(path);
        assert!(
            manifest.contains("runtime-onnx.workspace = true")
                || manifest.contains("runtime-onnx = { workspace = true"),
            "{path} must use runtime-onnx instead of depending directly on ort/ndarray"
        );
        assert!(
            !manifest_declares_dependency(&manifest, "ort")
                && !manifest_declares_dependency(&manifest, "ndarray"),
            "{path} must not depend directly on ort or ndarray"
        );
    }
}

#[test]
fn generic_model_runtime_stays_domain_independent() {
    let manifest = read_manifest("crates/runtime/model-runtime/Cargo.toml");
    for forbidden in ["audio-analysis", "image-analysis", "text-", "comfyui-"] {
        assert!(
            !manifest.contains(forbidden),
            "model-runtime must stay domain-independent and not depend on `{forbidden}`"
        );
    }
    assert!(
        manifest.contains("jobs-core.workspace = true"),
        "model-runtime should build model artifact handling on jobs-core generics"
    );
    assert!(
        manifest.contains("runtime-core.workspace = true"),
        "model-runtime surfaces should use runtime-core DTOs"
    );
    assert!(
        !manifest.contains("video-analysis-core.workspace = true"),
        "model-runtime must not depend on video-analysis-core for runtime DTOs"
    );
}

#[test]
fn root_facade_does_not_promote_domain_model_crates() {
    let source = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs"))
        .expect("read root facade");
    for forbidden in [
        "pub use video_analysis_models as models;",
        concat!("pub use audio_analysis_", "models as audio_models;"),
        "pub use image_analysis_models as image_models;",
    ] {
        assert!(
            !source.contains(forbidden),
            "root facade must expose capabilities and model_runtime, not `{forbidden}`"
        );
    }
    assert!(
        source.contains("pub use model_runtime;"),
        "root facade should expose the generic model infrastructure under model_runtime"
    );
    assert!(
        !source.contains("pub use audio_analysis_tasks"),
        "root facade must not promote aggregate audio task crates"
    );
    assert!(
        source.contains("pub use audio_analysis_recognition as audio_recognition;"),
        "root facade should expose concrete audio recognition APIs"
    );
    assert!(
        !source.contains("pub use image_analysis_tasks"),
        "root facade must not promote aggregate image task crates"
    );
    assert!(
        source.contains("pub use image_analysis_classification as image_classification;"),
        "root facade should expose concrete image classification APIs"
    );
    assert!(
        source.contains("pub use text_classification;"),
        "root facade should expose concrete text classification APIs"
    );
}

#[test]
fn concrete_text_crates_do_not_expose_aggregate_nlp_surfaces() {
    let classification = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/text/text-classification/src/lib.rs"),
    )
    .expect("read text-classification source");
    for forbidden in [
        "pub struct EmbeddingRequest",
        "pub struct SummaryRequest",
        "pub struct RerankRequest",
        "pub struct QuestionAnsweringRequest",
        "pub fn embed_texts",
        "pub fn summarize",
        "pub fn rerank",
        "pub fn answer_question",
    ] {
        assert!(
            !classification.contains(forbidden),
            "text-classification must not expose unrelated aggregate NLP API `{forbidden}`"
        );
    }

    let qa = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("crates/text/text-question-answering/src/lib.rs"),
    )
    .expect("read text-question-answering source");
    for forbidden in [
        "pub struct TextClassificationRequest",
        "pub struct SentimentRequest",
        "pub struct EmbeddingRequest",
        "pub struct ZeroShotClassificationRequest",
        "pub struct SummaryRequest",
        "pub struct RerankRequest",
        "pub fn classify_text",
        "pub fn analyze_sentiment",
        "pub fn embed_texts",
        "pub fn summarize",
        "pub fn rerank",
    ] {
        assert!(
            !qa.contains(forbidden),
            "text-question-answering must not expose unrelated aggregate NLP API `{forbidden}`"
        );
    }
}

#[test]
fn onnx_runtime_is_domain_neutral_and_task_crates_use_it() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let runtime_manifest = read_manifest("crates/runtime/runtime-onnx/Cargo.toml");
    for required in [
        "ort = { workspace = true, optional = true",
        "ndarray = { workspace = true, optional = true",
        "onnxruntime = [\"dep:ort\", \"dep:ndarray\"]",
    ] {
        assert!(
            runtime_manifest.contains(required),
            "runtime-onnx must keep ONNX Runtime mechanics optional through `{required}`"
        );
    }
    for forbidden in [
        "image-analysis",
        "video-analysis",
        "text-",
        "model-runtime",
        "comfyui-",
    ] {
        assert!(
            !runtime_manifest.contains(forbidden),
            "runtime-onnx must stay domain-neutral and not depend on `{forbidden}`"
        );
    }

    let mut direct_ort = Vec::new();
    collect_manifests(&root.join("crates"), &mut |manifest| {
        let source = fs::read_to_string(manifest).expect("read manifest");
        if manifest_declares_dependency(&source, "ort")
            && !manifest.ends_with("crates/runtime/runtime-onnx/Cargo.toml")
        {
            direct_ort.push(manifest.display().to_string());
        }
    });
    assert!(
        direct_ort.is_empty(),
        "only runtime-onnx may depend directly on ort: {}",
        direct_ort.join(", ")
    );

    for path in [
        "crates/image/image-analysis-classification/Cargo.toml",
        "crates/image/image-analysis-captioning/Cargo.toml",
        "crates/image/image-analysis-detection/Cargo.toml",
        "crates/image/image-analysis-embeddings/Cargo.toml",
        "crates/video/video-analysis-posture/Cargo.toml",
        "crates/video/video-analysis-recognition/Cargo.toml",
        "crates/text/text-model-runtime/Cargo.toml",
        "crates/text/text-embeddings/Cargo.toml",
    ] {
        let manifest = read_manifest(path);
        assert!(
            manifest.contains("runtime-onnx"),
            "{path} must use runtime-onnx for ONNX session execution"
        );
    }
}

#[test]
fn compatibility_task_and_model_crates_are_removed() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for path in [
        "crates/audio/audio-analysis-tasks",
        "crates/bindings/audio-analysis-tasks-wasm",
        "crates/image/image-analysis-tasks",
        "crates/image/image-analysis-models",
        "crates/bindings/image-analysis-tasks-wasm",
        "crates/bindings/image-analysis-models-wasm",
        "crates/text/text-nlp-tasks",
        "crates/text/text-nlp-models",
        "crates/text/text-nlp-cli",
        "crates/text/text-nlp-server",
        "crates/bindings/text-nlp-tasks-wasm",
        "crates/bindings/text-nlp-models-wasm",
        "crates/video/video-analysis-models",
        "crates/bindings/video-analysis-models-wasm",
    ] {
        assert!(
            !root.join(path).exists(),
            "compatibility crate `{path}` must be removed"
        );
    }

    let manifest = read_manifest("Cargo.toml");
    for forbidden in [
        "audio-analysis-tasks",
        "image-analysis-tasks",
        "image-analysis-models",
        "text-nlp-tasks",
        "text-nlp-models",
        "video-analysis-models",
    ] {
        assert!(
            !manifest.contains(forbidden),
            "workspace manifest must not keep removed compatibility crate `{forbidden}`"
        );
    }
}

fn read_manifest(path: &str) -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(path))
        .unwrap_or_else(|err| panic!("read manifest `{path}`: {err}"))
}

fn manifest_declares_dependency(source: &str, dependency: &str) -> bool {
    let mut in_dependency_section = false;
    let inline_prefix = format!("{dependency} ");
    let table_prefix = format!("{dependency}.");

    for line in source.lines() {
        let line = line
            .split_once('#')
            .map_or(line, |(before, _)| before)
            .trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            in_dependency_section = line.contains("dependencies");
            continue;
        }
        if !in_dependency_section {
            continue;
        }

        if line == dependency
            || line.starts_with(&inline_prefix)
            || line.starts_with(&table_prefix)
            || line
                .strip_prefix(dependency)
                .is_some_and(|rest| rest.trim_start().starts_with('='))
        {
            return true;
        }
    }

    false
}

#[test]
fn audio_analysis_models_crate_is_removed() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    assert!(
        !root.join("crates/audio/audio-analysis-models").exists(),
        "audio-analysis-models must not remain in the workspace"
    );
}

#[test]
fn no_audio_analysis_models_imports_remain() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut violations = Vec::new();
    for dir in ["crates", "src", "tests", "prototypes"] {
        collect_text_sources(&root.join(dir), &mut |path| {
            let source = fs::read_to_string(path).expect("read source");
            if source.contains(concat!("use audio_analysis_", "models"))
                || source.contains(concat!("audio_analysis_", "models::"))
                || source.contains(concat!("audio-analysis-", "models.workspace"))
                || source.contains(concat!("name = \"audio-analysis-", "models\""))
            {
                violations.push(path.display().to_string());
            }
        });
    }

    assert!(
        violations.is_empty(),
        "audio-analysis-models references must be removed: {}",
        violations.join(", ")
    );
}

#[test]
fn audio_tasks_crate_is_execution_free_and_jobs_are_feature_gated() {
    let separation_manifest = read_manifest("crates/audio/audio-analysis-separation/Cargo.toml");
    assert!(
        separation_manifest.contains("jobs-core = { workspace = true, optional = true }"),
        "source separation job helpers must keep jobs-core optional"
    );
    assert!(
        separation_manifest.contains("jobs = [\"dep:jobs-core\"]"),
        "source separation job helpers must be behind a jobs feature"
    );
}

#[test]
fn audio_asr_contract_uses_text_transcript_contracts() {
    let source = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("crates/audio/audio-analysis-recognition/src/lib.rs"),
    )
    .expect("read audio-analysis-recognition source");

    assert!(source
        .contains("pub use text_transcripts::{TranscriptSegmentContract, TranscriptionContract};"));
    assert!(source.contains("pub imported_segments: Vec<TranscriptSegmentContract>"));
    assert!(source.contains("pub transcript: TranscriptionContract"));
}

#[test]
fn foundational_audio_and_image_cores_stay_runtime_independent() {
    let audio_core = read_manifest("crates/audio/audio-analysis-core/Cargo.toml");
    for forbidden in [
        "video-analysis-ffmpeg",
        "model-runtime",
        "text-transcripts",
        "text-core",
    ] {
        assert!(
            !audio_core.contains(forbidden),
            "audio-analysis-core must not depend on runtime or transcript crate `{forbidden}`"
        );
    }

    let image_core = read_manifest("crates/image/image-analysis-core/Cargo.toml");
    for forbidden in [
        concat!("image-analysis-", "onnx"),
        concat!("video-analysis-", "onnx"),
        "model-runtime",
        "ort",
        "candle",
    ] {
        assert!(
            !image_core.contains(forbidden),
            "image-analysis-core must not depend on model/runtime crate `{forbidden}`"
        );
    }
}

#[test]
fn math_data_and_vector_crates_stay_independent_of_media_runtimes() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut violations = Vec::new();
    for family in ["crates/math", "crates/data", "crates/vector"] {
        collect_manifests(&root.join(family), &mut |manifest| {
            let source = fs::read_to_string(manifest).expect("read manifest");
            for forbidden in [
                "audio-analysis",
                "image-analysis",
                "text-",
                "video-analysis-ffmpeg",
                "video-analysis-ingest",
                "video-analysis-split",
                "model-runtime",
                "ffmpeg-next",
                "ort",
                "candle",
                "tokenizers",
            ] {
                if manifest_declares_dependency(&source, forbidden) {
                    violations.push(format!("{} -> {forbidden}", manifest.display()));
                }
            }
        });
    }

    assert!(
        violations.is_empty(),
        "math/data/vector crates must stay independent of media runtime dependencies: {}",
        violations.join(", ")
    );
}

#[test]
fn native_model_execution_dependencies_are_feature_gated() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut violations = Vec::new();
    collect_manifests(&root.join("crates"), &mut |manifest| {
        let source = fs::read_to_string(manifest).expect("read manifest");
        let mut in_dependency_section = false;
        for dependency in [
            "ort",
            "candle-core",
            "candle-nn",
            "candle-transformers",
            "tokenizers",
        ] {
            for line in source.lines().map(str::trim) {
                if line.starts_with('[') {
                    in_dependency_section = matches!(line, "[dependencies]" | "[dev-dependencies]");
                }
                if line.starts_with(&format!("{dependency} = "))
                    && in_dependency_section
                    && !line.contains("optional = true")
                {
                    violations.push(format!("{}: {}", manifest.display(), line));
                }
            }
        }
    });

    assert!(
        violations.is_empty(),
        "native model execution dependencies must remain optional: {}",
        violations.join(", ")
    );
}

#[test]
fn long_running_model_access_goes_through_model_runtime_jobs() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut violations = Vec::new();
    for dir in ["crates", "src", "prototypes"] {
        collect_rust_sources(&root.join(dir), &mut |path| {
            let relative = path.strip_prefix(root).unwrap_or(path);
            let path_text = relative.to_string_lossy();
            if model_access_scan_is_allowed(&path_text) {
                return;
            }
            let source = fs::read_to_string(path).expect("read Rust source");
            for forbidden in [
                ".download(&spec",
                "ModelBundleStore::download",
                "hf_hub::",
                "hf-hub",
            ] {
                if source.contains(forbidden) {
                    violations.push(format!("{path_text} -> {forbidden}"));
                }
            }
            let model_related =
                path_text.contains("model") || path_text.contains("external_command");
            if model_related
                && (source.contains("Command::new") || source.contains("std::process::Command"))
            {
                violations.push(format!("{path_text} -> direct external command"));
            }
        });
    }

    assert!(
        violations.is_empty(),
        "long-running model access must route through model-runtime::jobs: {}",
        violations.join(", ")
    );
}

#[test]
fn jobs_core_stays_model_agnostic() {
    let manifest = read_manifest("crates/jobs/jobs-core/Cargo.toml");
    for forbidden in [
        "model-runtime",
        "hf-hub",
        "reqwest",
        "ureq",
        "ort",
        "candle",
        "tokenizers",
        "whisper",
        "demucs",
        "audio-analysis",
        "image-analysis",
        "text-",
        "comfyui-",
    ] {
        assert!(
            !manifest.contains(forbidden),
            "jobs-core must remain model/domain agnostic and not depend on `{forbidden}`"
        );
    }
}

#[test]
fn model_runtime_is_the_only_hugging_face_downloader() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut violations = Vec::new();
    for dir in ["crates", "src", "prototypes"] {
        collect_text_sources(&root.join(dir), &mut |path| {
            let relative = path.strip_prefix(root).unwrap_or(path);
            let path_text = relative.to_string_lossy();
            if path_text.starts_with("crates/runtime/model-runtime/")
                || path_text.ends_with("README.md")
            {
                return;
            }
            let source = fs::read_to_string(path).expect("read source");
            if source.contains("hf_hub") || source.contains("hf-hub") {
                violations.push(path_text.to_string());
            }
        });
    }

    assert!(
        violations.is_empty(),
        "Hugging Face download logic must stay in model-runtime: {}",
        violations.join(", ")
    );
}

#[test]
fn default_surfaces_remain_side_effect_free() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut violations = Vec::new();
    collect_rust_sources(&root.join("crates"), &mut |path| {
        if path.file_name().and_then(|name| name.to_str()) != Some("surface.rs") {
            return;
        }
        let relative = path.strip_prefix(root).unwrap_or(path);
        let path_text = relative.to_string_lossy();
        let source = fs::read_to_string(path).expect("read surface source");
        for forbidden in [
            "std::fs::write",
            "fs::write",
            "File::create",
            "Command::new",
            "std::process::Command",
            "reqwest",
            "ureq",
            ".download(",
            "ModelBundleStore::download",
        ] {
            if source.contains(forbidden) && !surface_planning_exception(&path_text, forbidden) {
                violations.push(format!("{path_text} -> {forbidden}"));
            }
        }
    });

    assert!(
        violations.is_empty(),
        "default package surfaces must stay side-effect free: {}",
        violations.join(", ")
    );
}

#[test]
fn runtime_backend_defaults_stay_empty_and_opt_in() {
    for path in [
        "crates/runtime/model-runtime/Cargo.toml",
        "crates/text/text-model-runtime/Cargo.toml",
        "crates/text/text-linguistics/Cargo.toml",
        "crates/text/text-analysis/Cargo.toml",
        "crates/text/text-embeddings/Cargo.toml",
        "crates/runtime/runtime-onnx/Cargo.toml",
        "crates/image/image-analysis-classification/Cargo.toml",
        "crates/image/image-analysis-captioning/Cargo.toml",
        "crates/image/image-analysis-detection/Cargo.toml",
        "crates/image/image-analysis-embeddings/Cargo.toml",
        "crates/video/video-analysis-posture/Cargo.toml",
        "crates/video/video-analysis-recognition/Cargo.toml",
        "crates/audio/audio-analysis-separation/Cargo.toml",
        "crates/video/video-analysis-cli/Cargo.toml",
    ] {
        let manifest_path = Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
        if !manifest_path.exists() {
            continue;
        }
        let manifest = fs::read_to_string(&manifest_path).expect("read manifest");
        assert!(
            manifest.contains("default = []"),
            "`{path}` must keep runtime/backend features opt-in with `default = []`"
        );
        assert!(
            !default_features_include_external_tests(&manifest),
            "`{path}` must not include external-tests in default features"
        );
    }
}

#[test]
fn transcript_dtos_are_owned_by_text_transcripts() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("crates");
    let mut violations = Vec::new();
    collect_rust_sources(&root, &mut |path| {
        let source = fs::read_to_string(path).expect("read Rust source");
        for (line_index, line) in source.lines().enumerate() {
            if !line.contains("pub struct Transcript") {
                continue;
            }
            let path_text = path.to_string_lossy();
            let allowed = path_text.contains("crates/text/text-transcripts/")
                || line.contains("TranscriptStatsExtractor")
                || is_audio_transcription_surface_type(&path_text, line);
            if !allowed {
                violations.push(format!("{}:{}", path.display(), line_index + 1));
            }
        }
    });

    assert!(
        violations.is_empty(),
        "new public Transcript* DTOs must live in text-transcripts or be allowlisted compatibility shims: {}",
        violations.join(", ")
    );
}

#[test]
fn audio_crates_do_not_define_transcript_dtos() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/audio");
    let mut violations = Vec::new();
    collect_rust_sources(&root, &mut |path| {
        let source = fs::read_to_string(path).expect("read Rust source");
        for (line_index, line) in source.lines().enumerate() {
            let path_text = path.to_string_lossy();
            if (line.contains("pub struct Transcript") || line.contains("pub enum Transcript"))
                && !is_audio_transcription_surface_type(&path_text, line)
            {
                violations.push(format!("{}:{}", path.display(), line_index + 1));
            }
        }
    });

    assert!(
        violations.is_empty(),
        "audio crates must consume/re-export text-transcripts contracts instead of defining transcript DTOs: {}",
        violations.join(", ")
    );
}

#[test]
fn runtime_surface_dtos_are_owned_by_runtime_core() {
    for type_name in [
        "RuntimeCapabilities",
        "PackageSurface",
        "SurfaceOperation",
        "SurfaceRequest",
        "SurfaceResponse",
        "SurfaceExecutionPlan",
        "SurfaceArtifactExpectation",
        "SurfaceExecutionMode",
        "SurfaceSideEffect",
    ] {
        assert_public_contract_owned_by(type_name, "crates/runtime/runtime-core/");
    }
}

#[test]
fn surface_operation_has_no_workflow_graph_metadata() {
    let source = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/runtime/runtime-core/src/lib.rs"),
    )
    .expect("read runtime-core source");
    let body = public_struct_body(&source, "SurfaceOperation");

    for field in public_field_names(body) {
        let lower = field.to_ascii_lowercase();
        for forbidden in ["node", "edge", "graph", "socket", "layout", "workflow"] {
            assert!(
                !lower.contains(forbidden),
                "SurfaceOperation field `{field}` must not introduce workflow graph metadata"
            );
        }
        assert!(
            lower != "port"
                && !lower.ends_with("_port")
                && !lower.contains("port_")
                && !lower.ends_with("_ports")
                && !lower.contains("ports_"),
            "SurfaceOperation field `{field}` must not introduce port metadata"
        );
    }

    for forbidden_type in ["NodePort", "WorkflowNode", "WorkflowEdge"] {
        assert!(
            !source.contains(forbidden_type),
            "runtime-core must not add workflow graph DTO `{forbidden_type}`"
        );
    }
}

#[test]
fn foundation_contract_owner_rules_remain_enforced() {
    for (type_name, owner) in [
        ("JobSpec", "crates/jobs/jobs-core/"),
        ("JobArtifact", "crates/jobs/jobs-core/"),
        ("ArtifactRef", "crates/jobs/jobs-core/"),
        ("ModelSpec", "crates/runtime/model-runtime/"),
        ("ModelTask", "crates/runtime/model-runtime/"),
        ("ModelBundle", "crates/runtime/model-runtime/"),
        ("ModelBundleManifest", "crates/runtime/model-runtime/"),
        ("ModelRuntimeSelection", "crates/runtime/model-runtime/"),
        ("Timebase", "crates/video/video-analysis-core/"),
        ("Timestamp", "crates/video/video-analysis-core/"),
        ("VideoFrame", "crates/video/video-analysis-core/"),
        ("OwnedVideoFrame", "crates/video/video-analysis-core/"),
        ("BoundingBox", "crates/video/video-analysis-core/"),
        ("Observation", "crates/video/video-analysis-core/"),
        ("ImageView", "crates/image/image-analysis-core/"),
        ("OwnedImage", "crates/image/image-analysis-core/"),
        ("AudioFrames", "crates/audio/audio-analysis-core/"),
        ("AudioFeaturePoint", "crates/audio/audio-analysis-core/"),
        ("AudioFeatureSeries", "crates/audio/audio-analysis-core/"),
        ("TextDocumentContract", "crates/text/text-core/"),
        ("TextSegmentContract", "crates/text/text-core/"),
        ("TranscriptSegmentContract", "crates/text/text-transcripts/"),
        ("TranscriptionContract", "crates/text/text-transcripts/"),
        ("TensorShape", "crates/data/tensor-data/"),
        ("F32Tensor", "crates/data/tensor-data/"),
        ("DenseVector", "crates/vector/vector-analysis-core/"),
        ("SparseVector", "crates/math/math-sparse-data/"),
        ("DensePoint", "crates/data/dense-data/"),
        ("DenseDataset", "crates/data/dense-data/"),
        ("NumberSummary", "crates/data/numbers-core/"),
        ("NumberRange", "crates/data/numbers-core/"),
        ("Point2f", "crates/math/math-geometry-2d/"),
        ("RectU32", "crates/math/math-geometry-2d/"),
        ("NormalizedPoint2", "crates/math/math-geometry-2d/"),
        ("SampleRate", "crates/math/math-signal-core/"),
        ("SignalLevels", "crates/math/math-signal-core/"),
    ] {
        assert_public_contract_owned_by(type_name, owner);
    }
}

#[test]
fn foundation_adapters_delegate_to_library_owned_surfaces() {
    for case in [
        FoundationAdapterCase {
            crate_name: "jobs-core",
            import_name: "jobs_core",
            cli: "crates/jobs/jobs-core-cli/src/lib.rs",
            server: "crates/jobs/jobs-core-server/src/lib.rs",
            wasm: "crates/bindings/jobs-core-wasm/src/lib.rs",
        },
        FoundationAdapterCase {
            crate_name: "model-runtime",
            import_name: "model_runtime",
            cli: "crates/runtime/model-runtime-cli/src/lib.rs",
            server: "crates/runtime/model-runtime-server/src/lib.rs",
            wasm: "crates/bindings/model-runtime-wasm/src/lib.rs",
        },
        FoundationAdapterCase {
            crate_name: "video-analysis-core",
            import_name: "video_analysis_core",
            cli: "crates/video/video-analysis-core-cli/src/lib.rs",
            server: "crates/video/video-analysis-core-server/src/lib.rs",
            wasm: "crates/bindings/video-analysis-core-wasm/src/lib.rs",
        },
        FoundationAdapterCase {
            crate_name: "image-analysis-core",
            import_name: "image_analysis_core",
            cli: "crates/image/image-analysis-core-cli/src/lib.rs",
            server: "crates/image/image-analysis-core-server/src/lib.rs",
            wasm: "crates/bindings/image-analysis-core-wasm/src/lib.rs",
        },
        FoundationAdapterCase {
            crate_name: "audio-analysis-core",
            import_name: "audio_analysis_core",
            cli: "crates/audio/audio-analysis-core-cli/src/lib.rs",
            server: "crates/audio/audio-analysis-core-server/src/lib.rs",
            wasm: "crates/bindings/audio-analysis-core-wasm/src/lib.rs",
        },
        FoundationAdapterCase {
            crate_name: "text-core",
            import_name: "text_core",
            cli: "crates/text/text-core-cli/src/lib.rs",
            server: "crates/text/text-core-server/src/lib.rs",
            wasm: "crates/bindings/text-core-wasm/src/lib.rs",
        },
        FoundationAdapterCase {
            crate_name: "text-transcripts",
            import_name: "text_transcripts",
            cli: "crates/text/text-transcripts-cli/src/lib.rs",
            server: "crates/text/text-transcripts-server/src/lib.rs",
            wasm: "crates/bindings/text-transcripts-wasm/src/lib.rs",
        },
        FoundationAdapterCase {
            crate_name: "tensor-data",
            import_name: "tensor_data",
            cli: "crates/data/tensor-data-cli/src/lib.rs",
            server: "crates/data/tensor-data-server/src/lib.rs",
            wasm: "crates/bindings/tensor-data-wasm/src/lib.rs",
        },
        FoundationAdapterCase {
            crate_name: "vector-analysis-core",
            import_name: "vector_analysis_core",
            cli: "crates/vector/vector-analysis-core-cli/src/lib.rs",
            server: "crates/vector/vector-analysis-core-server/src/lib.rs",
            wasm: "crates/bindings/vector-analysis-core-wasm/src/lib.rs",
        },
        FoundationAdapterCase {
            crate_name: "math-sparse-data",
            import_name: "math_sparse_data",
            cli: "crates/math/math-sparse-data-cli/src/lib.rs",
            server: "crates/math/math-sparse-data-server/src/lib.rs",
            wasm: "crates/bindings/math-sparse-data-wasm/src/lib.rs",
        },
        FoundationAdapterCase {
            crate_name: "dense-data",
            import_name: "dense_data",
            cli: "crates/data/dense-data-cli/src/lib.rs",
            server: "crates/data/dense-data-server/src/lib.rs",
            wasm: "crates/bindings/dense-data-wasm/src/lib.rs",
        },
        FoundationAdapterCase {
            crate_name: "numbers-core",
            import_name: "numbers_core",
            cli: "crates/data/numbers-core-cli/src/lib.rs",
            server: "crates/data/numbers-core-server/src/lib.rs",
            wasm: "crates/bindings/numbers-core-wasm/src/lib.rs",
        },
        FoundationAdapterCase {
            crate_name: "math-geometry-2d",
            import_name: "math_geometry_2d",
            cli: "crates/math/math-geometry-2d-cli/src/lib.rs",
            server: "crates/math/math-geometry-2d-server/src/lib.rs",
            wasm: "crates/bindings/math-geometry-2d-wasm/src/lib.rs",
        },
        FoundationAdapterCase {
            crate_name: "math-signal-core",
            import_name: "math_signal_core",
            cli: "crates/math/math-signal-core-cli/src/lib.rs",
            server: "crates/math/math-signal-core-server/src/lib.rs",
            wasm: "crates/bindings/math-signal-core-wasm/src/lib.rs",
        },
    ] {
        assert_cli_adapter_delegates(case);
        assert_server_adapter_delegates(case);
        assert_wasm_adapter_delegates(case);
    }
}

#[test]
fn release_checklist_documents_contract_readiness_gate() {
    let checklist = read_source("docs/RELEASE_CHECKLIST.md");

    assert!(
        checklist.contains("moritzbrantner-runtime-core"),
        "release checklist must name runtime-core as the runtime DTO owner"
    );
    assert!(
        checklist.contains("video-analysis-core::runtime")
            && checklist.contains("re-exported")
            && checklist.contains("compatibility"),
        "release checklist must describe video-analysis-core::runtime as a compatibility re-export"
    );
    assert!(
        checklist.contains("## Contract-Readiness Gate"),
        "release checklist must include the named contract-readiness gate"
    );
    for command in [
        "cargo test --test contract_ownership --test dependency_layers --test foundation_surface_audit --test package_structure --test package_interop_pipeline",
        "bun run snapshot:check",
        "bun run hygiene:generated",
        "cargo fmt --check",
        "git diff --check",
    ] {
        assert!(
            checklist.contains(command),
            "release checklist contract-readiness gate missing `{command}`"
        );
    }
    assert!(
        !checklist.contains("Runtime surface DTOs live in `video-analysis-core::runtime`."),
        "release checklist must not retain stale video-analysis-core runtime DTO ownership wording"
    );
}

#[derive(Clone, Copy)]
struct FoundationAdapterCase {
    crate_name: &'static str,
    import_name: &'static str,
    cli: &'static str,
    server: &'static str,
    wasm: &'static str,
}

fn assert_cli_adapter_delegates(case: FoundationAdapterCase) {
    let source = read_source(case.cli);
    assert!(
        source.contains(&format!("{}::surface::package_surface()", case.import_name)),
        "{} CLI adapter must delegate package_surface to the library crate",
        case.crate_name
    );
    assert!(
        source.contains(&format!(
            "{}::surface::run_surface_operation",
            case.import_name
        )),
        "{} CLI adapter must delegate execution to the library crate",
        case.crate_name
    );
}

fn assert_server_adapter_delegates(case: FoundationAdapterCase) {
    let source = read_source(case.server);
    assert!(
        source.contains(&format!("{}::surface::package_surface()", case.import_name)),
        "{} server adapter must delegate package_surface to the library crate",
        case.crate_name
    );
    assert!(
        source.contains(&format!(
            "{}::surface::run_surface_operation",
            case.import_name
        )),
        "{} server adapter must delegate execution to the library crate",
        case.crate_name
    );
}

fn assert_wasm_adapter_delegates(case: FoundationAdapterCase) {
    let source = read_source(case.wasm);
    assert!(
        source.contains(&format!("{}::surface::package_surface()", case.import_name)),
        "{} WASM adapter must delegate package_surface to the library crate",
        case.crate_name
    );
    assert!(
        source.contains(&format!(
            "{}::surface::run_surface_operation",
            case.import_name
        )),
        "{} WASM adapter must delegate execution to the library crate",
        case.crate_name
    );
}

fn assert_public_contract_owned_by(type_name: &str, owner_prefix: &str) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut owners = Vec::new();
    collect_rust_sources(&root.join("crates"), &mut |path| {
        let source = fs::read_to_string(path).expect("read Rust source");
        if source
            .lines()
            .any(|line| public_type_declaration_name(line) == Some(type_name))
        {
            owners.push(
                path.strip_prefix(root)
                    .unwrap_or(path)
                    .to_string_lossy()
                    .to_string(),
            );
        }
    });

    assert!(
        !owners.is_empty(),
        "expected to find public contract `{type_name}`"
    );
    let violations = owners
        .iter()
        .filter(|path| !path.starts_with(owner_prefix))
        .cloned()
        .collect::<Vec<_>>();
    assert!(
        violations.is_empty(),
        "`{type_name}` must be owned by `{owner_prefix}`, found parallel definitions in: {}",
        violations.join(", ")
    );
}

fn public_type_declaration_name(line: &str) -> Option<&str> {
    let line = line.trim_start();
    let declaration = line
        .strip_prefix("pub struct ")
        .or_else(|| line.strip_prefix("pub enum "))?;
    declaration
        .split(|character: char| {
            character.is_whitespace() || matches!(character, '{' | '(' | ';' | '<')
        })
        .next()
}

fn read_source(path: &str) -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(path))
        .unwrap_or_else(|err| panic!("read source `{path}`: {err}"))
}

fn public_struct_body<'a>(source: &'a str, name: &str) -> &'a str {
    let declaration = format!("pub struct {name} {{");
    let start = source
        .find(&declaration)
        .unwrap_or_else(|| panic!("find struct declaration `{declaration}`"));
    let body_start = start + declaration.len();
    let end = source[body_start..]
        .find("\n}")
        .unwrap_or_else(|| panic!("find end of struct `{name}`"));
    &source[body_start..body_start + end]
}

fn public_field_names(body: &str) -> impl Iterator<Item = &str> {
    body.lines().filter_map(|line| {
        line.trim()
            .strip_prefix("pub ")
            .and_then(|field| field.split_once(':'))
            .map(|(name, _)| name.trim())
    })
}

fn is_audio_transcription_surface_type(path: &str, line: &str) -> bool {
    if path.contains("crates/audio/audio-analysis-recognition/src/transcription.rs") {
        return line.contains("pub enum TranscriptionInput")
            || line.contains("pub struct TranscriptionRuntimeSelection")
            || line.contains("pub struct TranscriptionRequest")
            || line.contains("pub struct TranscriptionResponse")
            || line.contains("pub enum TranscriptionProviderKind")
            || line.contains("pub struct TranscriptionBackendPlan")
            || line.contains("pub struct WhisperCppTranscriptionPlan");
    }

    path.contains("crates/audio/audio-analysis-transcription/src/lib.rs")
        && (line.contains("pub struct TranscriptionPipelineRequest")
            || line.contains("pub enum TranscriptionSource")
            || line.contains("pub enum TranscriptionProviderSelection")
            || line.contains("pub struct TranscriptionOutputOptions")
            || line.contains("pub struct TranscriptionArtifact")
            || line.contains("pub struct TranscriptionPipelineResponse")
            || line.contains("pub struct TranscriptionProviderPlan"))
}

fn collect_text_sources(dir: &Path, visit: &mut impl FnMut(&Path)) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path: PathBuf = entry.path();
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if file_name == "target"
            || file_name == ".cargo-target"
            || file_name == "vendor"
            || file_name == "dist"
            || file_name == "node_modules"
        {
            continue;
        }
        if path.is_dir() {
            collect_text_sources(&path, visit);
        } else if matches!(
            path.extension().and_then(|extension| extension.to_str()),
            Some("rs" | "toml" | "md" | "json" | "ts" | "tsx")
        ) {
            visit(&path);
        }
    }
}

fn collect_rust_sources(dir: &Path, visit: &mut impl FnMut(&Path)) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path: PathBuf = entry.path();
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if file_name == "target" || file_name == ".cargo-target" || file_name == "vendor" {
            continue;
        }
        if path.is_dir() {
            collect_rust_sources(&path, visit);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            visit(&path);
        }
    }
}

fn collect_manifests(dir: &Path, visit: &mut impl FnMut(&Path)) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path: PathBuf = entry.path();
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if file_name == "target"
            || file_name == ".cargo-target"
            || file_name == "vendor"
            || file_name == "node_modules"
            || file_name == "dist"
        {
            continue;
        }
        if path.is_dir() {
            collect_manifests(&path, visit);
        } else if file_name == "Cargo.toml" {
            visit(&path);
        }
    }
}

fn model_access_scan_is_allowed(path: &str) -> bool {
    path.starts_with("crates/runtime/model-runtime/")
        || path.starts_with("crates/image/image-analysis-classification/")
        || path.starts_with("crates/image/image-analysis-captioning/")
        || path.starts_with("crates/image/image-analysis-detection/")
        || path.starts_with("crates/image/image-analysis-embeddings/")
        || path.starts_with("crates/video/video-analysis-posture/")
        || path.starts_with("crates/video/video-analysis-recognition/")
        || path.starts_with("crates/text/text-model-runtime/")
        || path.starts_with("crates/video/video-analysis-recognition/src/external_command.rs")
        || path.contains("/tests/")
        || path.starts_with("prototypes/rust/video-analysis-use-cases/tests/")
}

fn surface_planning_exception(path: &str, forbidden: &str) -> bool {
    path.starts_with("crates/runtime/model-runtime/") && forbidden == ".download("
}

fn default_features_include_external_tests(manifest: &str) -> bool {
    manifest
        .lines()
        .map(str::trim)
        .any(|line| line.starts_with("default = ") && line.contains("external-tests"))
}
