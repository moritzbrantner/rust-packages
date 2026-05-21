#![doc = include_str!("../README.md")]

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
}
