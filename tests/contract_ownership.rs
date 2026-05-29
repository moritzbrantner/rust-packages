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
                "ort",
                "ndarray",
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
                "ort",
                "ndarray",
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
        manifest.contains("video-analysis-core.workspace = true"),
        "model-runtime surfaces should use video-analysis-core runtime DTOs"
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
fn image_onnx_uses_concrete_capability_contracts_directly() {
    let manifest = read_manifest("crates/image/image-analysis-onnx/Cargo.toml");
    assert!(
        manifest.contains("image-analysis-classification.workspace = true"),
        "image-analysis-onnx must consume image classification contracts directly"
    );
    assert!(
        manifest.contains("image-analysis-embeddings.workspace = true"),
        "image-analysis-onnx must consume image embedding contracts directly"
    );
    assert!(
        !manifest.contains("image-analysis-tasks.workspace = true"),
        "image-analysis-onnx must not depend on aggregate image-analysis-tasks"
    );
    assert!(
        !manifest.contains("image-analysis-models.workspace = true"),
        "image-analysis-onnx must not depend on the compatibility image-analysis-models crate"
    );
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
        "image-analysis-onnx",
        "video-analysis-onnx",
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
                "video-analysis-onnx",
                "video-analysis-split",
                "model-runtime",
                "ffmpeg-next",
                "ort",
                "candle",
                "tokenizers",
            ] {
                if source.contains(forbidden) {
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
        "video-analysis-onnx",
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
        "crates/image/image-analysis-onnx/Cargo.toml",
        "crates/video/video-analysis-onnx/Cargo.toml",
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
                || line.contains("TranscriptHeuristicAnalyzer")
                || line.contains("TranscriptStatsExtractor");
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
            if line.contains("pub struct Transcript") || line.contains("pub enum Transcript") {
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
        || path.starts_with("crates/image/image-analysis-onnx/")
        || path.starts_with("crates/video/video-analysis-onnx/")
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
