use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct DiagnosticCode(pub String);

impl DiagnosticCode {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for DiagnosticCode {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<String> for DiagnosticCode {
    fn from(value: String) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    pub severity: DiagnosticSeverity,
    pub code: DiagnosticCode,
    pub message: String,
    pub source: Option<String>,
    pub help: Option<String>,
}

impl Diagnostic {
    pub fn new(
        severity: DiagnosticSeverity,
        code: impl Into<DiagnosticCode>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            code: code.into(),
            message: message.into(),
            source: None,
            help: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeCapabilities {
    pub native: bool,
    pub server: bool,
    pub wasm: bool,
    pub mobile: MobileCapability,
    pub requirements: Vec<RuntimeRequirement>,
    pub max_recommended_input_bytes: Option<u64>,
}

impl RuntimeCapabilities {
    pub fn pure_rust() -> Self {
        Self {
            native: true,
            server: true,
            wasm: true,
            mobile: MobileCapability::Wasm,
            requirements: Vec::new(),
            max_recommended_input_bytes: None,
        }
    }

    pub fn with_max_recommended_input_bytes(mut self, bytes: u64) -> Self {
        self.max_recommended_input_bytes = Some(bytes);
        self
    }

    pub fn with_requirement(
        mut self,
        name: impl Into<String>,
        description: impl Into<String>,
        required: bool,
    ) -> Self {
        self.requirements.push(RuntimeRequirement {
            name: name.into(),
            description: Some(description.into()),
            required,
        });
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MobileCapability {
    Native,
    Wasm,
    ApiOnly,
    Unsupported,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeRequirement {
    pub name: String,
    pub description: Option<String>,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct OperationId(pub String);

impl OperationId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for OperationId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<String> for OperationId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OperationMetadata {
    pub id: OperationId,
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub capabilities: RuntimeCapabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PackageSurface {
    pub library: String,
    pub version: String,
    pub operations: Vec<SurfaceOperation>,
    pub capabilities: RuntimeCapabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceOperation {
    pub id: OperationId,
    pub name: String,
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
    pub output_schema: serde_json::Value,
    pub example_request: serde_json::Value,
    pub wasm_supported: bool,
    pub server_supported: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceRequest {
    pub operation: OperationId,
    pub input: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceResponse {
    pub operation: OperationId,
    pub value: serde_json::Value,
    pub diagnostics: Vec<Diagnostic>,
    pub artifacts: Vec<serde_json::Value>,
}

/// Builds the standard package-surface operation metadata used by library
/// crates and transport adapters.
pub fn surface_operation(
    id: impl Into<String>,
    name: impl Into<String>,
    description: impl Into<String>,
    example_request: serde_json::Value,
) -> SurfaceOperation {
    SurfaceOperation {
        id: OperationId::new(id),
        name: name.into(),
        description: Some(description.into()),
        input_schema: serde_json::json!({"type": "object", "additionalProperties": true}),
        output_schema: serde_json::json!({"type": "object"}),
        example_request,
        wasm_supported: true,
        server_supported: true,
    }
}

/// Builds the standard `describe` response without changing the shared
/// `SurfaceResponse` JSON shape.
pub fn describe_surface_response(
    surface: &PackageSurface,
    request: SurfaceRequest,
) -> SurfaceResponse {
    surface_response(
        request.operation,
        serde_json::json!({
            "library": &surface.library,
            "version": &surface.version,
            "operationCount": surface.operations.len(),
            "operations": surface
                .operations
                .iter()
                .map(|operation| operation.id.as_str())
                .collect::<Vec<_>>(),
            "input": request.input
        }),
    )
}

/// Builds a successful surface response with empty diagnostics and artifacts.
pub fn surface_response(operation: OperationId, value: serde_json::Value) -> SurfaceResponse {
    SurfaceResponse {
        operation,
        value,
        diagnostics: Vec::new(),
        artifacts: Vec::new(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct JobId(pub String);

impl JobId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for JobId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<String> for JobId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct ArtifactId(pub String);

impl ArtifactId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ArtifactId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<String> for ArtifactId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_uses_camel_case_json() {
        let diagnostic = Diagnostic::new(DiagnosticSeverity::Warning, "demo.warning", "check");
        let json = serde_json::to_string(&diagnostic).expect("serialize diagnostic");

        assert!(json.contains("\"severity\":\"warning\""));
        assert!(json.contains("\"code\":\"demo.warning\""));
    }

    #[test]
    fn pure_rust_capabilities_allow_wasm_and_server() {
        let capabilities = RuntimeCapabilities::pure_rust();

        assert!(capabilities.native);
        assert!(capabilities.server);
        assert!(capabilities.wasm);
        assert_eq!(capabilities.mobile, MobileCapability::Wasm);
    }

    #[test]
    fn capability_builders_preserve_pure_rust_defaults() {
        let capabilities = RuntimeCapabilities::pure_rust()
            .with_max_recommended_input_bytes(1024)
            .with_requirement("fixture", "test fixture input", false);

        assert!(capabilities.native);
        assert!(capabilities.server);
        assert!(capabilities.wasm);
        assert_eq!(capabilities.max_recommended_input_bytes, Some(1024));
        assert_eq!(capabilities.requirements[0].name, "fixture");
        assert!(!capabilities.requirements[0].required);
    }

    #[test]
    fn package_surface_uses_camel_case_json() {
        let surface = PackageSurface {
            library: "demo-core".to_string(),
            version: "0.1.0".to_string(),
            capabilities: RuntimeCapabilities::pure_rust(),
            operations: vec![SurfaceOperation {
                id: OperationId::new("describe"),
                name: "Describe".to_string(),
                description: Some("Describe package surface".to_string()),
                input_schema: serde_json::json!({"type": "object"}),
                output_schema: serde_json::json!({"type": "object"}),
                example_request: serde_json::json!({}),
                wasm_supported: true,
                server_supported: true,
            }],
        };

        let json = serde_json::to_string(&surface).expect("serialize surface");

        assert!(json.contains("\"inputSchema\""));
        assert!(json.contains("\"exampleRequest\""));
        assert!(json.contains("\"wasmSupported\":true"));
    }

    #[test]
    fn surface_helpers_preserve_standard_response_shape() {
        let surface = PackageSurface {
            library: "demo".to_string(),
            version: "0.1.0".to_string(),
            capabilities: RuntimeCapabilities::pure_rust(),
            operations: vec![surface_operation(
                "describe",
                "Describe",
                "Describe demo package",
                serde_json::json!({"includeOperations": true}),
            )],
        };
        let response = describe_surface_response(
            &surface,
            SurfaceRequest {
                operation: OperationId::new("describe"),
                input: serde_json::json!({"includeOperations": true}),
            },
        );

        assert_eq!(response.operation.as_str(), "describe");
        assert_eq!(response.value["library"], "demo");
        assert_eq!(response.value["operationCount"], 1);
        assert_eq!(response.diagnostics, Vec::new());
        assert_eq!(response.artifacts, Vec::<serde_json::Value>::new());
    }
}
