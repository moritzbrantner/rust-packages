use std::fs::File;
use std::path::Path;

use geo_io_osm::{collect_osm_pbf, CollectOsmOptions, IndexOptions, OsmFilterSpec};
use video_analysis_core::runtime::{
    ensure_structured_surface_value, OperationId, PackageSurface, SurfaceRequest, SurfaceResponse,
};

/// Wrapped library crate name.
pub const LIBRARY_CRATE: &str = "geo-io-osm";
/// Adapter surface kind.
pub const SURFACE_KIND: &str = "cli";
/// Rust import path for the wrapped crate.
pub const LIBRARY_IMPORT: &str = "use geo_io_osm";
/// Companion server package name.
pub const SERVER_PACKAGE: &str = "geo-io-osm-server";
/// Companion React app package name.
pub const APP_PACKAGE: &str = "geo-io-osm-app";
/// Companion WASM package name.
pub const WASM_PACKAGE: &str = "geo-io-osm-wasm";

pub fn package_surface() -> PackageSurface {
    geo_io_osm::surface::package_surface()
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
            {"name": "metadata", "description": "Print package and adapter metadata."},
            {"name": "info", "description": "Alias for metadata."},
            {"name": "schema", "description": "Print the CLI command schema."},
            {"name": "operations", "description": "Print library operations."},
            {"name": "run", "description": "Run one library-owned operation."},
            {"name": "filter", "description": "Filter an OSM PBF file and write GeoJSON."}
        ]
    })
    .to_string()
}

pub fn run_operation(operation: &str, input: serde_json::Value) -> Result<SurfaceResponse, String> {
    let mut response = geo_io_osm::surface::run_surface_operation(SurfaceRequest {
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

pub fn filter_to_geojson(input: &Path, spec_path: &Path, output: &Path) -> Result<usize, String> {
    let spec_text = std::fs::read_to_string(spec_path)
        .map_err(|error| format!("unable to read spec `{}`: {error}", spec_path.display()))?;
    let spec: OsmFilterSpec =
        serde_json::from_str(&spec_text).map_err(|error| format!("invalid spec JSON: {error}"))?;
    let collected = collect_osm_pbf(CollectOsmOptions {
        input: input.to_path_buf(),
        index_options: IndexOptions::from_spec(&spec.processing.index),
        spec,
    })
    .map_err(|error| error.to_string())?;
    let geo = collected.into_geo_feature_collection();
    let geojson = geo_io_geojson::to_geojson_feature_collection(&geo.features);
    if let Some(parent) = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("unable to create `{}`: {error}", parent.display()))?;
    }
    let file = File::create(output)
        .map_err(|error| format!("unable to create `{}`: {error}", output.display()))?;
    serde_json::to_writer_pretty(file, &geojson)
        .map_err(|error| format!("unable to write `{}`: {error}", output.display()))?;
    Ok(geojson.features.len())
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
