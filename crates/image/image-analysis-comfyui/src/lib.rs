#![doc = include_str!("../README.md")]

use std::collections::BTreeMap;

use comfyui_data::{
    ComfySocketType, ComfyWorkflow, WorkflowInput, WorkflowLink, WorkflowNode, WorkflowOutput,
};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ComfyWorkflowPreset {
    #[default]
    StandardStableDiffusion,
    FluxInpaint,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImageGenerationRequest {
    pub preset: ComfyWorkflowPreset,
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
            preset: ComfyWorkflowPreset::default(),
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

    pub fn preset(mut self, value: ComfyWorkflowPreset) -> Self {
        self.preset = value;
        self
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
    let workflow = match (request.preset, request.mode) {
        (ComfyWorkflowPreset::FluxInpaint, ImageGenerationMode::Inpaint) => {
            build_flux_inpaint_workflow(request)
        }
        (_, ImageGenerationMode::TextToImage) => build_text_to_image_workflow(request),
        (_, ImageGenerationMode::ImageToImage) => build_image_to_image_workflow(request),
        (_, ImageGenerationMode::Inpaint) => build_inpaint_workflow(request),
        (_, ImageGenerationMode::Upscale) => build_upscale_workflow(request),
    };
    workflow
        .validate()
        .map_err(|err| DetectError::Source(format!("invalid ComfyUI workflow: {err}")))?;
    Ok(workflow)
}

fn build_text_to_image_workflow(request: &ImageGenerationRequest) -> ComfyWorkflow {
    let links = vec![
        socket_link(1, 1, 0, 5, 0, ComfySocketType::Model),
        socket_link(2, 1, 1, 2, 0, ComfySocketType::Clip),
        socket_link(3, 1, 1, 3, 0, ComfySocketType::Clip),
        socket_link(4, 1, 2, 6, 1, ComfySocketType::Vae),
        socket_link(5, 2, 0, 5, 1, ComfySocketType::Conditioning),
        socket_link(6, 3, 0, 5, 2, ComfySocketType::Conditioning),
        socket_link(7, 4, 0, 5, 3, ComfySocketType::Latent),
        socket_link(8, 5, 0, 6, 0, ComfySocketType::Latent),
        socket_link(9, 6, 0, 7, 0, ComfySocketType::Image),
    ];
    workflow(
        vec![
            node(
                1,
                "CheckpointLoaderSimple",
                vec![],
                vec![
                    output("MODEL", ComfySocketType::Model, 0, &[1]),
                    output("CLIP", ComfySocketType::Clip, 1, &[2, 3]),
                    output("VAE", ComfySocketType::Vae, 2, &[4]),
                ],
                vec![Value::String(request.checkpoint.clone())],
            ),
            node(
                2,
                "CLIPTextEncode",
                vec![linked_input("clip", ComfySocketType::Clip, 2)],
                vec![output(
                    "CONDITIONING",
                    ComfySocketType::Conditioning,
                    0,
                    &[5],
                )],
                vec![Value::String(request.prompt.clone())],
            ),
            node(
                3,
                "CLIPTextEncode",
                vec![linked_input("clip", ComfySocketType::Clip, 3)],
                vec![output(
                    "CONDITIONING",
                    ComfySocketType::Conditioning,
                    0,
                    &[6],
                )],
                vec![Value::String(request.negative_prompt.clone())],
            ),
            node(
                4,
                "EmptyLatentImage",
                vec![],
                vec![output("LATENT", ComfySocketType::Latent, 0, &[7])],
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
                    linked_input("model", ComfySocketType::Model, 1),
                    linked_input("positive", ComfySocketType::Conditioning, 5),
                    linked_input("negative", ComfySocketType::Conditioning, 6),
                    linked_input("latent_image", ComfySocketType::Latent, 7),
                ],
                vec![output("LATENT", ComfySocketType::Latent, 0, &[8])],
                sampler_widgets(request, 1.0),
            ),
            node(
                6,
                "VAEDecode",
                vec![
                    linked_input("samples", ComfySocketType::Latent, 8),
                    linked_input("vae", ComfySocketType::Vae, 4),
                ],
                vec![output("IMAGE", ComfySocketType::Image, 0, &[9])],
                vec![],
            ),
            node(
                7,
                "SaveImage",
                vec![linked_input("images", ComfySocketType::Image, 9)],
                vec![],
                vec![Value::String(request.output_prefix.clone())],
            ),
        ],
        links,
    )
}

fn build_image_to_image_workflow(request: &ImageGenerationRequest) -> ComfyWorkflow {
    let links = vec![
        socket_link(1, 1, 0, 6, 0, ComfySocketType::Model),
        socket_link(2, 1, 1, 2, 0, ComfySocketType::Clip),
        socket_link(3, 1, 1, 3, 0, ComfySocketType::Clip),
        socket_link(4, 1, 2, 5, 1, ComfySocketType::Vae),
        socket_link(5, 2, 0, 6, 1, ComfySocketType::Conditioning),
        socket_link(6, 3, 0, 6, 2, ComfySocketType::Conditioning),
        socket_link(7, 4, 0, 5, 0, ComfySocketType::Image),
        socket_link(8, 5, 0, 6, 3, ComfySocketType::Latent),
        socket_link(9, 6, 0, 7, 0, ComfySocketType::Latent),
        socket_link(10, 1, 2, 7, 1, ComfySocketType::Vae),
        socket_link(11, 7, 0, 8, 0, ComfySocketType::Image),
    ];
    workflow(
        vec![
            node(
                1,
                "CheckpointLoaderSimple",
                vec![],
                vec![
                    output("MODEL", ComfySocketType::Model, 0, &[1]),
                    output("CLIP", ComfySocketType::Clip, 1, &[2, 3]),
                    output("VAE", ComfySocketType::Vae, 2, &[4, 10]),
                ],
                vec![Value::String(request.checkpoint.clone())],
            ),
            node(
                2,
                "CLIPTextEncode",
                vec![linked_input("clip", ComfySocketType::Clip, 2)],
                vec![output(
                    "CONDITIONING",
                    ComfySocketType::Conditioning,
                    0,
                    &[5],
                )],
                vec![Value::String(request.prompt.clone())],
            ),
            node(
                3,
                "CLIPTextEncode",
                vec![linked_input("clip", ComfySocketType::Clip, 3)],
                vec![output(
                    "CONDITIONING",
                    ComfySocketType::Conditioning,
                    0,
                    &[6],
                )],
                vec![Value::String(request.negative_prompt.clone())],
            ),
            node(
                4,
                "LoadImage",
                vec![],
                vec![output("IMAGE", ComfySocketType::Image, 0, &[7])],
                vec![Value::String(
                    request.input_image.clone().unwrap_or_default(),
                )],
            ),
            node(
                5,
                "VAEEncode",
                vec![
                    linked_input("pixels", ComfySocketType::Image, 7),
                    linked_input("vae", ComfySocketType::Vae, 4),
                ],
                vec![output("LATENT", ComfySocketType::Latent, 0, &[8])],
                vec![],
            ),
            node(
                6,
                "KSampler",
                vec![
                    linked_input("model", ComfySocketType::Model, 1),
                    linked_input("positive", ComfySocketType::Conditioning, 5),
                    linked_input("negative", ComfySocketType::Conditioning, 6),
                    linked_input("latent_image", ComfySocketType::Latent, 8),
                ],
                vec![output("LATENT", ComfySocketType::Latent, 0, &[9])],
                sampler_widgets(request, request.denoise),
            ),
            node(
                7,
                "VAEDecode",
                vec![
                    linked_input("samples", ComfySocketType::Latent, 9),
                    linked_input("vae", ComfySocketType::Vae, 10),
                ],
                vec![output("IMAGE", ComfySocketType::Image, 0, &[11])],
                vec![],
            ),
            node(
                8,
                "SaveImage",
                vec![linked_input("images", ComfySocketType::Image, 11)],
                vec![],
                vec![Value::String(request.output_prefix.clone())],
            ),
        ],
        links,
    )
}

fn build_inpaint_workflow(request: &ImageGenerationRequest) -> ComfyWorkflow {
    let links = vec![
        socket_link(1, 1, 0, 7, 0, ComfySocketType::Model),
        socket_link(2, 1, 1, 2, 0, ComfySocketType::Clip),
        socket_link(3, 1, 1, 3, 0, ComfySocketType::Clip),
        socket_link(4, 1, 2, 6, 1, ComfySocketType::Vae),
        socket_link(5, 2, 0, 7, 1, ComfySocketType::Conditioning),
        socket_link(6, 3, 0, 7, 2, ComfySocketType::Conditioning),
        socket_link(7, 4, 0, 6, 0, ComfySocketType::Image),
        socket_link(8, 5, 0, 6, 2, ComfySocketType::Mask),
        socket_link(9, 6, 0, 7, 3, ComfySocketType::Latent),
        socket_link(10, 7, 0, 8, 0, ComfySocketType::Latent),
        socket_link(11, 1, 2, 8, 1, ComfySocketType::Vae),
        socket_link(12, 8, 0, 9, 0, ComfySocketType::Image),
    ];
    workflow(
        vec![
            node(
                1,
                "CheckpointLoaderSimple",
                vec![],
                vec![
                    output("MODEL", ComfySocketType::Model, 0, &[1]),
                    output("CLIP", ComfySocketType::Clip, 1, &[2, 3]),
                    output("VAE", ComfySocketType::Vae, 2, &[4, 11]),
                ],
                vec![Value::String(request.checkpoint.clone())],
            ),
            node(
                2,
                "CLIPTextEncode",
                vec![linked_input("clip", ComfySocketType::Clip, 2)],
                vec![output(
                    "CONDITIONING",
                    ComfySocketType::Conditioning,
                    0,
                    &[5],
                )],
                vec![Value::String(request.prompt.clone())],
            ),
            node(
                3,
                "CLIPTextEncode",
                vec![linked_input("clip", ComfySocketType::Clip, 3)],
                vec![output(
                    "CONDITIONING",
                    ComfySocketType::Conditioning,
                    0,
                    &[6],
                )],
                vec![Value::String(request.negative_prompt.clone())],
            ),
            node(
                4,
                "LoadImage",
                vec![],
                vec![output("IMAGE", ComfySocketType::Image, 0, &[7])],
                vec![Value::String(
                    request.input_image.clone().unwrap_or_default(),
                )],
            ),
            node(
                5,
                "LoadImageMask",
                vec![],
                vec![output("MASK", ComfySocketType::Mask, 0, &[8])],
                vec![Value::String(
                    request.mask_image.clone().unwrap_or_default(),
                )],
            ),
            node(
                6,
                "VAEEncodeForInpaint",
                vec![
                    linked_input("pixels", ComfySocketType::Image, 7),
                    linked_input("vae", ComfySocketType::Vae, 4),
                    linked_input("mask", ComfySocketType::Mask, 8),
                ],
                vec![output("LATENT", ComfySocketType::Latent, 0, &[9])],
                vec![],
            ),
            node(
                7,
                "KSampler",
                vec![
                    linked_input("model", ComfySocketType::Model, 1),
                    linked_input("positive", ComfySocketType::Conditioning, 5),
                    linked_input("negative", ComfySocketType::Conditioning, 6),
                    linked_input("latent_image", ComfySocketType::Latent, 9),
                ],
                vec![output("LATENT", ComfySocketType::Latent, 0, &[10])],
                sampler_widgets(request, request.denoise),
            ),
            node(
                8,
                "VAEDecode",
                vec![
                    linked_input("samples", ComfySocketType::Latent, 10),
                    linked_input("vae", ComfySocketType::Vae, 11),
                ],
                vec![output("IMAGE", ComfySocketType::Image, 0, &[12])],
                vec![],
            ),
            node(
                9,
                "SaveImage",
                vec![linked_input("images", ComfySocketType::Image, 12)],
                vec![],
                vec![Value::String(request.output_prefix.clone())],
            ),
        ],
        links,
    )
}

fn build_upscale_workflow(request: &ImageGenerationRequest) -> ComfyWorkflow {
    let links = vec![
        socket_link(1, 1, 0, 3, 0, ComfySocketType::Image),
        socket_link(2, 2, 0, 3, 1, ComfySocketType::UpscaleModel),
        socket_link(3, 3, 0, 4, 0, ComfySocketType::Image),
    ];
    workflow(
        vec![
            node(
                1,
                "LoadImage",
                vec![],
                vec![output("IMAGE", ComfySocketType::Image, 0, &[1])],
                vec![Value::String(
                    request.input_image.clone().unwrap_or_default(),
                )],
            ),
            node(
                2,
                "UpscaleModelLoader",
                vec![],
                vec![output(
                    "UPSCALE_MODEL",
                    ComfySocketType::UpscaleModel,
                    0,
                    &[2],
                )],
                vec![Value::String(request.upscale_model.clone())],
            ),
            node(
                3,
                "ImageUpscaleWithModel",
                vec![
                    linked_input("image", ComfySocketType::Image, 1),
                    linked_input("upscale_model", ComfySocketType::UpscaleModel, 2),
                ],
                vec![output("IMAGE", ComfySocketType::Image, 0, &[3])],
                vec![],
            ),
            node(
                4,
                "SaveImage",
                vec![linked_input("images", ComfySocketType::Image, 3)],
                vec![],
                vec![Value::String(request.output_prefix.clone())],
            ),
        ],
        links,
    )
}

fn build_flux_inpaint_workflow(request: &ImageGenerationRequest) -> ComfyWorkflow {
    let links = vec![
        socket_link(1, 1, 0, 8, 0, ComfySocketType::Image),
        socket_link(2, 2, 0, 8, 2, ComfySocketType::Mask),
        socket_link(3, 3, 0, 6, 0, ComfySocketType::Clip),
        socket_link(4, 3, 0, 7, 0, ComfySocketType::Clip),
        socket_link(5, 4, 0, 9, 0, ComfySocketType::Model),
        socket_link(6, 5, 0, 8, 1, ComfySocketType::Vae),
        socket_link(7, 5, 0, 10, 1, ComfySocketType::Vae),
        socket_link(8, 6, 0, 9, 1, ComfySocketType::Conditioning),
        socket_link(9, 7, 0, 9, 2, ComfySocketType::Conditioning),
        socket_link(10, 8, 0, 9, 3, ComfySocketType::Latent),
        socket_link(11, 9, 0, 10, 0, ComfySocketType::Latent),
        socket_link(12, 10, 0, 11, 0, ComfySocketType::Image),
    ];
    workflow(
        vec![
            node(
                1,
                "LoadImage",
                vec![],
                vec![output("IMAGE", ComfySocketType::Image, 0, &[1])],
                vec![Value::String(
                    request.input_image.clone().unwrap_or_default(),
                )],
            ),
            node(
                2,
                "LoadImageMask",
                vec![],
                vec![output("MASK", ComfySocketType::Mask, 0, &[2])],
                vec![Value::String(
                    request.mask_image.clone().unwrap_or_default(),
                )],
            ),
            node(
                3,
                "DualCLIPLoader",
                vec![],
                vec![output("CLIP", ComfySocketType::Clip, 0, &[3, 4])],
                vec![
                    Value::String("clip_l.safetensors".to_string()),
                    Value::String("t5xxl_fp8_e4m3fn.safetensors".to_string()),
                    Value::String("flux".to_string()),
                ],
            ),
            node(
                4,
                "UNETLoader",
                vec![],
                vec![output("MODEL", ComfySocketType::Model, 0, &[5])],
                vec![
                    Value::String(request.checkpoint.clone()),
                    Value::String("default".to_string()),
                ],
            ),
            node(
                5,
                "VAELoader",
                vec![],
                vec![output("VAE", ComfySocketType::Vae, 0, &[6, 7])],
                vec![Value::String("ae.safetensors".to_string())],
            ),
            node(
                6,
                "CLIPTextEncode",
                vec![linked_input("clip", ComfySocketType::Clip, 3)],
                vec![output(
                    "CONDITIONING",
                    ComfySocketType::Conditioning,
                    0,
                    &[8],
                )],
                vec![Value::String(request.prompt.clone())],
            ),
            node(
                7,
                "CLIPTextEncode",
                vec![linked_input("clip", ComfySocketType::Clip, 4)],
                vec![output(
                    "CONDITIONING",
                    ComfySocketType::Conditioning,
                    0,
                    &[9],
                )],
                vec![Value::String(request.negative_prompt.clone())],
            ),
            node(
                8,
                "VAEEncodeForInpaint",
                vec![
                    linked_input("pixels", ComfySocketType::Image, 1),
                    linked_input("vae", ComfySocketType::Vae, 6),
                    linked_input("mask", ComfySocketType::Mask, 2),
                ],
                vec![output("LATENT", ComfySocketType::Latent, 0, &[10])],
                vec![],
            ),
            node(
                9,
                "KSampler",
                vec![
                    linked_input("model", ComfySocketType::Model, 5),
                    linked_input("positive", ComfySocketType::Conditioning, 8),
                    linked_input("negative", ComfySocketType::Conditioning, 9),
                    linked_input("latent_image", ComfySocketType::Latent, 10),
                ],
                vec![output("LATENT", ComfySocketType::Latent, 0, &[11])],
                sampler_widgets(request, request.denoise),
            ),
            node(
                10,
                "VAEDecode",
                vec![
                    linked_input("samples", ComfySocketType::Latent, 11),
                    linked_input("vae", ComfySocketType::Vae, 7),
                ],
                vec![output("IMAGE", ComfySocketType::Image, 0, &[12])],
                vec![],
            ),
            node(
                11,
                "SaveImage",
                vec![linked_input("images", ComfySocketType::Image, 12)],
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

fn linked_input(name: &str, value_type: ComfySocketType, link: u64) -> WorkflowInput {
    WorkflowInput {
        name: name.to_string(),
        value_type: value_type.to_string(),
        link: Some(link),
        extensions: BTreeMap::new(),
    }
}

fn output(
    name: &str,
    value_type: ComfySocketType,
    slot_index: u64,
    links: &[u64],
) -> WorkflowOutput {
    WorkflowOutput {
        name: name.to_string(),
        value_type: value_type.to_string(),
        slot_index: Some(slot_index),
        links: links.to_vec(),
        extensions: BTreeMap::new(),
    }
}

fn socket_link(
    id: u64,
    origin_id: u64,
    origin_slot: u64,
    target_id: u64,
    target_slot: u64,
    value_type: ComfySocketType,
) -> WorkflowLink {
    WorkflowLink::new(
        id,
        origin_id,
        origin_slot,
        target_id,
        target_slot,
        value_type.to_string(),
    )
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
    fn builds_flux_inpaint_workflow() {
        let workflow = build_generation_workflow(
            &ImageGenerationRequest::new("replace the person with a sculpture")
                .preset(ComfyWorkflowPreset::FluxInpaint)
                .mode(ImageGenerationMode::Inpaint)
                .checkpoint("flux1-dev.safetensors")
                .input_image("input.png")
                .mask_image("mask.png"),
        )
        .unwrap();
        assert!(workflow
            .nodes
            .iter()
            .any(|node| node.node_type == "UNETLoader"));
        assert!(workflow
            .nodes
            .iter()
            .any(|node| node.node_type == "DualCLIPLoader"));
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
