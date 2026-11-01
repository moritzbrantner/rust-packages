use std::fs;
use std::path::{Path, PathBuf};

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
