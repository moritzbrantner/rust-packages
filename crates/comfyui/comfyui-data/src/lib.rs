#![doc = include_str!("../README.md")]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io::{Read, Write};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tensor_data::F32Tensor;
use thiserror::Error;

#[derive(Debug, Error)]
/// Variants describing comfy data error.
pub enum ComfyDataError {
    #[error("duplicate workflow node id `{0}`")]
    /// The duplicate node identifier variant.
    DuplicateNodeId(WorkflowNodeId),
    #[error("duplicate workflow link id `{0}`")]
    /// The duplicate link identifier variant.
    DuplicateLinkId(u64),
    #[error("workflow link `{link_id}` references missing {endpoint} node `{node_id}`")]
    /// The missing link node variant.
    MissingLinkNode {
        /// The link identifier value for this variant.
        link_id: u64,
        /// The endpoint value for this variant.
        endpoint: &'static str,
        /// The node identifier value for this variant.
        node_id: WorkflowNodeId,
    },
    #[error("workflow node `{node_id}` input `{input}` references missing link `{link_id}`")]
    /// The missing input link variant.
    MissingInputLink {
        /// The node identifier value for this variant.
        node_id: WorkflowNodeId,
        /// Input value that triggered this variant.
        input: String,
        /// The link identifier value for this variant.
        link_id: u64,
    },
    #[error("workflow node `{node_id}` output `{output}` references missing link `{link_id}`")]
    /// The missing output link variant.
    MissingOutputLink {
        /// The node identifier value for this variant.
        node_id: WorkflowNodeId,
        /// The output value for this variant.
        output: String,
        /// The link identifier value for this variant.
        link_id: u64,
    },
    #[error("invalid ComfyUI JSON: {0}")]
    /// The JSON variant.
    Json(#[from] serde_json::Error),
    #[error("invalid conditioning schema: {0}")]
    /// The invalid conditioning variant.
    InvalidConditioning(String),
}

/// Type alias for result.
pub type Result<T> = std::result::Result<T, ComfyDataError>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// Variants describing comfy socket type.
pub enum ComfySocketType {
    /// The int variant.
    Int,
    /// The float variant.
    Float,
    /// The string variant.
    String,
    /// The boolean variant.
    Boolean,
    /// The combo variant.
    Combo,
    /// The image variant.
    Image,
    /// The mask variant.
    Mask,
    /// The audio variant.
    Audio,
    /// The video variant.
    Video,
    /// The latent variant.
    Latent,
    /// The model variant.
    Model,
    /// The clip variant.
    Clip,
    /// The clip vision variant.
    ClipVision,
    /// The vae variant.
    Vae,
    /// The conditioning variant.
    Conditioning,
    /// The upscale model variant.
    UpscaleModel,
    /// The model patch variant.
    ModelPatch,
    /// The mesh variant.
    Mesh,
    /// The noise variant.
    Noise,
    /// The sampler variant.
    Sampler,
    /// The sigmas variant.
    Sigmas,
    /// The guider variant.
    Guider,
    /// The custom variant.
    Custom(String),
}

impl ComfySocketType {
    /// Builds this value from socket type.
    pub fn from_socket_type(value: &str) -> Self {
        let normalized = value.trim();
        match normalized.to_ascii_uppercase().as_str() {
            "INT" => Self::Int,
            "FLOAT" => Self::Float,
            "STRING" => Self::String,
            "BOOLEAN" | "BOOL" => Self::Boolean,
            "COMBO" => Self::Combo,
            "IMAGE" => Self::Image,
            "MASK" => Self::Mask,
            "AUDIO" => Self::Audio,
            "VIDEO" => Self::Video,
            "LATENT" => Self::Latent,
            "MODEL" => Self::Model,
            "CLIP" => Self::Clip,
            "CLIP_VISION" => Self::ClipVision,
            "VAE" => Self::Vae,
            "CONDITIONING" => Self::Conditioning,
            "UPSCALE_MODEL" => Self::UpscaleModel,
            "MODEL_PATCH" | "MODELPATCH" => Self::ModelPatch,
            "MESH" => Self::Mesh,
            "NOISE" => Self::Noise,
            "SAMPLER" => Self::Sampler,
            "SIGMAS" => Self::Sigmas,
            "GUIDER" => Self::Guider,
            value => Self::Custom(value.to_string()),
        }
    }

