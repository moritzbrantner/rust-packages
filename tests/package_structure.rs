use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const CARGO_PACKAGE_PREFIXES: &[&str] = &["moritzbrantner-", "moenarch-"];
const NPM_PACKAGE_SCOPES: &[&str] = &["@moritzbrantner/", "@moenarch/"];

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
        "@moritzbrantner/video-analysis-web needs API integration tests"
    );
}

#[test]
fn frontend_libraries_and_ui_packages_have_expected_test_layers() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));

    assert!(
        has_frontend_unit_test(&root.join("packages/text-core-wasm")),
        "@moritzbrantner/text-core-wasm needs frontend package unit tests"
    );
    assert!(
        has_frontend_e2e_test(&root.join("packages/video-analysis-ui")),
        "@moritzbrantner/video-analysis-ui needs browser e2e tests"
    );
    assert!(
        has_frontend_e2e_test(&root.join("prototypes/web/video-analysis-web")),
        "@moritzbrantner/video-analysis-web needs browser e2e tests"
    );
}

#[test]
fn public_package_identifiers_use_active_ownership_prefix() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut failures = Vec::new();

    for file in tracked_identifier_files(root) {
        let relative = file.strip_prefix(root).unwrap_or(&file);
        let Ok(text) = fs::read_to_string(&file) else {
            continue;
        };
        let legacy_mb_scope = concat!("@", "mb-rust/");
        let legacy_video_scope = concat!("@", "video-analysis/");
        if text.contains(legacy_mb_scope) || text.contains(legacy_video_scope) {
            failures.push(format!(
                "{} still references an old npm package scope",
                relative.display()
            ));
        }

        if file.file_name().is_some_and(|name| name == "package.json") {
            if let Some(name) = json_name_field(&text) {
                if !has_known_npm_scope(&name) {
                    failures.push(format!(
                        "{} package name `{name}` is not under a known public npm scope",
                        relative.display()
                    ));
                }
            }
        }

        if file.file_name().is_some_and(|name| name == "Cargo.toml") {
            if let Some(name) = cargo_package_name(&text) {
                if !is_active_rust_package_name(&name) {
                    failures.push(format!(
                        "{} package name `{name}` is not under an active Rust package namespace",
                        relative.display()
                    ));
                }
            }
        }
    }

    assert!(
        failures.is_empty(),
        "public package identifiers need active ownership prefixes: {}",
        failures.join(", ")
    );
}

