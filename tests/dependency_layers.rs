use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
fn active_metadata_has_no_retired_runtime_crates() {
    let graph = MetadataGraph::load();
    for retired in [
        "moritzbrantner-runtime-artifacts",
        "moritzbrantner-runtime-artifacts-cli",
        "moritzbrantner-runtime-artifacts-server",
        "moritzbrantner-runtime-artifacts-wasm",
        "moritzbrantner-runtime-jobs",
        "moritzbrantner-runtime-jobs-cli",
        "moritzbrantner-runtime-jobs-server",
        "moritzbrantner-runtime-jobs-wasm",
        concat!("moritzbrantner-image-analysis-", "onnx"),
        concat!("moritzbrantner-image-analysis-", "onnx-cli"),
        concat!("moritzbrantner-image-analysis-", "onnx-server"),
        concat!("moritzbrantner-image-analysis-", "onnx-wasm"),
        concat!("moenarch-video-analysis-", "onnx"),
        concat!("moenarch-video-analysis-", "onnx-cli"),
        concat!("moenarch-video-analysis-", "onnx-server"),
        concat!("moenarch-video-analysis-", "onnx-wasm"),
    ] {
        assert!(
            !graph.packages.contains_key(retired),
            "retired crate `{retired}` must not appear in active Cargo metadata"
        );
    }
}

