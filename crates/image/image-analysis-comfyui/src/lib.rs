#![doc = include_str!("../README.md")]

use std::collections::BTreeMap;

use comfyui_data::{ComfyWorkflow, WorkflowInput, WorkflowLink, WorkflowNode, WorkflowOutput};
use serde_json::Value;
use video_analysis_core::{DetectError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImageGenerationMode {
    #[default]
    TextToImage,
    ImageToImage,
    Inpaint,
    Upscale,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImageGenerationRequest {
    pub mode: ImageGenerationMode,
    pub prompt: String,
    pub negative_prompt: String,
    pub width: u32,
    pub height: u32,
    pub steps: u32,
    pub cfg_scale: f32,
    pub sampler_name: String,
    pub scheduler: String,
    pub seed: u64,
    pub checkpoint: String,
    pub input_image: Option<String>,
    pub mask_image: Option<String>,
    pub denoise: f32,
    pub output_prefix: String,
    pub upscale_model: String,
}

impl ImageGenerationRequest {
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            mode: ImageGenerationMode::TextToImage,
            prompt: prompt.into(),
            negative_prompt: String::new(),
            width: 1024,
            height: 1024,
            steps: 30,
            cfg_scale: 7.0,
            sampler_name: "euler".to_string(),
            scheduler: "normal".to_string(),
            seed: 0,
            checkpoint: "sdxl.safetensors".to_string(),
            input_image: None,
            mask_image: None,
            denoise: 0.8,
            output_prefix: "image_analysis".to_string(),
            upscale_model: "4x-UltraSharp.pth".to_string(),
        }
    }

    pub fn mode(mut self, value: ImageGenerationMode) -> Self {
        self.mode = value;
        self
    }

    pub fn negative_prompt(mut self, value: impl Into<String>) -> Self {
        self.negative_prompt = value.into();
        self
    }

    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn steps(mut self, value: u32) -> Self {
        self.steps = value;
        self
    }

    pub fn cfg_scale(mut self, value: f32) -> Self {
        self.cfg_scale = value;
        self
    }

    pub fn sampler_name(mut self, value: impl Into<String>) -> Self {
        self.sampler_name = value.into();
        self
    }

    pub fn scheduler(mut self, value: impl Into<String>) -> Self {
        self.scheduler = value.into();
        self
    }

    pub fn seed(mut self, value: u64) -> Self {
        self.seed = value;
        self
    }

    pub fn checkpoint(mut self, value: impl Into<String>) -> Self {
        self.checkpoint = value.into();
        self
    }

    pub fn input_image(mut self, value: impl Into<String>) -> Self {
        self.input_image = Some(value.into());
        self
    }

    pub fn mask_image(mut self, value: impl Into<String>) -> Self {
        self.mask_image = Some(value.into());
        self
    }

    pub fn denoise(mut self, value: f32) -> Self {
        self.denoise = value;
        self
    }

    pub fn output_prefix(mut self, value: impl Into<String>) -> Self {
        self.output_prefix = value.into();
        self
    }

    pub fn upscale_model(mut self, value: impl Into<String>) -> Self {
        self.upscale_model = value.into();
        self
    }
}

pub fn build_generation_workflow(request: &ImageGenerationRequest) -> Result<ComfyWorkflow> {
    validate_request(request)?;
    let workflow = match request.mode {
        ImageGenerationMode::TextToImage => build_text_to_image_workflow(request),
        ImageGenerationMode::ImageToImage => build_image_to_image_workflow(request),
        ImageGenerationMode::Inpaint => build_inpaint_workflow(request),
        ImageGenerationMode::Upscale => build_upscale_workflow(request),
    };
    workflow
        .validate()
        .map_err(|err| DetectError::Source(format!("invalid ComfyUI workflow: {err}")))?;
    Ok(workflow)
}

