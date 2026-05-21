use runtime_contracts::{MobileCapability, OperationId, OperationMetadata, RuntimeCapabilities};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TextStatisticsRequest {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TextStatisticsResult {
    pub byte_count: usize,
    pub character_count: usize,
    pub word_count: usize,
    pub line_count: usize,
    pub sentence_count: usize,
}

pub fn text_statistics_metadata() -> OperationMetadata {
    OperationMetadata {
        id: OperationId::new("text.statistics"),
        name: "Text statistics".to_string(),
        description: Some("Counts bytes, characters, words, lines, and sentences.".to_string()),
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: RuntimeCapabilities {
            native: true,
            server: true,
            wasm: true,
            mobile: MobileCapability::Wasm,
            requirements: Vec::new(),
            max_recommended_input_bytes: Some(1_000_000),
        },
    }
}
