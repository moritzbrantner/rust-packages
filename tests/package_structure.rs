use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn library_manifests_do_not_embed_generic_runtime_surfaces() {
    for manifest in workspace_manifests(Path::new(env!("CARGO_MANIFEST_DIR"))) {
        let manifest_text = fs::read_to_string(&manifest).unwrap();
        assert!(
            !manifest_text.contains("package-surfaces"),
            "{} still points at shared package-surface sources",
            manifest.display()
        );
        assert!(
            !manifest_text.contains("# Shared package surface targets."),
            "{} still declares generic runtime surface binaries",
            manifest.display()
        );
    }
}

#[test]
fn readme_local_markdown_links_resolve() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let readme = fs::read_to_string(root.join("README.md")).unwrap();
    let missing = local_markdown_link_targets(&readme)
        .into_iter()
        .filter(|target| !root.join(target).exists())
        .collect::<Vec<_>>();

    assert!(
        missing.is_empty(),
        "README local links must resolve: {}",
        missing.join(", ")
    );
}

#[test]
fn rust_library_crates_have_unit_tests() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let missing = workspace_manifests(root)
        .into_iter()
        .filter_map(|manifest| {
            let crate_dir = manifest.parent()?;
            let src_dir = crate_dir.join("src");
            let lib_rs = src_dir.join("lib.rs");
            if !lib_rs.is_file() {
                return None;
            }
            (!has_rust_test_marker(&src_dir)).then(|| {
                crate_dir
                    .strip_prefix(root)
                    .unwrap_or(crate_dir)
                    .display()
                    .to_string()
            })
        })
        .collect::<Vec<_>>();

    assert!(
        missing.is_empty(),
        "Rust library crates need unit tests in src/: {}",
        missing.join(", ")
    );
}

#[test]
fn cli_packages_have_integration_tests() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let missing = workspace_manifests(root)
        .into_iter()
        .filter_map(|manifest| {
            let manifest_text = fs::read_to_string(&manifest).ok()?;
            let crate_dir = manifest.parent()?;
            let has_cli_surface =
                manifest_text.contains("[[bin]]") || crate_dir.join("src/main.rs").is_file();
            if !has_cli_surface {
                return None;
            }
            (!has_rust_integration_test(crate_dir)).then(|| {
                crate_dir
                    .strip_prefix(root)
                    .unwrap_or(crate_dir)
                    .display()
                    .to_string()
            })
        })
        .collect::<Vec<_>>();

    assert!(
        missing.is_empty(),
        "CLI packages need integration tests under tests/: {}",
        missing.join(", ")
    );
}

#[test]
fn api_packages_have_http_integration_tests() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let web_package = root.join("prototypes/web/video-analysis-web");

    assert!(
        web_package.join("src/api.integration.test.ts").is_file(),
        "@video-analysis/web needs API integration tests"
    );
}

#[test]
fn frontend_libraries_and_ui_packages_have_expected_test_layers() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));

    assert!(
        has_frontend_unit_test(&root.join("packages/text-core-wasm")),
        "@mb-rust/text-core-wasm needs frontend package unit tests"
    );
    assert!(
        has_frontend_e2e_test(&root.join("packages/video-analysis-ui")),
        "@video-analysis/ui needs browser e2e tests"
    );
    assert!(
        has_frontend_e2e_test(&root.join("prototypes/web/video-analysis-web")),
        "@video-analysis/web needs browser e2e tests"
    );
}

