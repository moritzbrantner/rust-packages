use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io::{Read, Write};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ComfyDataError {
    #[error("duplicate workflow node id `{0}`")]
    DuplicateNodeId(WorkflowNodeId),
    #[error("duplicate workflow link id `{0}`")]
    DuplicateLinkId(u64),
    #[error("workflow link `{link_id}` references missing {endpoint} node `{node_id}`")]
    MissingLinkNode {
        link_id: u64,
        endpoint: &'static str,
        node_id: WorkflowNodeId,
    },
    #[error("workflow node `{node_id}` input `{input}` references missing link `{link_id}`")]
    MissingInputLink {
        node_id: WorkflowNodeId,
        input: String,
        link_id: u64,
    },
    #[error("workflow node `{node_id}` output `{output}` references missing link `{link_id}`")]
    MissingOutputLink {
        node_id: WorkflowNodeId,
        output: String,
        link_id: u64,
    },
    #[error("invalid ComfyUI JSON: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, ComfyDataError>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WorkflowNodeId {
    Number(u64),
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
pub struct ComfyWorkflow {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<Value>,
    #[serde(default)]
    pub nodes: Vec<WorkflowNode>,
    #[serde(default)]
    pub links: Vec<WorkflowLink>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<WorkflowGroup>,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub config: Map<String, Value>,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub extra: Map<String, Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_node_id: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_link_id: Option<u64>,
    #[serde(flatten)]
    pub extensions: BTreeMap<String, Value>,
}

impl ComfyWorkflow {
    pub fn from_reader(reader: impl Read) -> Result<Self> {
        Ok(serde_json::from_reader(reader)?)
    }

    pub fn from_json_str(input: &str) -> Result<Self> {
        Ok(serde_json::from_str(input)?)
    }

    pub fn write_pretty(&self, writer: impl Write) -> Result<()> {
        Ok(serde_json::to_writer_pretty(writer, self)?)
    }

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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowNode {
    pub id: WorkflowNodeId,
    #[serde(rename = "type")]
    pub node_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pos: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<Value>,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub flags: Map<String, Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<i64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<WorkflowInput>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outputs: Vec<WorkflowOutput>,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub properties: Map<String, Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub widgets_values: Vec<Value>,
    #[serde(flatten)]
    pub extensions: BTreeMap<String, Value>,
}

impl WorkflowNode {
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
pub struct WorkflowInput {
    pub name: String,
    #[serde(rename = "type")]
    pub value_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link: Option<u64>,
    #[serde(flatten)]
    pub extensions: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowOutput {
    pub name: String,
    #[serde(rename = "type")]
    pub value_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slot_index: Option<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<u64>,
    #[serde(flatten)]
    pub extensions: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(from = "WorkflowLinkTuple", into = "WorkflowLinkTuple")]
pub struct WorkflowLink {
    pub id: u64,
    pub origin_id: WorkflowNodeId,
    pub origin_slot: u64,
    pub target_id: WorkflowNodeId,
    pub target_slot: u64,
    pub value_type: String,
}

impl WorkflowLink {
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
pub struct WorkflowGroup {
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bounding: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_size: Option<u64>,
    #[serde(flatten)]
    pub extensions: BTreeMap<String, Value>,
}

pub type PromptGraph = BTreeMap<String, PromptNode>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PromptNode {
    pub class_type: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub inputs: BTreeMap<String, Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
    #[serde(flatten)]
    pub extensions: BTreeMap<String, Value>,
}

impl PromptNode {
    pub fn new(class_type: impl Into<String>) -> Self {
        Self {
            class_type: class_type.into(),
            inputs: BTreeMap::new(),
            _meta: None,
            extensions: BTreeMap::new(),
        }
    }

    pub fn input(mut self, name: impl Into<String>, value: impl Into<Value>) -> Self {
        self.inputs.insert(name.into(), value.into());
        self
    }

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
pub struct PromptLink {
    pub node_id: String,
    pub output_index: u64,
}

pub fn prompt_link(node_id: impl Into<String>, output_index: u64) -> Value {
    Value::Array(vec![
        Value::String(node_id.into()),
        Value::Number(output_index.into()),
    ])
}

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

pub fn prompt_from_reader(reader: impl Read) -> Result<PromptGraph> {
    Ok(serde_json::from_reader(reader)?)
}

pub fn prompt_from_json_str(input: &str) -> Result<PromptGraph> {
    Ok(serde_json::from_str(input)?)
}

pub fn write_prompt_pretty(prompt: &PromptGraph, writer: impl Write) -> Result<()> {
    Ok(serde_json::to_writer_pretty(writer, prompt)?)
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