fn build_text_to_image_workflow(request: &ImageGenerationRequest) -> ComfyWorkflow {
    let links = vec![
        WorkflowLink::new(1, 1_u64, 0, 5_u64, 0, "MODEL"),
        WorkflowLink::new(2, 1_u64, 1, 2_u64, 0, "CLIP"),
        WorkflowLink::new(3, 1_u64, 1, 3_u64, 0, "CLIP"),
        WorkflowLink::new(4, 1_u64, 2, 6_u64, 1, "VAE"),
        WorkflowLink::new(5, 2_u64, 0, 5_u64, 1, "CONDITIONING"),
        WorkflowLink::new(6, 3_u64, 0, 5_u64, 2, "CONDITIONING"),
        WorkflowLink::new(7, 4_u64, 0, 5_u64, 3, "LATENT"),
        WorkflowLink::new(8, 5_u64, 0, 6_u64, 0, "LATENT"),
        WorkflowLink::new(9, 6_u64, 0, 7_u64, 0, "IMAGE"),
    ];
    workflow(
        vec![
            node(
                1,
                "CheckpointLoaderSimple",
                vec![],
                vec![
                    output("MODEL", "MODEL", 0, &[1]),
                    output("CLIP", "CLIP", 1, &[2, 3]),
                    output("VAE", "VAE", 2, &[4]),
                ],
                vec![Value::String(request.checkpoint.clone())],
            ),
            node(
                2,
                "CLIPTextEncode",
                vec![linked_input("clip", "CLIP", 2)],
                vec![output("CONDITIONING", "CONDITIONING", 0, &[5])],
                vec![Value::String(request.prompt.clone())],
            ),
            node(
                3,
                "CLIPTextEncode",
                vec![linked_input("clip", "CLIP", 3)],
                vec![output("CONDITIONING", "CONDITIONING", 0, &[6])],
                vec![Value::String(request.negative_prompt.clone())],
            ),
            node(
                4,
                "EmptyLatentImage",
                vec![],
                vec![output("LATENT", "LATENT", 0, &[7])],
                vec![
                    Value::from(request.width),
                    Value::from(request.height),
                    Value::from(1_u64),
                ],
            ),
            node(
                5,
                "KSampler",
                vec![
                    linked_input("model", "MODEL", 1),
                    linked_input("positive", "CONDITIONING", 5),
                    linked_input("negative", "CONDITIONING", 6),
                    linked_input("latent_image", "LATENT", 7),
                ],
                vec![output("LATENT", "LATENT", 0, &[8])],
                sampler_widgets(request, 1.0),
            ),
            node(
                6,
                "VAEDecode",
                vec![
                    linked_input("samples", "LATENT", 8),
                    linked_input("vae", "VAE", 4),
                ],
                vec![output("IMAGE", "IMAGE", 0, &[9])],
                vec![],
            ),
            node(
                7,
                "SaveImage",
                vec![linked_input("images", "IMAGE", 9)],
                vec![],
                vec![Value::String(request.output_prefix.clone())],
            ),
        ],
        links,
    )
}