#[test]
fn library_crates_have_complete_runtime_surfaces() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut missing = Vec::new();

    for manifest in library_manifests(root) {
        let crate_dir = manifest.parent().expect("crate dir");
        let name = package_name(&manifest);
        let surface_name = surface_package_name(&name);
        let parent = crate_dir.parent().expect("crate parent");

        let cli_dir = parent.join(format!("{surface_name}-cli"));
        let server_dir = parent.join(format!("{surface_name}-server"));
        let rust_wasm_dir = root
            .join("crates")
            .join("bindings")
            .join(format!("{surface_name}-wasm"));
        let package_wasm_dir = root.join("packages").join(format!("{surface_name}-wasm"));
        let app_dir = root.join("packages").join(format!("{surface_name}-app"));

        if !cli_dir.join("Cargo.toml").is_file() {
            missing.push(format!("{surface_name}: missing {surface_name}-cli"));
        } else {
            let cargo = fs::read_to_string(cli_dir.join("Cargo.toml")).unwrap();
            if !has_exact_base_dependency(&cargo, &name, surface_name) {
                missing.push(format!(
                    "{surface_name}: cli does not depend on exact base crate"
                ));
            }
        }

        if !server_dir.join("Cargo.toml").is_file() {
            missing.push(format!("{surface_name}: missing {surface_name}-server"));
        } else {
            let cargo = fs::read_to_string(server_dir.join("Cargo.toml")).unwrap();
            if !has_exact_base_dependency(&cargo, &name, surface_name) {
                missing.push(format!(
                    "{surface_name}: server does not depend on exact base crate"
                ));
            }
            let server_lib = fs::read_to_string(server_dir.join("src/lib.rs")).unwrap();
            if server_lib.contains("This generic adapter is ready for crate-specific operations") {
                missing.push(format!(
                    "{surface_name}: server still contains generic placeholder"
                ));
            }
        }

        if !rust_wasm_dir.join("Cargo.toml").is_file() {
            missing.push(format!("{surface_name}: missing Rust wasm crate"));
        }
        if !package_wasm_dir.join("package.json").is_file() {
            missing.push(format!("{surface_name}: missing Bun wasm package"));
        }
        if !app_dir.join("package.json").is_file() {
            missing.push(format!("{surface_name}: missing Vite app package"));
        } else {
            let package_json = fs::read_to_string(app_dir.join("package.json")).unwrap();
            if !package_json.contains(&format!("@mb-rust/{surface_name}-wasm")) {
                missing.push(format!(
                    "{surface_name}: app does not depend on matching wasm package"
                ));
            }
            let api_ts = fs::read_to_string(app_dir.join("src/api.ts")).unwrap();
            if !api_ts.contains("runWasmOperation") || !api_ts.contains("serverBaseUrl") {
                missing.push(format!(
                    "{surface_name}: app does not expose both wasm and server runtimes"
                ));
            }
        }
    }

    assert!(
        missing.is_empty(),
        "Library crates need complete runtime surfaces: {}",
        missing.join(", ")
    );
}

#[test]
fn representative_adapters_delegate_to_library_owned_surfaces() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for crate_name in [
        "audio-analysis-processing",
        "image-analysis-processing",
        "video-analysis-editing",
    ] {
        let domain = if crate_name.starts_with("audio-") {
            "audio"
        } else if crate_name.starts_with("image-") {
            "image"
        } else {
            "video"
        };
        let rust_ident = crate_name.replace('-', "_");
        let cli = read_source(root.join(format!("crates/{domain}/{crate_name}-cli/src/lib.rs")));
        let server =
            read_source(root.join(format!("crates/{domain}/{crate_name}-server/src/lib.rs")));
        let wasm = read_source(root.join(format!("crates/bindings/{crate_name}-wasm/src/lib.rs")));
        let call = format!("{rust_ident}::surface::run_surface_operation");

        for (surface, source) in [("cli", cli), ("server", server), ("wasm", wasm)] {
            assert!(
                source.contains(&call),
                "{crate_name} {surface} adapter must call library-owned run_surface_operation"
            );
            assert!(
                !source.contains(".operation.as_str()"),
                "{crate_name} {surface} adapter must not branch on operation IDs"
            );
        }
    }
}

#[test]
fn retired_runtime_surfaces_are_documented_while_tracked() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let docs = fs::read_to_string(root.join("docs/runtime-surfaces.md")).unwrap();
    for retired in ["runtime-artifacts", "runtime-jobs"] {
        if root.join("crates/runtime").join(retired).exists() {
            assert!(
                docs.contains(retired),
                "tracked retired runtime surface {retired} must be documented"
            );
        }
    }
}

#[test]
fn retired_runtime_frontend_apps_do_not_return() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for path in [
        "packages/runtime-artifacts-app",
        "packages/runtime-jobs-app",
    ] {
        assert!(
            !root.join(path).exists(),
            "retired runtime app surface `{path}` must not be recreated; use jobs-core or model-runtime surfaces"
        );
    }
}

fn workspace_manifests(root: &Path) -> Vec<PathBuf> {
    let mut manifests = Vec::new();
    collect_manifests(root, &mut manifests);
    manifests
}

fn library_manifests(root: &Path) -> Vec<PathBuf> {
    workspace_manifests(root)
        .into_iter()
        .filter(|manifest| {
            let relative = manifest.strip_prefix(root).unwrap_or(manifest);
            let parts = relative.components().collect::<Vec<_>>();
            if parts.len() < 4 {
                return false;
            }
            let path = relative.to_string_lossy();
            path.starts_with("crates/")
                && !path.starts_with("crates/bindings/")
                && manifest
                    .parent()
                    .and_then(|parent| parent.file_name())
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| {
                        !name.ends_with("-cli")
                            && !name.ends_with("-server")
                            && !name.ends_with("-wasm")
                    })
        })
        .collect()
}

