use video_analysis_core::runtime::{
    ensure_structured_surface_value, OperationId, PackageSurface, SurfaceRequest, SurfaceResponse,
};

pub const LIBRARY_CRATE: &str = "text-analysis";
pub const SURFACE_KIND: &str = "cli";
pub const LIBRARY_IMPORT: &str = "use text_analysis";
pub const SERVER_PACKAGE: &str = "text-analysis-server";
pub const APP_PACKAGE: &str = "text-analysis-app";
pub const WASM_PACKAGE: &str = "text-analysis-wasm";

pub fn package_surface() -> PackageSurface {
    text_analysis::surface::package_surface()
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
    let mut response = text_analysis::surface::run_surface_operation(SurfaceRequest {
        operation: OperationId::new(operation),
        input,
    })?;
    let value = std::mem::take(&mut response.value);
    response.value = ensure_structured_surface_value(
        &response.operation,
        operation.to_string(),
        format!("Ran package-surface operation `{}`.", operation),
        value,
    );
    Ok(response)
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