fn build_image_to_image_workflow(request: &ImageGenerationRequest) -> ComfyWorkflow {
    let links = vec![
        WorkflowLink::new(1, 1_u64, 0, 6_u64, 0, "MODEL"),
        WorkflowLink::new(2, 1_u64, 1, 2_u64, 0, "CLIP"),
        WorkflowLink::new(3, 1_u64, 1, 3_u64, 0, "CLIP"),
        WorkflowLink::new(4, 1_u64, 2, 5_u64, 1, "VAE"),
        WorkflowLink::new(5, 2_u64, 0, 6_u64, 1, "CONDITIONING"),
        WorkflowLink::new(6, 3_u64, 0, 6_u64, 2, "CONDITIONING"),
        WorkflowLink::new(7, 4_u64, 0, 5_u64, 0, "IMAGE"),
        WorkflowLink::new(8, 5_u64, 0, 6_u64, 3, "LATENT"),
        WorkflowLink::new(9, 6_u64, 0, 7_u64, 0, "LATENT"),
        WorkflowLink::new(10, 1_u64, 2, 7_u64, 1, "VAE"),
        WorkflowLink::new(11, 7_u64, 0, 8_u64, 0, "IMAGE"),
    ];
    workflow(
        vec![
            node(
                1,
                "CheckpointLoaderSimple",
                vec![],
                vec![
                    output("MODEL", "MODEL", 0, &[1]),
                    output("CLIP", "CLIP", 1, &[2, 3]),
                    output("VAE", "VAE", 2, &[4, 10]),
                ],
                vec![Value::String(request.checkpoint.clone())],
            ),
            node(
                2,
                "CLIPTextEncode",
                vec![linked_input("clip", "CLIP", 2)],
                vec![output("CONDITIONING", "CONDITIONING", 0, &[5])],
                vec![Value::String(request.prompt.clone())],
            ),
            node(
                3,
                "CLIPTextEncode",
                vec![linked_input("clip", "CLIP", 3)],
                vec![output("CONDITIONING", "CONDITIONING", 0, &[6])],
                vec![Value::String(request.negative_prompt.clone())],
            ),
            node(
                4,
                "LoadImage",
                vec![],
                vec![output("IMAGE", "IMAGE", 0, &[7])],
                vec![Value::String(
                    request.input_image.clone().unwrap_or_default(),
                )],
            ),
            node(
                5,
                "VAEEncode",
                vec![
                    linked_input("pixels", "IMAGE", 7),
                    linked_input("vae", "VAE", 4),
                ],
                vec![output("LATENT", "LATENT", 0, &[8])],
                vec![],
            ),
            node(
                6,
                "KSampler",
                vec![
                    linked_input("model", "MODEL", 1),
                    linked_input("positive", "CONDITIONING", 5),
                    linked_input("negative", "CONDITIONING", 6),
                    linked_input("latent_image", "LATENT", 8),
                ],
                vec![output("LATENT", "LATENT", 0, &[9])],
                sampler_widgets(request, request.denoise),
            ),
            node(
                7,
                "VAEDecode",
                vec![
                    linked_input("samples", "LATENT", 9),
                    linked_input("vae", "VAE", 10),
                ],
                vec![output("IMAGE", "IMAGE", 0, &[11])],
                vec![],
            ),
            node(
                8,
                "SaveImage",
                vec![linked_input("images", "IMAGE", 11)],
                vec![],
                vec![Value::String(request.output_prefix.clone())],
            ),
        ],
        links,
    )
}