#[test]
fn transport_wrappers_depend_only_on_wrapped_library_and_runtime_core() {
    let graph = MetadataGraph::load();
    let mut failures = Vec::new();

    for package in graph.packages.values() {
        let Some(kind) = transport_kind(&package.name) else {
            continue;
        };
        let base = package.name.trim_end_matches(kind);
        if !graph.packages.contains_key(base) {
            continue;
        }
        let mut allowed = BTreeSet::from([base.to_string(), "moenarch-runtime-core".to_string()]);
        if package.name == "moritzbrantner-geo-io-osm-cli" {
            allowed.insert("moritzbrantner-geo-io-geojson".to_string());
        }
        if matches!(
            package.name.as_str(),
            "moenarch-text-analysis-server"
                | "moenarch-text-embeddings-server"
                | "moenarch-text-linguistics-server"
        ) {
            allowed.insert("moenarch-text-model-runtime".to_string());
        }
        if package.name == "moenarch-video-analysis-cli" {
            continue;
        }
        let extra = package
            .internal_dependencies
            .difference(&allowed)
            .cloned()
            .collect::<Vec<_>>();
        if !extra.is_empty() {
            failures.push(format!(
                "{} depends on extra workspace crates: {}",
                package.name,
                extra.join(", ")
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "transport wrappers should stay thin: {}",
        failures.join("; ")
    );
}

#[test]
fn foundation_crates_do_not_depend_on_domain_crates() {
    let graph = MetadataGraph::load();
    let foundation = BTreeSet::from([
        "moenarch-media-core",
        "moenarch-runtime-core",
        "moenarch-runtime-onnx",
        "moenarch-jobs-core",
        "moenarch-numbers-core",
        "moenarch-tensor-data",
        "moenarch-vector-analysis-core",
        "moenarch-math-sparse-data",
    ]);
    let domain_prefixes = [
        "moritzbrantner-audio-",
        "moritzbrantner-image-",
        "moritzbrantner-text-",
        "moritzbrantner-three-d-",
        "moritzbrantner-video-",
        "moritzbrantner-comfyui-",
    ];
    let mut failures = Vec::new();

    for name in foundation {
        let Some(package) = graph.packages.get(name) else {
            continue;
        };
        for dependency in &package.internal_dependencies {
            if dependency == "moenarch-video-analysis-core" {
                continue;
            }
            if domain_prefixes
                .iter()
                .any(|prefix| dependency.starts_with(prefix))
            {
                failures.push(format!("{name} depends on domain crate {dependency}"));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "foundation crates must not depend on domain crates: {}",
        failures.join("; ")
    );
}

#[test]
fn audio_contract_consumers_do_not_depend_on_visual_analysis_core() {
    let graph = MetadataGraph::load();

    for package_name in [
        "moenarch-audio-analysis-core",
        "moenarch-audio-analysis-fourier",
        "moenarch-audio-analysis-io",
        "moenarch-audio-analysis-pitch",
        "moenarch-audio-analysis-processing",
        "moenarch-audio-analysis-recognition",
        "moenarch-audio-analysis-rhythm",
        "moenarch-audio-analysis-speakers",
        "moenarch-audio-analysis-synthesis",
        "moenarch-audio-analysis-test-support",
        "moenarch-audio-generation-midi",
        "moenarch-audio-generation-tts",
    ] {
        let package = graph
            .packages
            .get(package_name)
            .unwrap_or_else(|| panic!("missing workspace package `{package_name}`"));
        assert!(
            !package
                .internal_dependencies
                .contains("moenarch-video-analysis-core"),
            "`{package_name}` must consume canonical audio contracts without depending on visual analysis"
        );
    }
}

#[derive(Debug)]
struct Package {
    name: String,
    internal_dependencies: BTreeSet<String>,
}

#[derive(Debug)]
struct MetadataGraph {
    packages: BTreeMap<String, Package>,
}

impl MetadataGraph {
    fn load() -> Self {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let output = Command::new("cargo")
            .args(["metadata", "--format-version", "1", "--no-deps"])
            .current_dir(root)
            .output()
            .expect("run cargo metadata");
        assert!(
            output.status.success(),
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let metadata: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("parse cargo metadata");
        let members = metadata["workspace_members"]
            .as_array()
            .expect("workspace members")
            .iter()
            .filter_map(|member| member.as_str())
            .collect::<BTreeSet<_>>();
        let workspace_packages = metadata["packages"]
            .as_array()
            .expect("packages")
            .iter()
            .filter(|package| {
                package["id"]
                    .as_str()
                    .is_some_and(|id| members.contains(id))
            })
            .collect::<Vec<_>>();
        let workspace_names = workspace_packages
            .iter()
            .filter_map(|package| package["name"].as_str())
            .collect::<BTreeSet<_>>();
        let packages = workspace_packages
            .into_iter()
            .map(|package| {
                let name = package["name"].as_str().expect("package name").to_string();
                let manifest = PathBuf::from(package["manifest_path"].as_str().expect("manifest"));
                let internal_dependencies = package["dependencies"]
                    .as_array()
                    .expect("dependencies")
                    .iter()
                    .filter_map(|dependency| dependency["name"].as_str())
                    .filter(|dependency| workspace_names.contains(dependency))
                    .filter(|dependency| !is_dev_only_dependency(&manifest, dependency))
                    .map(str::to_string)
                    .collect::<BTreeSet<_>>();
                (
                    name.clone(),
                    Package {
                        name,
                        internal_dependencies,
                    },
                )
            })
            .collect();
        Self { packages }
    }
}

fn transport_kind(package_name: &str) -> Option<&'static str> {
    if package_name.ends_with("-cli") {
        Some("-cli")
    } else if package_name.ends_with("-server") {
        Some("-server")
    } else if package_name.ends_with("-wasm") {
        Some("-wasm")
    } else {
        None
    }
}

fn is_dev_only_dependency(manifest: &Path, dependency_name: &str) -> bool {
    let Ok(text) = std::fs::read_to_string(manifest) else {
        return false;
    };
    let key = dependency_name.trim_start_matches("moritzbrantner-");
    let mut section = "";
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            section = trimmed;
            continue;
        }
        if section == "[dev-dependencies]"
            && (trimmed.starts_with(&format!("{key} "))
                || trimmed.starts_with(&format!("{key}="))
                || trimmed.starts_with(&format!("{key}.")))
        {
            return true;
        }
    }
    false
}
