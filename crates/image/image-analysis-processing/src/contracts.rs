use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ApplyRequest {
    pub image: ImagePayload,
    pub operation: OperationRequest,
    pub preview_limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PipelineRequest {
    pub image: ImagePayload,
    pub operations: Vec<OperationRequest>,
    pub preview_limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CompositeRequest {
    pub base: ImagePayload,
    pub overlay: ImagePayload,
    pub mask: Option<ImagePayload>,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub opacity: Option<f32>,
    pub blend_mode: Option<String>,
    pub preview_limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HashRequest {
    pub image: ImagePayload,
    pub hash_size: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ImagePayload {
    pub width: u32,
    pub height: u32,
    pub pixel_format: String,
    pub stride: Option<usize>,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OperationRequest {
    #[serde(rename = "type")]
    pub kind: String,
    pub x: Option<u32>,
    pub y: Option<u32>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub radius: Option<u32>,
    pub brightness: Option<i16>,
    pub contrast: Option<f32>,
    pub saturation: Option<f32>,
    pub clockwise_turns: Option<u8>,
    pub level: Option<u8>,
}
