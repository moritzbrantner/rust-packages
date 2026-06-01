use std::io;

use runtime_core::{
    server::{self, ServerAdapterMetadata},
    PackageSurface, SurfaceRequest, SurfaceResponse,
};

/// Wrapped library crate name.
pub const LIBRARY_CRATE: &str = "video-analysis-colmap-backend";
/// Adapter surface kind.
pub const SURFACE_KIND: &str = "api";
/// Rust import path for the wrapped crate.
pub const LIBRARY_IMPORT: &str = "use video_analysis_colmap_backend";
/// Companion CLI package name.
pub const CLI_PACKAGE: &str = "video-analysis-colmap-backend-cli";
/// Companion React app package name.
pub const APP_PACKAGE: &str = "video-analysis-colmap-backend-app";
/// Companion WASM package name.
pub const WASM_PACKAGE: &str = "video-analysis-colmap-backend-wasm";

pub type HttpResponse = server::HttpResponse;

const METADATA: ServerAdapterMetadata = ServerAdapterMetadata {
    library_crate: LIBRARY_CRATE,
    surface_kind: SURFACE_KIND,
    library_import: LIBRARY_IMPORT,
    cli_package: CLI_PACKAGE,
    app_package: APP_PACKAGE,
    wasm_package: WASM_PACKAGE,
};

pub fn package_surface() -> PackageSurface {
    video_analysis_colmap_backend::surface::package_surface()
}

pub fn serve(addr: &str) -> io::Result<()> {
    server::serve(addr, METADATA, package_surface, run_server_operation)
}

pub fn response_for(method: &str, path: &str, body: &str) -> HttpResponse {
    server::response_for(
        method,
        path,
        body,
        METADATA,
        package_surface,
        run_server_operation,
    )
}

pub fn package_metadata_json() -> String {
    server::package_metadata_json(METADATA, package_surface())
}

fn run_server_operation(request: SurfaceRequest) -> Result<SurfaceResponse, String> {
    if request.operation.as_str()
        == video_analysis_colmap_backend::surface::RECONSTRUCT_VIDEO_OPERATION
    {
        return video_analysis_colmap_backend::reconstruct_video_surface_operation(request.input);
    }
    video_analysis_colmap_backend::surface::run_surface_operation(request)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_endpoint_reports_package() {
        let response = response_for("GET", "/health", "");
        assert_eq!(response.status_code, 200);
        assert!(response.body.contains(LIBRARY_CRATE));
    }
}