fn package_name(manifest: &Path) -> String {
    fs::read_to_string(manifest)
        .unwrap()
        .lines()
        .find_map(|line| {
            let line = line.trim();
            line.strip_prefix("name = ")
                .map(|name| name.trim_matches('"').to_string())
        })
        .expect("package name")
}

fn read_source(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path.as_ref())
        .unwrap_or_else(|err| panic!("read source `{}`: {err}", path.as_ref().display()))
}

fn surface_package_name(package_name: &str) -> &str {
    package_name
}

fn has_exact_base_dependency(cargo: &str, package_name: &str, surface_name: &str) -> bool {
    cargo.contains(&format!(
        "{surface_name} = {{ path = \"../{package_name}\" }}"
    )) || cargo.contains(&format!(
        "{surface_name} = {{ path = \"../{surface_name}\" }}"
    )) || cargo.lines().any(|line| {
        let line = line.trim();
        line.starts_with(&format!("{surface_name} = {{"))
            && (line.contains(&format!("path = \"../{package_name}\""))
                || line.contains(&format!("path = \"../{surface_name}\"")))
    })
}

fn local_markdown_link_targets(markdown: &str) -> Vec<String> {
    let mut targets = Vec::new();
    let mut rest = markdown;

    while let Some(link_start) = rest.find("](") {
        rest = &rest[link_start + 2..];
        let Some(link_end) = rest.find(')') else {
            break;
        };
        let target = &rest[..link_end];
        rest = &rest[link_end + 1..];

        let target = target.split('#').next().unwrap_or_default().trim();
        if target.is_empty()
            || target.contains("://")
            || target.starts_with("mailto:")
            || target.starts_with('/')
        {
            continue;
        }
        targets.push(target.to_string());
    }

    targets
}

fn has_rust_test_marker(src_dir: &Path) -> bool {
    let Ok(entries) = fs::read_dir(src_dir) else {
        return false;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if has_rust_test_marker(&path) {
                return true;
            }
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            let Ok(source) = fs::read_to_string(&path) else {
                continue;
            };
            if source.contains("#[cfg(test)]") || source.contains("#[test]") {
                return true;
            }
        }
    }
    false
}

fn has_rust_integration_test(crate_dir: &Path) -> bool {
    crate_dir
        .join("tests")
        .read_dir()
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .any(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "rs")
        })
}

fn has_frontend_unit_test(package_dir: &Path) -> bool {
    has_file_matching(package_dir, |path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| {
                name.ends_with(".test.js")
                    || name.ends_with(".test.ts")
                    || name.ends_with(".test.tsx")
            })
    })
}

fn has_frontend_e2e_test(package_dir: &Path) -> bool {
    has_file_matching(&package_dir.join("e2e"), |path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".spec.ts") || name.ends_with(".spec.tsx"))
    })
}

fn has_file_matching(dir: &Path, predicate: impl Fn(&Path) -> bool + Copy) -> bool {
    let Ok(entries) = fs::read_dir(dir) else {
        return false;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if file_name == "node_modules" || file_name == "dist" || file_name == "target" {
            continue;
        }
        if path.is_dir() {
            if has_file_matching(&path, predicate) {
                return true;
            }
        } else if predicate(&path) {
            return true;
        }
    }
    false
}

fn collect_manifests(dir: &Path, manifests: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if file_name == ".git"
            || file_name == ".cargo-target"
            || file_name == "target"
            || file_name == "vendor"
            || file_name == "node_modules"
            || is_retired_runtime_surface(&path)
        {
            continue;
        }
        if path.is_dir() {
            collect_manifests(&path, manifests);
        } else if file_name == "Cargo.toml" {
            manifests.push(path);
        }
    }
}

fn is_retired_runtime_surface(path: &Path) -> bool {
    path.components().any(|component| {
        component.as_os_str().to_str().is_some_and(|name| {
            matches!(
                name,
                "runtime-artifacts"
                    | "runtime-artifacts-cli"
                    | "runtime-artifacts-server"
                    | "runtime-artifacts-wasm"
                    | "runtime-jobs"
                    | "runtime-jobs-cli"
                    | "runtime-jobs-server"
                    | "runtime-jobs-wasm"
            )
        })
    })
}