#[test]
fn synthetic_fixtures_are_not_public_package_identifier_inputs() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let files = tracked_identifier_files(root);

    assert!(
        files
            .iter()
            .all(|path| !path.starts_with(root.join("scripts/fixtures"))),
        "synthetic test fixtures must not be treated as public packages"
    );
    assert!(
        files.iter().any(|path| path == &root.join("Cargo.toml")),
        "the real root package manifest must remain covered"
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
            if !wasm_package_dependency_names(surface_name)
                .iter()
                .any(|dependency| package_json.contains(dependency))
            {
                missing.push(format!(
                    "{surface_name}: app does not depend on matching wasm package"
                ));
            }
            let app_tsx = fs::read_to_string(app_dir.join("src/App.tsx")).unwrap();
            if !app_tsx.contains("PackageSurfaceWorkbench") {
                missing.push(format!(
                    "{surface_name}: app does not render the shared package surface workbench"
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
fn all_paired_rust_adapters_delegate_to_library_owned_surfaces() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let packages = active_workspace_packages(root);
    let library_packages = packages
        .iter()
        .filter_map(|package| paired_library_package(root, package))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut failures = Vec::new();

    for package in packages {
        let Some(name) = package["name"].as_str() else {
            continue;
        };
        let Some((surface_name, adapter_kind)) = adapter_surface_name(name) else {
            continue;
        };
        let Some(library) = library_packages.get(surface_name) else {
            continue;
        };
        if adapter_parity_exception(name) {
            continue;
        }
        let manifest = PathBuf::from(package["manifest_path"].as_str().expect("manifest path"));
        let source_path = manifest.parent().expect("crate dir").join("src/lib.rs");
        let source = read_source(&source_path);
        let call_package = format!("{}::surface::package_surface()", library.import_name);
        let call_run = format!("{}::surface::run_surface_operation", library.import_name);

        if !source.contains(&call_package) {
            failures.push(format!(
                "{name} ({adapter_kind}) must call `{call_package}`"
            ));
        }
        if !source.contains(&call_run) {
            failures.push(format!("{name} ({adapter_kind}) must call `{call_run}`"));
        }
        if source.contains(".operation.as_str()") {
            failures.push(format!(
                "{name} ({adapter_kind}) must not branch on operation IDs"
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "paired Rust adapters must stay thin wrappers: {}",
        failures.join(", ")
    );
}

#[test]
fn retired_runtime_surfaces_are_removed_and_documented() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let docs = fs::read_to_string(root.join("docs/runtime-surfaces.md")).unwrap();
    for retired in ["runtime-artifacts", "runtime-jobs"] {
        assert!(
            !root.join("crates/runtime").join(retired).exists(),
            "retired runtime surface {retired} must not be an active crate"
        );
        assert!(
            docs.contains(retired),
            "removed runtime surface {retired} must remain documented"
        );
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

#[test]
fn cargo_package_selectors_use_active_prefixed_names() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let packages = active_workspace_packages(root);
    let names = packages
        .iter()
        .filter_map(|package| package["name"].as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let unprefixed = names
        .iter()
        .filter_map(|name| strip_known_cargo_prefix(name).map(|short| (short, *name)))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut failures = Vec::new();

    for file in tracked_command_files(root) {
        let relative = file.strip_prefix(root).unwrap_or(&file);
        let Ok(text) = fs::read_to_string(&file) else {
            continue;
        };
        for (line_index, line) in text.lines().enumerate() {
            if !line.contains("cargo") || !line.contains("-p") {
                continue;
            }
            for package in cargo_package_selectors(line) {
                if is_active_rust_package_name(&package) {
                    if !names.contains(package.as_str())
                        && !has_active_known_owner_package(&package, &names)
                        && !line.contains('<')
                        && !line.contains('{')
                    {
                        failures.push(format!(
                            "{}:{} uses unknown package selector `-p {package}`",
                            relative.display(),
                            line_index + 1
                        ));
                    }
                } else if let Some(expected) = unprefixed.get(package.as_str()) {
                    failures.push(format!(
                        "{}:{} uses `-p {package}`; use `-p {expected}`",
                        relative.display(),
                        line_index + 1
                    ));
                }
            }
        }
    }

    assert!(
        failures.is_empty(),
        "Cargo package selectors must use active prefixed package names: {}",
        failures.join(", ")
    );
}

#[test]
fn generated_server_wrappers_delegate_to_runtime_core() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let offenders = active_workspace_packages(root)
        .into_iter()
        .filter_map(|package| {
            let name = package["name"].as_str()?;
            if !name.ends_with("-server") {
                return None;
            }
            let manifest = PathBuf::from(package["manifest_path"].as_str()?);
            let source = fs::read_to_string(manifest.parent()?.join("src/lib.rs")).ok()?;
            let reimplements_http = source.contains("fn run_request")
                || source.contains("TcpListener")
                || source.contains("DiagnosticSeverity")
                || source.contains("Content-Length");
            reimplements_http.then(|| name.to_string())
        })
        .collect::<Vec<_>>();

    assert!(
        offenders.is_empty(),
        "server wrappers must delegate HTTP routing to runtime_core::server: {}",
        offenders.join(", ")
    );
}

fn workspace_manifests(root: &Path) -> Vec<PathBuf> {
    active_workspace_packages(root)
        .into_iter()
        .map(|package| PathBuf::from(package["manifest_path"].as_str().expect("manifest path")))
        .collect()
}

fn library_manifests(root: &Path) -> Vec<PathBuf> {
    active_workspace_packages(root)
        .into_iter()
        .filter_map(|package| {
            let manifest = PathBuf::from(package["manifest_path"].as_str().expect("manifest path"));
            let package_name = package["name"].as_str().expect("package name");
            let relative = manifest.strip_prefix(root).unwrap_or(&manifest);
            let path = relative.to_string_lossy();
            if !path.starts_with("crates/")
                || path.starts_with("crates/bindings/")
                || package_name.ends_with("-cli")
                || package_name.ends_with("-server")
                || package_name.ends_with("-wasm")
                || excluded_library_package(package_name)
            {
                return None;
            }
            has_library_target(&package).then_some(manifest)
        })
        .collect()
}

#[derive(Debug)]
struct PairedLibraryPackage {
    import_name: String,
}

fn paired_library_package(
    root: &Path,
    package: &serde_json::Value,
) -> Option<(String, PairedLibraryPackage)> {
    let manifest = PathBuf::from(package["manifest_path"].as_str()?);
    let package_name = package["name"].as_str()?;
    let relative = manifest.strip_prefix(root).ok()?.to_string_lossy();
    if !relative.starts_with("crates/")
        || relative.starts_with("crates/bindings/")
        || package_name.ends_with("-cli")
        || package_name.ends_with("-server")
        || package_name.ends_with("-wasm")
        || !has_library_target(package)
    {
        return None;
    }
    let surface_name = surface_package_name(package_name).to_string();
    let source = fs::read_to_string(manifest.parent()?.join("src/lib.rs")).ok()?;
    source.contains("pub mod surface;").then_some((
        surface_name.clone(),
        PairedLibraryPackage {
            import_name: surface_name.replace('-', "_"),
        },
    ))
}

fn adapter_surface_name(package_name: &str) -> Option<(&str, &'static str)> {
    let surface_name = surface_package_name(package_name);
    surface_name
        .strip_suffix("-cli")
        .map(|name| (name, "cli"))
        .or_else(|| {
            surface_name
                .strip_suffix("-server")
                .map(|name| (name, "server"))
        })
        .or_else(|| {
            surface_name
                .strip_suffix("-wasm")
                .map(|name| (name, "wasm"))
        })
}

fn adapter_parity_exception(package_name: &str) -> bool {
    matches!(
        package_name,
        // Native server dispatch boundary: reconstruct.video delegates into the
        // library crate's server-side reconstruction entry point.
        "moenarch-video-analysis-sfm-server"
    )
}

fn active_workspace_packages(root: &Path) -> Vec<serde_json::Value> {
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
        .collect::<std::collections::BTreeSet<_>>();
    metadata["packages"]
        .as_array()
        .expect("packages")
        .iter()
        .filter(|package| {
            package["id"]
                .as_str()
                .is_some_and(|id| members.contains(id))
        })
        .cloned()
        .collect()
}

fn has_library_target(package: &serde_json::Value) -> bool {
    package["targets"].as_array().is_some_and(|targets| {
        targets.iter().any(|target| {
            target["kind"]
                .as_array()
                .is_some_and(|kinds| kinds.iter().any(|kind| kind == "lib"))
        })
    })
}

fn cargo_package_selectors(line: &str) -> Vec<String> {
    line.split_whitespace()
        .collect::<Vec<_>>()
        .windows(2)
        .filter(|window| window[0] == "-p")
        .map(|window| {
            window[1]
                .trim_matches(|ch: char| ch == '\'' || ch == '"' || ch == ',' || ch == ';')
                .to_string()
        })
        .collect()
}

fn tracked_command_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for relative in ["scripts", "docs", ".github"] {
        let path = root.join(relative);
        if path.is_file() {
            files.push(path);
        } else {
            collect_command_files(&path, &mut files);
        }
    }
    files
}

fn collect_command_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_command_files(&path, files);
        } else if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| matches!(ext, "json" | "md" | "sh" | "yml" | "yaml"))
        {
            files.push(path);
        }
    }
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

fn cargo_package_name(manifest_text: &str) -> Option<String> {
    let mut in_package = false;
    for line in manifest_text.lines() {
        let line = line.trim();
        if line == "[package]" {
            in_package = true;
            continue;
        }
        if in_package && line.starts_with('[') {
            return None;
        }
        if in_package {
            if let Some(name) = line.strip_prefix("name = ") {
                return Some(name.trim_matches('"').to_string());
            }
        }
    }
    None
}

fn json_name_field(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let line = line.trim();
        let value = line.strip_prefix("\"name\": ")?;
        Some(value.trim_end_matches(',').trim_matches('"').to_string())
    })
}

