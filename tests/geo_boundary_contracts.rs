use std::collections::BTreeSet;
use std::process::Command;

use serde_json::Value;

#[test]
fn geo_crates_keep_dependency_boundaries() {
    let metadata = cargo_metadata();

    assert_normal_deps_are_subset(
        &metadata,
        "moritzbrantner-geo-core",
        &[
            "serde",
            "serde_json",
            "runtime-core",
            "moritzbrantner-runtime-core",
        ],
    );
    assert_normal_deps_are_subset(
        &metadata,
        "moritzbrantner-geo-io-geojson",
        &[
            "geojson",
            "geo-core",
            "moritzbrantner-geo-core",
            "serde",
            "serde_json",
            "runtime-core",
            "moritzbrantner-runtime-core",
        ],
    );
    assert_normal_deps_are_subset(
        &metadata,
        "moritzbrantner-geo-io-osm",
        &[
            "base64",
            "geo-core",
            "geo-io-geojson",
            "geo-types",
            "moritzbrantner-geo-core",
            "moritzbrantner-geo-io-geojson",
            "osmpbfreader",
            "protobuf",
            "redb",
            "regex",
            "runtime-core",
            "serde",
            "serde_json",
            "tempfile",
            "moritzbrantner-runtime-core",
        ],
    );
    assert_normal_deps_are_subset(
        &metadata,
        "moritzbrantner-geo-clustering",
        &[
            "geo-core",
            "moritzbrantner-geo-core",
            "runtime-core",
            "serde",
            "serde_json",
            "moritzbrantner-runtime-core",
        ],
    );
    assert_normal_deps_are_subset(
        &metadata,
        "moritzbrantner-geo-viz",
        &[
            "geo-clustering",
            "geo-core",
            "geo-io-geojson",
            "maps-kernels-core",
            "moritzbrantner-geo-clustering",
            "moritzbrantner-geo-core",
            "moritzbrantner-geo-io-geojson",
            "moritzbrantner-maps-kernels-core",
            "runtime-core",
            "rstar",
            "serde",
            "serde_json",
            "moritzbrantner-runtime-core",
        ],
    );

    assert_normal_deps_are_subset(
        &metadata,
        "moritzbrantner-maps-kernels-core",
        &[
            "runtime-core",
            "moritzbrantner-runtime-core",
            "serde",
            "serde_json",
        ],
    );

    for (adapter, library) in [
        ("moritzbrantner-geo-core-cli", "moritzbrantner-geo-core"),
        ("moritzbrantner-geo-core-server", "moritzbrantner-geo-core"),
        ("moritzbrantner-geo-core-wasm", "moritzbrantner-geo-core"),
        (
            "moritzbrantner-geo-io-geojson-cli",
            "moritzbrantner-geo-io-geojson",
        ),
        (
            "moritzbrantner-geo-io-geojson-server",
            "moritzbrantner-geo-io-geojson",
        ),
        (
            "moritzbrantner-geo-io-geojson-wasm",
            "moritzbrantner-geo-io-geojson",
        ),
        ("moritzbrantner-geo-io-osm-cli", "moritzbrantner-geo-io-osm"),
        (
            "moritzbrantner-geo-io-osm-server",
            "moritzbrantner-geo-io-osm",
        ),
        (
            "moritzbrantner-geo-io-osm-wasm",
            "moritzbrantner-geo-io-osm",
        ),
        (
            "moritzbrantner-geo-clustering-cli",
            "moritzbrantner-geo-clustering",
        ),
        (
            "moritzbrantner-geo-clustering-server",
            "moritzbrantner-geo-clustering",
        ),
        (
            "moritzbrantner-geo-clustering-wasm",
            "moritzbrantner-geo-clustering",
        ),
        ("moritzbrantner-geo-viz-cli", "moritzbrantner-geo-viz"),
        ("moritzbrantner-geo-viz-server", "moritzbrantner-geo-viz"),
        ("moritzbrantner-geo-viz-wasm", "moritzbrantner-geo-viz"),
    ] {
        assert_adapter_depends_on_library_only(&metadata, adapter, library);
    }
}

fn cargo_metadata() -> Value {
    let output = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .output()
        .expect("run cargo metadata");
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("parse cargo metadata")
}

fn assert_normal_deps_are_subset(metadata: &Value, package: &str, allowed: &[&str]) {
    let deps = normal_dependencies(metadata, package);
    let allowed = allowed.iter().copied().collect::<BTreeSet<_>>();
    for dep in deps {
        assert!(
            allowed.contains(dep.as_str()),
            "{package} must not depend on `{dep}` across the geo crate boundary"
        );
    }
}

fn assert_adapter_depends_on_library_only(metadata: &Value, adapter: &str, library: &str) {
    let deps = normal_dependencies(metadata, adapter);
    assert!(
        deps.contains(library),
        "{adapter} must depend on wrapped library `{library}`"
    );

    for dep in deps {
        let short_dep = dep.strip_prefix("moritzbrantner-").unwrap_or(&dep);
        let is_geo_adapter = short_dep.starts_with("geo-")
            && (short_dep.ends_with("-cli")
                || short_dep.ends_with("-server")
                || short_dep.ends_with("-wasm"));
        assert!(
            !is_geo_adapter,
            "{adapter} must not depend on sibling adapter crate `{dep}`"
        );
    }
}

fn normal_dependencies(metadata: &Value, package: &str) -> BTreeSet<String> {
    let packages = metadata["packages"]
        .as_array()
        .expect("metadata packages array");
    let package = packages
        .iter()
        .find(|candidate| candidate["name"].as_str() == Some(package))
        .unwrap_or_else(|| panic!("package `{package}` not found"));

    package["dependencies"]
        .as_array()
        .expect("dependencies array")
        .iter()
        .filter(|dependency| dependency["kind"].is_null())
        .flat_map(|dependency| {
            [
                dependency["name"].as_str().map(str::to_owned),
                dependency["rename"].as_str().map(str::to_owned),
            ]
        })
        .flatten()
        .collect()
}