fn build_inpaint_workflow(request: &ImageGenerationRequest) -> ComfyWorkflow {
    let links = vec![
        WorkflowLink::new(1, 1_u64, 0, 7_u64, 0, "MODEL"),
        WorkflowLink::new(2, 1_u64, 1, 2_u64, 0, "CLIP"),
        WorkflowLink::new(3, 1_u64, 1, 3_u64, 0, "CLIP"),
        WorkflowLink::new(4, 1_u64, 2, 6_u64, 1, "VAE"),
        WorkflowLink::new(5, 2_u64, 0, 7_u64, 1, "CONDITIONING"),
        WorkflowLink::new(6, 3_u64, 0, 7_u64, 2, "CONDITIONING"),
        WorkflowLink::new(7, 4_u64, 0, 6_u64, 0, "IMAGE"),
        WorkflowLink::new(8, 5_u64, 0, 6_u64, 2, "MASK"),
        WorkflowLink::new(9, 6_u64, 0, 7_u64, 3, "LATENT"),
        WorkflowLink::new(10, 7_u64, 0, 8_u64, 0, "LATENT"),
        WorkflowLink::new(11, 1_u64, 2, 8_u64, 1, "VAE"),
        WorkflowLink::new(12, 8_u64, 0, 9_u64, 0, "IMAGE"),
    ];
    workflow(
        vec![
            node(
                1,
                "CheckpointLoaderSimple",
                vec![],
                vec![
                    output("MODEL", "MODEL", 0, &[1]),
                    output("CLIP", "CLIP", 1, &[2, 3]),
                    output("VAE", "VAE", 2, &[4, 11]),
                ],
                vec![Value::String(request.checkpoint.clone())],
            ),
            node(
                2,
                "CLIPTextEncode",
                vec![linked_input("clip", "CLIP", 2)],
                vec![output("CONDITIONING", "CONDITIONING", 0, &[5])],
                vec![Value::String(request.prompt.clone())],
            ),
            node(
                3,
                "CLIPTextEncode",
                vec![linked_input("clip", "CLIP", 3)],
                vec![output("CONDITIONING", "CONDITIONING", 0, &[6])],
                vec![Value::String(request.negative_prompt.clone())],
            ),
            node(
                4,
                "LoadImage",
                vec![],
                vec![output("IMAGE", "IMAGE", 0, &[7])],
                vec![Value::String(
                    request.input_image.clone().unwrap_or_default(),
                )],
            ),
            node(
                5,
                "LoadImageMask",
                vec![],
                vec![output("MASK", "MASK", 0, &[8])],
                vec![Value::String(
                    request.mask_image.clone().unwrap_or_default(),
                )],
            ),
            node(
                6,
                "VAEEncodeForInpaint",
                vec![
                    linked_input("pixels", "IMAGE", 7),
                    linked_input("vae", "VAE", 4),
                    linked_input("mask", "MASK", 8),
                ],
                vec![output("LATENT", "LATENT", 0, &[9])],
                vec![],
            ),
            node(
                7,
                "KSampler",
                vec![
                    linked_input("model", "MODEL", 1),
                    linked_input("positive", "CONDITIONING", 5),
                    linked_input("negative", "CONDITIONING", 6),
                    linked_input("latent_image", "LATENT", 9),
                ],
                vec![output("LATENT", "LATENT", 0, &[10])],
                sampler_widgets(request, request.denoise),
            ),
            node(
                8,
                "VAEDecode",
                vec![
                    linked_input("samples", "LATENT", 10),
                    linked_input("vae", "VAE", 11),
                ],
                vec![output("IMAGE", "IMAGE", 0, &[12])],
                vec![],
            ),
            node(
                9,
                "SaveImage",
                vec![linked_input("images", "IMAGE", 12)],
                vec![],
                vec![Value::String(request.output_prefix.clone())],
            ),
        ],
        links,
    )
}

fn build_upscale_workflow(request: &ImageGenerationRequest) -> ComfyWorkflow {
    let links = vec![
        WorkflowLink::new(1, 1_u64, 0, 3_u64, 0, "IMAGE"),
        WorkflowLink::new(2, 2_u64, 0, 3_u64, 1, "UPSCALE_MODEL"),
        WorkflowLink::new(3, 3_u64, 0, 4_u64, 0, "IMAGE"),
    ];
    workflow(
        vec![
            node(
                1,
                "LoadImage",
                vec![],
                vec![output("IMAGE", "IMAGE", 0, &[1])],
                vec![Value::String(
                    request.input_image.clone().unwrap_or_default(),
                )],
            ),
            node(
                2,
                "UpscaleModelLoader",
                vec![],
                vec![output("UPSCALE_MODEL", "UPSCALE_MODEL", 0, &[2])],
                vec![Value::String(request.upscale_model.clone())],
            ),
            node(
                3,
                "ImageUpscaleWithModel",
                vec![
                    linked_input("image", "IMAGE", 1),
                    linked_input("upscale_model", "UPSCALE_MODEL", 2),
                ],
                vec![output("IMAGE", "IMAGE", 0, &[3])],
                vec![],
            ),
            node(
                4,
                "SaveImage",
                vec![linked_input("images", "IMAGE", 3)],
                vec![],
                vec![Value::String(request.output_prefix.clone())],
            ),
        ],
        links,
    )
}

