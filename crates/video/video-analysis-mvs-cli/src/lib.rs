use runtime_contracts::{OperationId, PackageSurface, SurfaceRequest, SurfaceResponse};

/// Wrapped library crate name.
pub const LIBRARY_CRATE: &str = "video-analysis-mvs";
/// Adapter surface kind.
pub const SURFACE_KIND: &str = "cli";
/// Rust import path for the wrapped crate.
pub const LIBRARY_IMPORT: &str = "use video_analysis_mvs";
/// Companion server package name.
pub const SERVER_PACKAGE: &str = "video-analysis-mvs-server";
/// Companion React app package name.
pub const APP_PACKAGE: &str = "video-analysis-mvs-app";
/// Companion WASM package name.
pub const WASM_PACKAGE: &str = "video-analysis-mvs-wasm";

pub fn package_surface() -> PackageSurface {
    video_analysis_mvs::surface::package_surface()
}

pub fn package_metadata_json() -> String {
    serde_json::json!({
        "package": format!("{}-cli", LIBRARY_CRATE),
        "surface": SURFACE_KIND,
        "library": LIBRARY_CRATE,
        "libraryImport": LIBRARY_IMPORT,
        "serverPackage": SERVER_PACKAGE,
        "appPackage": APP_PACKAGE,
        "wasmPackage": WASM_PACKAGE,
        "operations": package_surface().operations
    })
    .to_string()
}

pub fn command_schema_json() -> String {
    serde_json::json!({
        "commands": [
            {"name": "info", "description": "Print package and adapter metadata."},
            {"name": "schema", "description": "Print the CLI command schema."},
            {"name": "operations", "description": "Print library operations."},
            {"name": "run", "description": "Run one library-owned operation."}
        ]
    })
    .to_string()
}

pub fn run_operation(operation: &str, input: serde_json::Value) -> Result<SurfaceResponse, String> {
    video_analysis_mvs::surface::run_surface_operation(SurfaceRequest {
        operation: OperationId::new(operation),
        input,
    })
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
