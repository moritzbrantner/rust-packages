//! Library-owned runtime surface for `video-analysis-youtube`.

use runtime_core::{OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation};

/// Returns the package surface exposed by transport wrappers.
pub fn package_surface() -> PackageSurface {
    PackageSurface {
        library: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: RuntimeCapabilities::pure_rust().with_requirement(
            "yt-dlp",
            "Required for YouTube metadata, caption, and media acquisition.",
            true,
        ),
        operations: vec![
            operation(
                "video.youtube.discoverCollection",
                "Discover YouTube collection",
                "Discovers video entries from a channel, playlist, or collection URL.",
            ),
            operation(
                "video.youtube.downloadCaptions",
                "Download YouTube captions",
                "Downloads captions with yt-dlp and parses them through text-transcripts.",
            ),
            operation(
                "video.youtube.downloadMedia",
                "Download YouTube media",
                "Downloads media with yt-dlp and validates the reported output path.",
            ),
        ],
    }
}

fn operation(id: &str, name: &str, description: &str) -> SurfaceOperation {
    SurfaceOperation {
        id: OperationId::new(id),
        name: name.to_string(),
        description: Some(description.to_string()),
        input_schema: serde_json::json!({"type": "object", "additionalProperties": true}),
        output_schema: serde_json::json!({"type": "object"}),
        example_request: serde_json::json!({}),
        wasm_supported: false,
        server_supported: true,
    }
}
