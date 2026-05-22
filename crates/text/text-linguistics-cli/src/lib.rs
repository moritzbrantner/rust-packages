/// Wrapped library crate name.
pub const LIBRARY_CRATE: &str = "text-linguistics";
/// Adapter surface kind.
pub const SURFACE_KIND: &str = "cli";
/// Rust import path for the wrapped crate.
pub const LIBRARY_IMPORT: &str = "use text_linguistics";
/// Companion server package name.
pub const SERVER_PACKAGE: &str = "text-linguistics-server";
/// Companion React app package name.
pub const APP_PACKAGE: &str = "text-linguistics-app";

/// Returns JSON metadata for this CLI adapter.
pub fn package_metadata_json() -> String {
    serde_json::json!({
        "package": format!("{}-cli", LIBRARY_CRATE),
        "surface": SURFACE_KIND,
        "library": LIBRARY_CRATE,
        "libraryImport": LIBRARY_IMPORT,
        "serverPackage": SERVER_PACKAGE,
        "appPackage": APP_PACKAGE
    })
    .to_string()
}

/// Returns a compact command schema for this generic CLI adapter.
pub fn command_schema_json() -> String {
    serde_json::json!({
        "commands": [
            {
                "name": "info",
                "description": "Print package and adapter metadata."
            },
            {
                "name": "schema",
                "description": "Print the generic CLI command schema."
            },
            {
                "name": "analyze",
                "description": "Analyze supplied text and emit JSON.",
                "options": [
                    "--profile <fast|balanced|rich>",
                    "--entity-recognition <local-model|heuristic>",
                    "--model-dir <path>",
                    "--no-auto-download"
                ],
                "defaultModel": "bert-base-ner"
            },
            {
                "name": "analyze-file",
                "description": "Analyze a UTF-8 text file and emit JSON.",
                "options": [
                    "--profile <fast|balanced|rich>",
                    "--entity-recognition <local-model|heuristic>",
                    "--model-dir <path>",
                    "--no-auto-download"
                ],
                "defaultModel": "bert-base-ner"
            }
        ]
    })
    .to_string()
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
