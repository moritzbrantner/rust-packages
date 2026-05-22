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
        linguistics_manifest.contains("model-runtime = { workspace = true, optional = true }"),
        "text-linguistics must use model-runtime for optional model bundle support"
    );

    let runtime_manifest = read_manifest("crates/text/text-model-runtime/Cargo.toml");
    assert!(
        runtime_manifest.contains("model-runtime = { workspace = true, optional = true }"),
        "text-model-runtime must use model-runtime for optional model bundle support"
    );
}

#[test]
fn generic_model_runtime_stays_domain_independent() {
    let manifest = read_manifest("crates/runtime/model-runtime/Cargo.toml");
    for forbidden in [
        "video-analysis-core",
        "audio-analysis",
        "image-analysis",
        "text-",
        "comfyui-",
    ] {
        assert!(
            !manifest.contains(forbidden),
            "model-runtime must stay domain-independent and not depend on `{forbidden}`"
        );
    }
    assert!(
        manifest.contains("jobs-core = { workspace = true, optional = true }"),
        "model-runtime job helpers must keep jobs-core behind the jobs feature"
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
        source.contains("pub use audio_analysis_tasks as audio_tasks;"),
        "root facade should expose aggregate audio task APIs as audio_tasks"
    );
    assert!(
        source.contains("pub use image_analysis_tasks as image_tasks;"),
        "root facade should expose aggregate image task APIs as image_tasks"
    );
}

#[test]
fn video_analysis_models_is_marked_compatibility_only() {
    let readme = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/video/video-analysis-models/README.md"),
    )
    .expect("read video-analysis-models README");
    assert!(
        readme.contains("Deprecated compatibility crate"),
        "video-analysis-models must be documented as a temporary compatibility crate"
    );
}

#[test]
fn image_analysis_models_is_marked_compatibility_only() {
    let readme = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/image/image-analysis-models/README.md"),
    )
    .expect("read image-analysis-models README");
    assert!(
        readme.contains("Deprecated compatibility crate"),
        "image-analysis-models must be documented as a temporary compatibility crate"
    );
}

#[test]
fn image_onnx_uses_task_and_capability_contracts_directly() {
    let manifest = read_manifest("crates/image/image-analysis-onnx/Cargo.toml");
    assert!(
        manifest.contains("image-analysis-tasks.workspace = true"),
        "image-analysis-onnx must consume aggregate image task contracts directly"
    );
    assert!(
        !manifest.contains("image-analysis-models.workspace = true"),
        "image-analysis-onnx must not depend on the compatibility image-analysis-models crate"
    );
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
    let tasks_source = read_manifest("crates/audio/audio-analysis-tasks/src/lib.rs");
    assert!(
        !tasks_source.contains("Command::new"),
        "audio-analysis-tasks must not perform external command execution directly"
    );

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