fn read_source(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path.as_ref())
        .unwrap_or_else(|err| panic!("read source `{}`: {err}", path.as_ref().display()))
}

fn surface_package_name(package_name: &str) -> &str {
    strip_known_cargo_prefix(package_name).unwrap_or(package_name)
}

fn has_known_cargo_prefix(package_name: &str) -> bool {
    CARGO_PACKAGE_PREFIXES
        .iter()
        .any(|prefix| package_name.starts_with(prefix))
}

fn strip_known_cargo_prefix(package_name: &str) -> Option<&str> {
    CARGO_PACKAGE_PREFIXES
        .iter()
        .find_map(|prefix| package_name.strip_prefix(prefix))
}

fn has_active_known_owner_package(
    package_name: &str,
    names: &std::collections::BTreeSet<&str>,
) -> bool {
    let Some(short) = strip_known_cargo_prefix(package_name) else {
        return false;
    };
    CARGO_PACKAGE_PREFIXES.iter().any(|prefix| {
        let candidate = format!("{prefix}{short}");
        names.contains(candidate.as_str())
    })
}

fn has_known_npm_scope(package_name: &str) -> bool {
    NPM_PACKAGE_SCOPES
        .iter()
        .any(|scope| package_name.starts_with(scope))
}

fn wasm_package_dependency_names(surface_name: &str) -> Vec<String> {
    NPM_PACKAGE_SCOPES
        .iter()
        .map(|scope| format!("{scope}{surface_name}-wasm"))
        .collect()
}