fn validate_request(request: &ImageGenerationRequest) -> Result<()> {
    if request.width == 0 || request.height == 0 {
        return Err(DetectError::InvalidDimensions {
            width: request.width,
            height: request.height,
        });
    }
    if !request.cfg_scale.is_finite() || request.cfg_scale < 0.0 {
        return Err(DetectError::InvalidArgument(
            "cfg_scale must be finite and non-negative".to_string(),
        ));
    }
    if !request.denoise.is_finite() || !(0.0..=1.0).contains(&request.denoise) {
        return Err(DetectError::InvalidArgument(
            "denoise must be finite and in the range 0..=1".to_string(),
        ));
    }
    match request.mode {
        ImageGenerationMode::TextToImage => {}
        ImageGenerationMode::ImageToImage | ImageGenerationMode::Upscale => {
            if request
                .input_image
                .as_deref()
                .unwrap_or_default()
                .is_empty()
            {
                return Err(DetectError::InvalidArgument(
                    "input_image is required for this generation mode".to_string(),
                ));
            }
        }
        ImageGenerationMode::Inpaint => {
            if request
                .input_image
                .as_deref()
                .unwrap_or_default()
                .is_empty()
                || request.mask_image.as_deref().unwrap_or_default().is_empty()
            {
                return Err(DetectError::InvalidArgument(
                    "input_image and mask_image are required for inpaint mode".to_string(),
                ));
            }
        }
    }
    Ok(())
}

fn workflow(nodes: Vec<WorkflowNode>, links: Vec<WorkflowLink>) -> ComfyWorkflow {
    ComfyWorkflow {
        last_node_id: nodes
            .iter()
            .filter_map(|node| match &node.id {
                comfyui_data::WorkflowNodeId::Number(value) => Some(*value),
                comfyui_data::WorkflowNodeId::String(_) => None,
            })
            .max(),
        last_link_id: links.iter().map(|link| link.id).max(),
        nodes,
        links,
        ..ComfyWorkflow::default()
    }
}

fn node(
    id: u64,
    node_type: &str,
    inputs: Vec<WorkflowInput>,
    outputs: Vec<WorkflowOutput>,
    widgets_values: Vec<Value>,
) -> WorkflowNode {
    let mut node = WorkflowNode::new(id, node_type);
    node.inputs = inputs;
    node.outputs = outputs;
    node.widgets_values = widgets_values;
    node
}

fn linked_input(name: &str, value_type: &str, link: u64) -> WorkflowInput {
    WorkflowInput {
        name: name.to_string(),
        value_type: value_type.to_string(),
        link: Some(link),
        extensions: BTreeMap::new(),
    }
}

fn output(name: &str, value_type: &str, slot_index: u64, links: &[u64]) -> WorkflowOutput {
    WorkflowOutput {
        name: name.to_string(),
        value_type: value_type.to_string(),
        slot_index: Some(slot_index),
        links: links.to_vec(),
        extensions: BTreeMap::new(),
    }
}

fn sampler_widgets(request: &ImageGenerationRequest, denoise: f32) -> Vec<Value> {
    vec![
        Value::from(request.seed),
        Value::from(request.steps),
        Value::from(request.cfg_scale),
        Value::String(request.sampler_name.clone()),
        Value::String(request.scheduler.clone()),
        Value::from(denoise),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_text_to_image_workflow() {
        let workflow =
            build_generation_workflow(&ImageGenerationRequest::new("red fox in a field")).unwrap();
        assert!(workflow
            .nodes
            .iter()
            .any(|node| node.node_type == "KSampler"));
    }

    #[test]
    fn builds_image_to_image_workflow() {
        let workflow = build_generation_workflow(
            &ImageGenerationRequest::new("stylized portrait")
                .mode(ImageGenerationMode::ImageToImage)
                .input_image("input.png"),
        )
        .unwrap();
        assert!(workflow
            .nodes
            .iter()
            .any(|node| node.node_type == "VAEEncode"));
    }

    #[test]
    fn builds_inpaint_workflow() {
        let workflow = build_generation_workflow(
            &ImageGenerationRequest::new("repair the damaged area")
                .mode(ImageGenerationMode::Inpaint)
                .input_image("input.png")
                .mask_image("mask.png"),
        )
        .unwrap();
        assert!(workflow
            .nodes
            .iter()
            .any(|node| node.node_type == "VAEEncodeForInpaint"));
    }

    #[test]
    fn builds_upscale_workflow() {
        let workflow = build_generation_workflow(
            &ImageGenerationRequest::new("unused")
                .mode(ImageGenerationMode::Upscale)
                .input_image("input.png"),
        )
        .unwrap();
        assert!(workflow
            .nodes
            .iter()
            .any(|node| node.node_type == "ImageUpscaleWithModel"));
    }
}