    /// Borrows this value as a str.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Int => "INT",
            Self::Float => "FLOAT",
            Self::String => "STRING",
            Self::Boolean => "BOOLEAN",
            Self::Combo => "COMBO",
            Self::Image => "IMAGE",
            Self::Mask => "MASK",
            Self::Audio => "AUDIO",
            Self::Video => "VIDEO",
            Self::Latent => "LATENT",
            Self::Model => "MODEL",
            Self::Clip => "CLIP",
            Self::ClipVision => "CLIP_VISION",
            Self::Vae => "VAE",
            Self::Conditioning => "CONDITIONING",
            Self::UpscaleModel => "UPSCALE_MODEL",
            Self::ModelPatch => "MODELPATCH",
            Self::Mesh => "MESH",
            Self::Noise => "NOISE",
            Self::Sampler => "SAMPLER",
            Self::Sigmas => "SIGMAS",
            Self::Guider => "GUIDER",
            Self::Custom(value) => value.as_str(),
        }
    }
}

impl fmt::Display for ComfySocketType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
/// Data type for workflow type inventory.
pub struct WorkflowTypeInventory {
    /// The inputs value.
    pub inputs: BTreeSet<ComfySocketType>,
    /// The outputs value.
    pub outputs: BTreeSet<ComfySocketType>,
    /// The links value.
    pub links: BTreeSet<ComfySocketType>,
}

impl WorkflowTypeInventory {
    /// Returns all.
    pub fn all(&self) -> BTreeSet<ComfySocketType> {
        self.inputs
            .iter()
            .chain(&self.outputs)
            .chain(&self.links)
            .cloned()
            .collect()
    }

    /// Returns contains.
    pub fn contains(&self, socket_type: &ComfySocketType) -> bool {
        self.inputs.contains(socket_type)
            || self.outputs.contains(socket_type)
            || self.links.contains(socket_type)
    }

