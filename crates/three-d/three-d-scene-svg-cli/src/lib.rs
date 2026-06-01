use runtime_core::{
    cli::{self, CliAdapterMetadata},
    PackageSurface, SurfaceResponse,
};

/// Wrapped library crate name.
pub const LIBRARY_CRATE: &str = "three-d-scene-svg";
/// Adapter surface kind.
pub const SURFACE_KIND: &str = "cli";
/// Rust import path for the wrapped crate.
pub const LIBRARY_IMPORT: &str = "use three_d_scene_svg";
/// Companion server package name.
pub const SERVER_PACKAGE: &str = "three-d-scene-svg-server";
/// Companion React app package name.
pub const APP_PACKAGE: &str = "three-d-scene-svg-app";
/// Companion WASM package name.
pub const WASM_PACKAGE: &str = "three-d-scene-svg-wasm";

const METADATA: CliAdapterMetadata = CliAdapterMetadata {
    library_crate: LIBRARY_CRATE,
    surface_kind: SURFACE_KIND,
    library_import: LIBRARY_IMPORT,
    server_package: SERVER_PACKAGE,
    app_package: APP_PACKAGE,
    wasm_package: WASM_PACKAGE,
};

pub fn package_surface() -> PackageSurface {
    three_d_scene_svg::surface::package_surface()
}

pub fn package_metadata_json() -> String {
    cli::package_metadata_json(METADATA, package_surface())
}

pub fn command_schema_json() -> String {
    cli::command_schema_json()
}

pub fn run_operation(operation: &str, input: serde_json::Value) -> Result<SurfaceResponse, String> {
    cli::run_wrapped_operation(
        operation,
        input,
        three_d_scene_svg::surface::run_surface_operation,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_mentions_wrapped_library() {
        let metadata = package_metadata_json();
        assert!(metadata.contains(LIBRARY_CRATE));
        assert!(metadata.contains(SURFACE_KIND));
    }
}
