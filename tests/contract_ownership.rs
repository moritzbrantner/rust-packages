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
        "pub use audio_analysis_models as audio_models;",
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

fn read_manifest(path: &str) -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(path))
        .unwrap_or_else(|err| panic!("read manifest `{path}`: {err}"))
}

#[test]
fn audio_asr_contract_uses_text_transcript_contracts() {
    let source = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/audio/audio-analysis-models/src/lib.rs"),
    )
    .expect("read audio-analysis-models source");

    assert!(source
        .contains("pub use text_transcripts::{TranscriptSegmentContract, TranscriptionContract};"));
    assert!(source.contains("pub imported_segments: Vec<TranscriptSegmentContract>"));
    assert!(source.contains("pub transcript: TranscriptionContract"));
    assert!(source
        .contains("#[deprecated(note = \"use text_transcripts::TranscriptSegmentContract\")]"));
}

#[test]
fn transcript_dtos_are_owned_by_text_transcripts_with_explicit_audio_compatibility_shim() {
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
                || (path_text.contains("crates/audio/audio-analysis-models/src/lib.rs")
                    && line.contains("TranscriptSegmentPrediction"))
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