    /// Returns whether is empty.
    pub fn is_empty(&self) -> bool {
        self.inputs.is_empty() && self.outputs.is_empty() && self.links.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Data type for conditioning item.
pub struct ConditioningItem {
    /// The embedding value.
    pub embedding: F32Tensor,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// The pooled embedding value.
    pub pooled_embedding: Option<F32Tensor>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    /// Metadata associated with this value.
    pub metadata: BTreeMap<String, Value>,
}

impl ConditioningItem {
    /// Creates a new value.
    pub fn new(embedding: F32Tensor) -> Result<Self> {
        let item = Self {
            embedding,
            pooled_embedding: None,
            metadata: BTreeMap::new(),
        };
        item.validate()?;
        Ok(item)
    }

    /// Returns this value with pooled embedding.
    pub fn with_pooled_embedding(mut self, pooled_embedding: F32Tensor) -> Result<Self> {
        self.pooled_embedding = Some(pooled_embedding);
        self.validate()?;
        Ok(self)
    }

    /// Validates this value.
    pub fn validate(&self) -> Result<()> {
        self.embedding
            .validate()
            .map_err(|err| ComfyDataError::InvalidConditioning(err.to_string()))?;
        let embedding_dims = self.embedding.shape().dimensions();
        if embedding_dims.len() != 2 {
            return Err(ComfyDataError::InvalidConditioning(
                "conditioning embeddings must be rank 2 [T,C] tensors".to_string(),
            ));
        }
        if let Some(pooled_embedding) = &self.pooled_embedding {
            pooled_embedding
                .validate()
                .map_err(|err| ComfyDataError::InvalidConditioning(err.to_string()))?;
            let pooled_dims = pooled_embedding.shape().dimensions();
            if pooled_dims.len() != 1 {
                return Err(ComfyDataError::InvalidConditioning(
                    "pooled conditioning embeddings must be rank 1 [C] tensors".to_string(),
                ));
            }
            if pooled_dims[0] != embedding_dims[1] {
                return Err(ComfyDataError::InvalidConditioning(format!(
                    "pooled conditioning embedding width {} does not match embedding channel dimension {}",
                    pooled_dims[0], embedding_dims[1]
                )));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Data type for conditioning batch.
pub struct ConditioningBatch {
    /// The items value.
    pub items: Vec<ConditioningItem>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    /// Metadata associated with this value.
    pub metadata: BTreeMap<String, Value>,
}

impl ConditioningBatch {
    /// Creates a new value.
    pub fn new(items: Vec<ConditioningItem>) -> Result<Self> {
        let batch = Self {
            items,
            metadata: BTreeMap::new(),
        };
        batch.validate()?;
        Ok(batch)
    }

    /// Validates this value.
    pub fn validate(&self) -> Result<()> {
        if self.items.is_empty() {
            return Err(ComfyDataError::InvalidConditioning(
                "conditioning batches must contain at least one item".to_string(),
            ));
        }
        for item in &self.items {
            item.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(untagged)]
/// Variants describing workflow node identifier.
pub enum WorkflowNodeId {
    /// The number variant.
    Number(u64),
    /// The string variant.
    String(String),
}

impl fmt::Display for WorkflowNodeId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(value) => write!(formatter, "{value}"),
            Self::String(value) => formatter.write_str(value),
        }
    }
}

impl From<u64> for WorkflowNodeId {
    fn from(value: u64) -> Self {
        Self::Number(value)
    }
}

impl From<String> for WorkflowNodeId {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&str> for WorkflowNodeId {
    fn from(value: &str) -> Self {
        Self::String(value.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
/// Data type for comfy workflow.
pub struct ComfyWorkflow {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// The version value.
    pub version: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// The state value.
    pub state: Option<Value>,
    #[serde(default)]
    /// The nodes value.
    pub nodes: Vec<WorkflowNode>,
    #[serde(default)]
    /// The links value.
    pub links: Vec<WorkflowLink>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// The groups value.
    pub groups: Vec<WorkflowGroup>,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    /// The config value.
    pub config: Map<String, Value>,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    /// The extra value.
    pub extra: Map<String, Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// The last node identifier value.
    pub last_node_id: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// The last link identifier value.
    pub last_link_id: Option<u64>,
    #[serde(flatten)]
    /// The extensions value.
    pub extensions: BTreeMap<String, Value>,
}

impl ComfyWorkflow {
    /// Builds this value from reader.
    pub fn from_reader(reader: impl Read) -> Result<Self> {
        Ok(serde_json::from_reader(reader)?)
    }

    /// Builds this value from JSON str.
    pub fn from_json_str(input: &str) -> Result<Self> {
        Ok(serde_json::from_str(input)?)
    }

    /// Writes pretty.
    pub fn write_pretty(&self, writer: impl Write) -> Result<()> {
        Ok(serde_json::to_writer_pretty(writer, self)?)
    }

    /// Validates this value.
    pub fn validate(&self) -> Result<()> {
        let mut node_ids = BTreeSet::new();
        for node in &self.nodes {
            if !node_ids.insert(node.id.clone()) {
                return Err(ComfyDataError::DuplicateNodeId(node.id.clone()));
            }
        }

        let mut link_ids = BTreeSet::new();
        for link in &self.links {
            if !link_ids.insert(link.id) {
                return Err(ComfyDataError::DuplicateLinkId(link.id));
            }
            if !node_ids.contains(&link.origin_id) {
                return Err(ComfyDataError::MissingLinkNode {
                    link_id: link.id,
                    endpoint: "origin",
                    node_id: link.origin_id.clone(),
                });
            }
            if !node_ids.contains(&link.target_id) {
                return Err(ComfyDataError::MissingLinkNode {
                    link_id: link.id,
                    endpoint: "target",
                    node_id: link.target_id.clone(),
                });
            }
        }

        for node in &self.nodes {
            for input in &node.inputs {
                if let Some(link_id) = input.link {
                    if !link_ids.contains(&link_id) {
                        return Err(ComfyDataError::MissingInputLink {
                            node_id: node.id.clone(),
                            input: input.name.clone(),
                            link_id,
                        });
                    }
                }
            }
            for output in &node.outputs {
                for link_id in &output.links {
                    if !link_ids.contains(link_id) {
                        return Err(ComfyDataError::MissingOutputLink {
                            node_id: node.id.clone(),
                            output: output.name.clone(),
                            link_id: *link_id,
                        });
                    }
                }
            }
        }
        Ok(())
    }

    /// Returns observed socket types.
    pub fn observed_socket_types(&self) -> WorkflowTypeInventory {
        let mut inventory = WorkflowTypeInventory::default();
        for node in &self.nodes {
            for input in &node.inputs {
                inventory
                    .inputs
                    .insert(ComfySocketType::from_socket_type(&input.value_type));
            }
            for output in &node.outputs {
                inventory
                    .outputs
                    .insert(ComfySocketType::from_socket_type(&output.value_type));
            }
        }
        for link in &self.links {
            inventory
                .links
                .insert(ComfySocketType::from_socket_type(&link.value_type));
        }
        inventory
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Data type for workflow node.
pub struct WorkflowNode {
    /// Identifier for this value.
    pub id: WorkflowNodeId,
    #[serde(rename = "type")]
    /// The node type value.
    pub node_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// The pos value.
    pub pos: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// The size value.
    pub size: Option<Value>,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    /// The flags value.
    pub flags: Map<String, Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// The order value.
    pub order: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// The mode value.
    pub mode: Option<i64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// The inputs value.
    pub inputs: Vec<WorkflowInput>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// The outputs value.
    pub outputs: Vec<WorkflowOutput>,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    /// The properties value.
    pub properties: Map<String, Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// The widgets values value.
    pub widgets_values: Vec<Value>,
    #[serde(flatten)]
    /// The extensions value.
    pub extensions: BTreeMap<String, Value>,
}

impl WorkflowNode {
    /// Creates a new value.
    pub fn new(id: impl Into<WorkflowNodeId>, node_type: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            node_type: node_type.into(),
            pos: None,
            size: None,
            flags: Map::new(),
            order: None,
            mode: None,
            inputs: Vec::new(),
            outputs: Vec::new(),
            properties: Map::new(),
            widgets_values: Vec::new(),
            extensions: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Data type for workflow input.
pub struct WorkflowInput {
    /// Human-readable name for this value.
    pub name: String,
    #[serde(rename = "type")]
    /// The value type value.
    pub value_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// The link value.
    pub link: Option<u64>,
    #[serde(flatten)]
    /// The extensions value.
    pub extensions: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Data type for workflow output.
pub struct WorkflowOutput {
    /// Human-readable name for this value.
    pub name: String,
    #[serde(rename = "type")]
    /// The value type value.
    pub value_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// The slot index value.
    pub slot_index: Option<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// The links value.
    pub links: Vec<u64>,
    #[serde(flatten)]
    /// The extensions value.
    pub extensions: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(from = "WorkflowLinkTuple", into = "WorkflowLinkTuple")]
/// Data type for workflow link.
pub struct WorkflowLink {
    /// Identifier for this value.
    pub id: u64,
    /// The origin identifier value.
    pub origin_id: WorkflowNodeId,
    /// The origin slot value.
    pub origin_slot: u64,
    /// The target identifier value.
    pub target_id: WorkflowNodeId,
    /// The target slot value.
    pub target_slot: u64,
    /// The value type value.
    pub value_type: String,
}

impl WorkflowLink {
    /// Creates a new value.
    pub fn new(
        id: u64,
        origin_id: impl Into<WorkflowNodeId>,
        origin_slot: u64,
        target_id: impl Into<WorkflowNodeId>,
        target_slot: u64,
        value_type: impl Into<String>,
    ) -> Self {
        Self {
            id,
            origin_id: origin_id.into(),
            origin_slot,
            target_id: target_id.into(),
            target_slot,
            value_type: value_type.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct WorkflowLinkTuple(u64, WorkflowNodeId, u64, WorkflowNodeId, u64, String);

impl From<WorkflowLinkTuple> for WorkflowLink {
    fn from(value: WorkflowLinkTuple) -> Self {
        Self {
            id: value.0,
            origin_id: value.1,
            origin_slot: value.2,
            target_id: value.3,
            target_slot: value.4,
            value_type: value.5,
        }
    }
}

impl From<WorkflowLink> for WorkflowLinkTuple {
    fn from(value: WorkflowLink) -> Self {
        Self(
            value.id,
            value.origin_id,
            value.origin_slot,
            value.target_id,
            value.target_slot,
            value.value_type,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Data type for workflow group.
pub struct WorkflowGroup {
    /// The title value.
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// The bounding value.
    pub bounding: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// The color value.
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// The font size value.
    pub font_size: Option<u64>,
    #[serde(flatten)]
    /// The extensions value.
    pub extensions: BTreeMap<String, Value>,
}

/// Type alias for prompt graph.
pub type PromptGraph = BTreeMap<String, PromptNode>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Data type for prompt node.
pub struct PromptNode {
    /// The class type value.
    pub class_type: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    /// The inputs value.
    pub inputs: BTreeMap<String, Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// The meta value.
    pub _meta: Option<Value>,
    #[serde(flatten)]
    /// The extensions value.
    pub extensions: BTreeMap<String, Value>,
}

impl PromptNode {
    /// Creates a new value.
    pub fn new(class_type: impl Into<String>) -> Self {
        Self {
            class_type: class_type.into(),
            inputs: BTreeMap::new(),
            _meta: None,
            extensions: BTreeMap::new(),
        }
    }

    /// Returns input.
    pub fn input(mut self, name: impl Into<String>, value: impl Into<Value>) -> Self {
        self.inputs.insert(name.into(), value.into());
        self
    }

    /// Returns linked input.
    pub fn linked_input(
        self,
        name: impl Into<String>,
        node_id: impl Into<String>,
        output_index: u64,
    ) -> Self {
        self.input(name, prompt_link(node_id, output_index))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Data type for prompt link.
pub struct PromptLink {
    /// The node identifier value.
    pub node_id: String,
    /// The output index value.
    pub output_index: u64,
}

/// Returns prompt link.
pub fn prompt_link(node_id: impl Into<String>, output_index: u64) -> Value {
    Value::Array(vec![
        Value::String(node_id.into()),
        Value::Number(output_index.into()),
    ])
}

/// Parses parse prompt link.
pub fn parse_prompt_link(value: &Value) -> Option<PromptLink> {
    let values = value.as_array()?;
    if values.len() != 2 {
        return None;
    }
    Some(PromptLink {
        node_id: match &values[0] {
            Value::String(value) => value.clone(),
            Value::Number(value) => value.to_string(),
            _ => return None,
        },
        output_index: values[1].as_u64()?,
    })
}

/// Returns prompt from reader.
pub fn prompt_from_reader(reader: impl Read) -> Result<PromptGraph> {
    Ok(serde_json::from_reader(reader)?)
}

/// Returns prompt from JSON str.
pub fn prompt_from_json_str(input: &str) -> Result<PromptGraph> {
    Ok(serde_json::from_str(input)?)
}

/// Writes prompt pretty.
pub fn write_prompt_pretty(prompt: &PromptGraph, writer: impl Write) -> Result<()> {
    Ok(serde_json::to_writer_pretty(writer, prompt)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image_analysis_comfyui::{
        build_generation_workflow, ComfyWorkflowPreset, ImageGenerationMode, ImageGenerationRequest,
    };

    #[test]
    fn parses_and_validates_workflow_links() {
        let json = r#"
        {
          "nodes": [
            {"id": 1, "type": "CheckpointLoaderSimple", "outputs": [{"name": "MODEL", "type": "MODEL", "links": [7]}]},
            {"id": 2, "type": "KSampler", "inputs": [{"name": "model", "type": "MODEL", "link": 7}]}
          ],
          "links": [[7, 1, 0, 2, 0, "MODEL"]]
        }
        "#;
        let workflow = ComfyWorkflow::from_json_str(json).unwrap();

        workflow.validate().unwrap();
        assert_eq!(workflow.links[0].origin_id, WorkflowNodeId::Number(1));
        assert_eq!(workflow.links[0].value_type, "MODEL");
    }

    #[test]
    fn catches_missing_workflow_link() {
        let mut workflow = ComfyWorkflow::default();
        workflow.nodes.push(WorkflowNode {
            inputs: vec![WorkflowInput {
                name: "model".to_string(),
                value_type: "MODEL".to_string(),
                link: Some(99),
                extensions: BTreeMap::new(),
            }],
            ..WorkflowNode::new(1_u64, "KSampler")
        });

        let error = workflow.validate().unwrap_err();
        assert!(matches!(error, ComfyDataError::MissingInputLink { .. }));
    }

    #[test]
    fn builds_prompt_graph_links() {
        let mut prompt = PromptGraph::new();
        prompt.insert(
            "1".to_string(),
            PromptNode::new("CheckpointLoaderSimple").input("ckpt_name", "sdxl.safetensors"),
        );
        prompt.insert(
            "2".to_string(),
            PromptNode::new("KSampler").linked_input("model", "1", 0),
        );

        let link = parse_prompt_link(&prompt["2"].inputs["model"]).unwrap();
        assert_eq!(link.node_id, "1");
        assert_eq!(link.output_index, 0);
    }

    #[test]
    fn normalizes_socket_types_emitted_by_image_analysis_comfyui() {
        let workflows = [
            build_generation_workflow(&ImageGenerationRequest::new("red cube")).unwrap(),
            build_generation_workflow(
                &ImageGenerationRequest::new("repair")
                    .mode(ImageGenerationMode::Inpaint)
                    .input_image("input.png")
                    .mask_image("mask.png"),
            )
            .unwrap(),
            build_generation_workflow(
                &ImageGenerationRequest::new("upscale")
                    .mode(ImageGenerationMode::Upscale)
                    .input_image("input.png"),
            )
            .unwrap(),
            build_generation_workflow(
                &ImageGenerationRequest::new("flux")
                    .preset(ComfyWorkflowPreset::FluxInpaint)
                    .mode(ImageGenerationMode::Inpaint)
                    .checkpoint("flux1-dev.safetensors")
                    .input_image("input.png")
                    .mask_image("mask.png"),
            )
            .unwrap(),
        ];

        let observed: BTreeSet<_> = workflows
            .iter()
            .flat_map(|workflow| workflow.observed_socket_types().all())
            .map(|socket_type| socket_type.to_string())
            .collect();

        assert!(observed.contains("MODEL"));
        assert!(observed.contains("CLIP"));
        assert!(observed.contains("VAE"));
        assert!(observed.contains("CONDITIONING"));
        assert!(observed.contains("LATENT"));
        assert!(observed.contains("IMAGE"));
        assert!(observed.contains("MASK"));
        assert!(observed.contains("UPSCALE_MODEL"));
    }

    #[test]
    fn conditioning_batch_rejects_empty_items() {
        let error = ConditioningBatch::new(Vec::new()).unwrap_err();
        assert!(matches!(error, ComfyDataError::InvalidConditioning(_)));
    }

    #[test]
    fn conditioning_batch_rejects_non_rank_two_embeddings() {
        let error =
            ConditioningItem::new(F32Tensor::from_dims([3], vec![0.0; 3]).unwrap()).unwrap_err();
        assert!(matches!(error, ComfyDataError::InvalidConditioning(_)));
    }

    #[test]
    fn conditioning_batch_rejects_pooled_width_mismatches() {
        let error = ConditioningItem::new(F32Tensor::from_dims([2, 4], vec![0.0; 8]).unwrap())
            .unwrap()
            .with_pooled_embedding(F32Tensor::from_dims([3], vec![0.0; 3]).unwrap())
            .unwrap_err();
        assert!(matches!(error, ComfyDataError::InvalidConditioning(_)));
    }

    #[test]
    fn conditioning_batch_round_trips_through_serde() {
        let batch = ConditioningBatch::new(vec![ConditioningItem::new(
            F32Tensor::from_dims([2, 4], vec![0.0; 8]).unwrap(),
        )
        .unwrap()
        .with_pooled_embedding(F32Tensor::from_dims([4], vec![0.5; 4]).unwrap())
        .unwrap()])
        .unwrap();
        let json = serde_json::to_vec(&batch).unwrap();
        let decoded: ConditioningBatch = serde_json::from_slice(&json).unwrap();
        decoded.validate().unwrap();
        assert_eq!(decoded, batch);
    }
}