fn excluded_library_package(package_name: &str) -> bool {
    matches!(
        package_name,
        "moenarch-audio-analysis-test-support"
            | "moenarch-runtime-core"
            | "moenarch-runtime-onnx"
            | "moenarch-video-analysis-test-support"
    )
}

fn is_active_rust_package_name(name: &str) -> bool {
    has_known_cargo_prefix(name)
}

fn has_exact_base_dependency(cargo: &str, package_name: &str, surface_name: &str) -> bool {
    cargo.contains(&format!(
        "{surface_name} = {{ path = \"../{package_name}\" }}"
    )) || cargo.contains(&format!(
        "{surface_name} = {{ path = \"../{surface_name}\" }}"
    )) || cargo.contains(&format!(
        "{surface_name} = {{ package = \"{package_name}\", path = \"../{surface_name}\" }}"
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

fn tracked_identifier_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_identifier_files(root, &mut files);
    files
}

fn collect_identifier_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if file_name == ".git"
            || file_name == ".cargo-target"
            || file_name == ".external-test-tools"
            || file_name == "target"
            || file_name == "vendor"
            || file_name == "references"
            || file_name == "node_modules"
            || file_name == "pkg"
            || file_name == "dist"
            || path.ends_with("scripts/fixtures")
        {
            continue;
        }
        if path.ends_with("prototypes/web/video-analysis-web/public/workspace-architecture.json") {
            continue;
        }
        if path.is_dir() {
            collect_identifier_files(&path, files);
        } else if is_identifier_file(&path) {
            files.push(path);
        }
    }
}

fn is_identifier_file(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    if matches!(
        file_name,
        "Cargo.toml" | "package.json" | "README.md" | "bun.lock" | "Cargo.lock"
    ) {
        return true;
    }
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension,
                "rs" | "ts" | "tsx" | "js" | "mjs" | "json" | "md" | "py" | "sh"
            )
        })
}
