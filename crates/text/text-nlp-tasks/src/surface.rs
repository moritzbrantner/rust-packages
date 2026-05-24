//! Library-owned runtime surface for `text-nlp-tasks`.

use runtime_contracts::{PackageSurface, SurfaceRequest, SurfaceResponse};

/// Returns the package surface exposed by every transport wrapper.
pub fn package_surface() -> PackageSurface {
    let mut surface = text_nlp_models::surface::package_surface();
    surface.library = env!("CARGO_PKG_NAME").to_string();
    surface.version = env!("CARGO_PKG_VERSION").to_string();
    surface
}

/// Runs one library-owned operation.
pub fn run_surface_operation(request: SurfaceRequest) -> Result<SurfaceResponse, String> {
    let mut response = text_nlp_models::surface::run_surface_operation(request)?;
    if response.value.get("library").is_some() {
        response.value["library"] = serde_json::json!(env!("CARGO_PKG_NAME"));
        response.value["version"] = serde_json::json!(env!("CARGO_PKG_VERSION"));
    }
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use runtime_contracts::OperationId;

    #[test]
    fn package_surface_lists_delegated_nlp_operations() {
        let ids = package_surface()
            .operations
            .into_iter()
            .map(|operation| operation.id.0)
            .collect::<Vec<_>>();
        assert!(ids.contains(&"nlp.models".to_string()));
        assert!(ids.contains(&"nlp.rerank".to_string()));
    }

    #[test]
    fn delegated_models_operation_returns_catalog() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("nlp.models"),
            input: serde_json::json!({}),
        })
        .expect("models");
        assert!(!response.value["models"].as_array().unwrap().is_empty());
    }
}
