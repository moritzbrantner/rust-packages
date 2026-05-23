#![doc = include_str!("../README.md")]

pub mod surface;
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use comfyui_data::{
    ComfySocketType, ComfyWorkflow, WorkflowInput, WorkflowLink, WorkflowNode, WorkflowOutput,
};
use comfyui_models::{ComfyModelRef, ComfyModelRole};
use serde_json::{json, Value};
use video_analysis_core::{DetectError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// Variants describing image generation mode.
pub enum ImageGenerationMode {
    #[default]
    /// The text to image variant.
    TextToImage,
    /// The image to image variant.
    ImageToImage,
    /// The inpaint variant.
    Inpaint,
    /// The upscale variant.
    Upscale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// Variants describing comfy workflow preset.
pub enum ComfyWorkflowPreset {
    #[default]
    /// The standard stable diffusion variant.
    StandardStableDiffusion,
    /// The flux inpaint variant.
    FluxInpaint,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Options for a ComfyUI HTTP client.
pub struct ComfyUiClientOptions {
    /// Base URL such as `http://127.0.0.1:8188`.
    pub base_url: Option<String>,
    /// Request timeout.
    pub timeout: Duration,
    /// Poll interval used when waiting for output.
    pub poll_interval: Duration,
    /// Whether execution should wait for a completed prompt.
    pub wait_for_output: bool,
}

impl Default for ComfyUiClientOptions {
    fn default() -> Self {
        Self {
            base_url: std::env::var("COMFYUI_URL").ok(),
            timeout: Duration::from_secs(30),
            poll_interval: Duration::from_millis(500),
            wait_for_output: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Result of submitting a workflow to ComfyUI.
pub struct ComfyPromptSubmission {
    /// Prompt identifier returned by ComfyUI.
    pub prompt_id: String,
    /// Raw response JSON from `/prompt`.
    pub response: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Status of a ComfyUI image-edit execution.
pub struct ComfyPromptStatus {
    /// Status string such as `planned`, `submitted`, or `completed`.
    pub status: String,
    /// Output image path, when known.
    pub output_image: Option<String>,
    /// Human-readable message.
    pub message: Option<String>,
    /// Additional metadata.
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
/// Minimal ComfyUI HTTP client.
pub struct ComfyUiClient {
    options: ComfyUiClientOptions,
}

impl ComfyUiClient {
    /// Creates a client.
    pub fn new(options: ComfyUiClientOptions) -> Self {
        Self { options }
    }

    /// Returns options.
    pub fn options(&self) -> &ComfyUiClientOptions {
        &self.options
    }

    /// Submits a workflow to `/prompt`.
    pub fn submit_prompt(&self, workflow: &ComfyWorkflow) -> Result<Option<ComfyPromptSubmission>> {
        let Some(base_url) = normalized_base_url(self.options.base_url.as_deref()) else {
            return Ok(None);
        };
        let body = json!({ "prompt": workflow });
        let response = http_json(
            "POST",
            &format!("{base_url}/prompt"),
            Some(&serde_json::to_vec(&body).map_err(|err| {
                DetectError::Source(format!("failed to encode ComfyUI prompt: {err}"))
            })?),
            self.options.timeout,
        )?;
        let prompt_id = response
            .get("prompt_id")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                DetectError::Source(
                    "ComfyUI /prompt response did not include prompt_id".to_string(),
                )
            })?
            .to_string();
        Ok(Some(ComfyPromptSubmission {
            prompt_id,
            response,
        }))
    }

    /// Polls `/history/{prompt_id}` once and returns raw JSON.
    pub fn prompt_history(&self, prompt_id: &str) -> Result<Option<Value>> {
        let Some(base_url) = normalized_base_url(self.options.base_url.as_deref()) else {
            return Ok(None);
        };
        http_json(
            "GET",
            &format!("{base_url}/history/{prompt_id}"),
            None,
            self.options.timeout,
        )
        .map(Some)
    }
}

impl Default for ComfyUiClient {
    fn default() -> Self {
        Self::new(ComfyUiClientOptions::default())
    }
}

#[derive(Debug, Clone)]
/// Native executor for ComfyUI image-edit workflows.
pub struct ComfyImageEditExecutor {
    client: ComfyUiClient,
}

impl ComfyImageEditExecutor {
    /// Creates a new executor.
    pub fn new(options: ComfyUiClientOptions) -> Self {
        Self {
            client: ComfyUiClient::new(options),
        }
    }

    /// Returns the underlying client.
    pub fn client(&self) -> &ComfyUiClient {
        &self.client
    }

    /// Executes or plans an image-edit workflow.
    pub fn execute(
        &self,
        workflow: &ComfyWorkflow,
        output_image: impl AsRef<Path>,
    ) -> Result<ComfyPromptStatus> {
        let output_image = output_image.as_ref().to_string_lossy().into_owned();
        let Some(submission) = self.client.submit_prompt(workflow)? else {
            return Ok(ComfyPromptStatus {
                status: "planned".to_string(),
                output_image: Some(output_image),
                message: Some("set COMFYUI_URL to execute the workflow".to_string()),
                metadata: BTreeMap::new(),
            });
        };

        let mut metadata = BTreeMap::new();
        metadata.insert("prompt_id".to_string(), submission.prompt_id.clone());
        if !self.client.options.wait_for_output {
            return Ok(ComfyPromptStatus {
                status: "submitted".to_string(),
                output_image: Some(output_image),
                message: Some("workflow submitted to ComfyUI".to_string()),
                metadata,
            });
        }

        let history = poll_history(&self.client, &submission.prompt_id)?;
        if let Some(image_ref) = first_comfy_image_ref(&history) {
            if let Some(base_url) = normalized_base_url(self.client.options.base_url.as_deref()) {
                let bytes = http_bytes(
                    "GET",
                    &format!("{base_url}/view?{}", image_ref.query_string()),
                    None,
                    self.client.options.timeout,
                )?;
                let path = PathBuf::from(&output_image);
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&path, bytes)?;
            }
        }
        Ok(ComfyPromptStatus {
            status: "completed".to_string(),
            output_image: Some(output_image),
            message: Some("workflow completed in ComfyUI".to_string()),
            metadata,
        })
    }
}

fn poll_history(client: &ComfyUiClient, prompt_id: &str) -> Result<Value> {
    let attempts = (client.options.timeout.as_millis()
        / client.options.poll_interval.as_millis().max(1))
    .max(1) as usize;
    for _ in 0..attempts {
        let history = client.prompt_history(prompt_id)?.unwrap_or(Value::Null);
        if history.get(prompt_id).is_some() || non_empty_object(&history) {
            return Ok(history);
        }
        thread::sleep(client.options.poll_interval);
    }
    Err(DetectError::Source(format!(
        "timed out waiting for ComfyUI prompt `{prompt_id}`"
    )))
}

fn non_empty_object(value: &Value) -> bool {
    value.as_object().is_some_and(|object| !object.is_empty())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ComfyImageRef {
    filename: String,
    subfolder: String,
    image_type: String,
}

impl ComfyImageRef {
    fn query_string(&self) -> String {
        format!(
            "filename={}&subfolder={}&type={}",
            url_query_escape(&self.filename),
            url_query_escape(&self.subfolder),
            url_query_escape(&self.image_type)
        )
    }
}

fn first_comfy_image_ref(history: &Value) -> Option<ComfyImageRef> {
    find_comfy_image_ref(history)
}

fn find_comfy_image_ref(value: &Value) -> Option<ComfyImageRef> {
    if let Some(object) = value.as_object() {
        if let Some(filename) = object.get("filename").and_then(Value::as_str) {
            return Some(ComfyImageRef {
                filename: filename.to_string(),
                subfolder: object
                    .get("subfolder")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                image_type: object
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("output")
                    .to_string(),
            });
        }
        for child in object.values() {
            if let Some(found) = find_comfy_image_ref(child) {
                return Some(found);
            }
        }
    }
    if let Some(array) = value.as_array() {
        for child in array {
            if let Some(found) = find_comfy_image_ref(child) {
                return Some(found);
            }
        }
    }
    None
}

fn normalized_base_url(value: Option<&str>) -> Option<String> {
    let value = value?.trim().trim_end_matches('/');
    (!value.is_empty()).then(|| value.to_string())
}

fn http_json(method: &str, url: &str, body: Option<&[u8]>, timeout: Duration) -> Result<Value> {
    let bytes = http_bytes(method, url, body, timeout)?;
    serde_json::from_slice(&bytes)
        .map_err(|err| DetectError::Source(format!("invalid HTTP JSON response: {err}")))
}

fn http_bytes(method: &str, url: &str, body: Option<&[u8]>, timeout: Duration) -> Result<Vec<u8>> {
    let parsed = ParsedHttpUrl::parse(url)?;
    let mut stream = std::net::TcpStream::connect((parsed.host.as_str(), parsed.port))?;
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))?;
    let body = body.unwrap_or_default();
    write!(
        stream,
        "{method} {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nContent-Length: {}\r\nContent-Type: application/json\r\n\r\n",
        parsed.path_and_query,
        parsed.host,
        body.len()
    )?;
    stream.write_all(body)?;
    let mut response = Vec::new();
    let mut chunk = [0_u8; 8192];
    loop {
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(read) => response.extend_from_slice(&chunk[..read]),
            Err(err)
                if err.kind() == std::io::ErrorKind::ConnectionReset && !response.is_empty() =>
            {
                break;
            }
            Err(err) => return Err(err.into()),
        }
    }
    parse_http_response(url, &response)
}

fn parse_http_response(url: &str, response: &[u8]) -> Result<Vec<u8>> {
    let Some(header_end) = response.windows(4).position(|window| window == b"\r\n\r\n") else {
        return Err(DetectError::Source(format!(
            "invalid HTTP response from `{url}`"
        )));
    };
    let headers = String::from_utf8_lossy(&response[..header_end]);
    let mut lines = headers.lines();
    let status = lines.next().unwrap_or_default();
    if !status.contains(" 200 ") {
        return Err(DetectError::Source(format!(
            "HTTP request to `{url}` failed: {status}"
        )));
    }
    Ok(response[header_end + 4..].to_vec())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedHttpUrl {
    host: String,
    port: u16,
    path_and_query: String,
}

impl ParsedHttpUrl {
    fn parse(url: &str) -> Result<Self> {
        let without_scheme = url.strip_prefix("http://").ok_or_else(|| {
            DetectError::InvalidArgument(
                "ComfyUI client currently supports http:// URLs".to_string(),
            )
        })?;
        let (authority, path) = without_scheme
            .split_once('/')
            .map(|(authority, path)| (authority, format!("/{path}")))
            .unwrap_or((without_scheme, "/".to_string()));
        let (host, port) = authority
            .rsplit_once(':')
            .and_then(|(host, port)| port.parse::<u16>().ok().map(|port| (host, port)))
            .unwrap_or((authority, 80));
        if host.is_empty() {
            return Err(DetectError::InvalidArgument(
                "ComfyUI URL host must not be empty".to_string(),
            ));
        }
        Ok(Self {
            host: host.to_string(),
            port,
            path_and_query: path,
        })
    }
}

fn url_query_escape(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![byte as char]
            }
            b' ' => vec!['+'],
            _ => format!("%{byte:02X}").chars().collect(),
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for image generation request.
pub struct ImageGenerationRequest {
    /// The preset value.
    pub preset: ComfyWorkflowPreset,
    /// The mode value.
    pub mode: ImageGenerationMode,
    /// The prompt value.
    pub prompt: String,
    /// The negative prompt value.
    pub negative_prompt: String,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// The steps value.
    pub steps: u32,
    /// The cfg scale value.
    pub cfg_scale: f32,
    /// The sampler name value.
    pub sampler_name: String,
    /// The scheduler value.
    pub scheduler: String,
    /// The seed value.
    pub seed: u64,
    /// The checkpoint value.
    pub checkpoint: ComfyModelRef,
    /// The input image value.
    pub input_image: Option<String>,
    /// The mask image value.
    pub mask_image: Option<String>,
    /// The denoise value.
    pub denoise: f32,
    /// The output prefix value.
    pub output_prefix: String,
    /// The upscale model value.
    pub upscale_model: ComfyModelRef,
}

impl ImageGenerationRequest {
    /// Creates a new value.
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
            checkpoint: ComfyModelRef::new(ComfyModelRole::Checkpoint, "sdxl.safetensors"),
            input_image: None,
            mask_image: None,
            denoise: 0.8,
            output_prefix: "image_analysis".to_string(),
            upscale_model: ComfyModelRef::new(ComfyModelRole::UpscaleModel, "4x-UltraSharp.pth"),
        }
    }

    /// Returns preset.
    pub fn preset(mut self, value: ComfyWorkflowPreset) -> Self {
        self.preset = value;
        self
    }

    /// Returns mode.
    pub fn mode(mut self, value: ImageGenerationMode) -> Self {
        self.mode = value;
        self
    }

    /// Returns negative prompt.
    pub fn negative_prompt(mut self, value: impl Into<String>) -> Self {
        self.negative_prompt = value.into();
        self
    }

    /// Returns size.
    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Returns steps.
    pub fn steps(mut self, value: u32) -> Self {
        self.steps = value;
        self
    }

    /// Returns cfg scale.
    pub fn cfg_scale(mut self, value: f32) -> Self {
        self.cfg_scale = value;
        self
    }

    /// Returns sampler name.
    pub fn sampler_name(mut self, value: impl Into<String>) -> Self {
        self.sampler_name = value.into();
        self
    }

    /// Returns scheduler.
    pub fn scheduler(mut self, value: impl Into<String>) -> Self {
        self.scheduler = value.into();
        self
    }

    /// Returns seed.
    pub fn seed(mut self, value: u64) -> Self {
        self.seed = value;
        self
    }

    /// Returns checkpoint ref.
    pub fn checkpoint_ref(mut self, value: ComfyModelRef) -> Self {
        self.checkpoint = value;
        self
    }

    /// Returns checkpoint.
    pub fn checkpoint(mut self, value: impl Into<String>) -> Self {
        self.checkpoint = ComfyModelRef::new(ComfyModelRole::Checkpoint, value);
        self
    }

    /// Returns input image.
    pub fn input_image(mut self, value: impl Into<String>) -> Self {
        self.input_image = Some(value.into());
        self
    }

    /// Returns mask image.
    pub fn mask_image(mut self, value: impl Into<String>) -> Self {
        self.mask_image = Some(value.into());
        self
    }

    /// Returns denoise.
    pub fn denoise(mut self, value: f32) -> Self {
        self.denoise = value;
        self
    }

    /// Returns output prefix.
    pub fn output_prefix(mut self, value: impl Into<String>) -> Self {
        self.output_prefix = value.into();
        self
    }

    /// Returns upscale model ref.
    pub fn upscale_model_ref(mut self, value: ComfyModelRef) -> Self {
        self.upscale_model = value;
        self
    }

    /// Returns upscale model.
    pub fn upscale_model(mut self, value: impl Into<String>) -> Self {
        self.upscale_model = ComfyModelRef::new(ComfyModelRole::UpscaleModel, value);
        self
    }
}

/// Builds generation workflow.
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
                vec![Value::String(request.checkpoint.name.clone())],
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
                vec![Value::String(request.checkpoint.name.clone())],
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
                vec![Value::String(request.checkpoint.name.clone())],
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
                vec![Value::String(request.upscale_model.name.clone())],
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
                    Value::String(request.checkpoint.name.clone()),
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
    if request.checkpoint.role != ComfyModelRole::Checkpoint {
        return Err(DetectError::InvalidArgument(
            "checkpoint model ref must use the checkpoint role".to_string(),
        ));
    }
    if request.checkpoint.name.trim().is_empty() {
        return Err(DetectError::InvalidArgument(
            "checkpoint model name is required".to_string(),
        ));
    }
    if request.upscale_model.role != ComfyModelRole::UpscaleModel {
        return Err(DetectError::InvalidArgument(
            "upscale_model ref must use the upscale_model role".to_string(),
        ));
    }
    if request.upscale_model.name.trim().is_empty() {
        return Err(DetectError::InvalidArgument(
            "upscale_model name is required".to_string(),
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
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    #[test]
    fn typed_model_ref_builders_match_compatibility_builders() {
        let typed = ImageGenerationRequest::new("fox")
            .checkpoint_ref(ComfyModelRef::new(
                ComfyModelRole::Checkpoint,
                "flux1-dev.safetensors",
            ))
            .upscale_model_ref(ComfyModelRef::new(
                ComfyModelRole::UpscaleModel,
                "4x-UltraSharp.pth",
            ));
        let compatible = ImageGenerationRequest::new("fox")
            .checkpoint("flux1-dev.safetensors")
            .upscale_model("4x-UltraSharp.pth");
        assert_eq!(
            build_generation_workflow(&typed).unwrap(),
            build_generation_workflow(&compatible).unwrap()
        );
    }

    #[test]
    fn rejects_role_mismatches_in_model_refs() {
        let error = build_generation_workflow(&ImageGenerationRequest::new("fox").checkpoint_ref(
            ComfyModelRef::new(ComfyModelRole::UpscaleModel, "4x-UltraSharp.pth"),
        ))
        .unwrap_err();
        assert!(matches!(error, DetectError::InvalidArgument(_)));
    }

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

    #[test]
    fn comfy_executor_plans_without_base_url() {
        let workflow = build_generation_workflow(&ImageGenerationRequest::new("fox")).unwrap();
        let executor = ComfyImageEditExecutor::new(ComfyUiClientOptions {
            base_url: None,
            ..ComfyUiClientOptions::default()
        });
        let status = executor.execute(&workflow, "edited.png").unwrap();
        assert_eq!(status.status, "planned");
        assert_eq!(status.output_image.as_deref(), Some("edited.png"));
    }

    #[test]
    fn comfy_client_posts_prompt_and_returns_prompt_id() {
        let captured = Arc::new(Mutex::new(String::new()));
        let captured_thread = Arc::clone(&captured);
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let text = read_mock_request(&mut stream);
            *captured_thread.lock().unwrap() = text;
            let body = r#"{"prompt_id":"abc123"}"#;
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        });

        let workflow = build_generation_workflow(&ImageGenerationRequest::new("fox")).unwrap();
        let client = ComfyUiClient::new(ComfyUiClientOptions {
            base_url: Some(format!("http://{addr}")),
            timeout: Duration::from_secs(2),
            poll_interval: Duration::from_millis(1),
            wait_for_output: false,
        });
        let submission = client.submit_prompt(&workflow).unwrap().unwrap();
        handle.join().unwrap();
        assert_eq!(submission.prompt_id, "abc123");
        let request = captured.lock().unwrap();
        assert!(request.starts_with("POST /prompt HTTP/1.1"));
        assert!(request.contains("\"prompt\""));
        assert!(request.contains("KSampler"));
    }

    #[test]
    fn comfy_executor_reports_submitted_status() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let _ = read_mock_request(&mut stream);
            let body = r#"{"prompt_id":"submitted-id"}"#;
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        });
        let workflow = build_generation_workflow(&ImageGenerationRequest::new("fox")).unwrap();
        let executor = ComfyImageEditExecutor::new(ComfyUiClientOptions {
            base_url: Some(format!("http://{addr}")),
            timeout: Duration::from_secs(2),
            poll_interval: Duration::from_millis(1),
            wait_for_output: false,
        });
        let status = executor.execute(&workflow, "edited.png").unwrap();
        handle.join().unwrap();
        assert_eq!(status.status, "submitted");
        assert_eq!(status.metadata["prompt_id"], "submitted-id");
    }

    #[test]
    fn comfy_executor_waits_for_history_and_downloads_output_image() {
        let captured_view_request = Arc::new(Mutex::new(String::new()));
        let captured_view_request_thread = Arc::clone(&captured_view_request);
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let _ = read_mock_request(&mut stream);
            let body = r#"{"prompt_id":"wait-id"}"#;
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
            let _ = stream.shutdown(std::net::Shutdown::Both);

            let (mut stream, _) = listener.accept().unwrap();
            let history_request = read_mock_request(&mut stream);
            assert!(history_request.starts_with("GET /history/wait-id HTTP/1.1"));
            let body = r#"{"wait-id":{"outputs":{"9":{"images":[{"filename":"edited image.png","subfolder":"nested output","type":"output"}]}}}}"#;
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
            let _ = stream.shutdown(std::net::Shutdown::Both);

            let (mut stream, _) = listener.accept().unwrap();
            let view_request = read_mock_request(&mut stream);
            *captured_view_request_thread.lock().unwrap() = view_request;
            let body = b"png-bytes";
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n",
                body.len()
            )
            .unwrap();
            stream.write_all(body).unwrap();
            let _ = stream.shutdown(std::net::Shutdown::Both);
        });

        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("downloads/edited.png");
        let workflow = build_generation_workflow(&ImageGenerationRequest::new("fox")).unwrap();
        let executor = ComfyImageEditExecutor::new(ComfyUiClientOptions {
            base_url: Some(format!("http://{addr}/")),
            timeout: Duration::from_secs(2),
            poll_interval: Duration::from_millis(1),
            wait_for_output: true,
        });
        let status = executor.execute(&workflow, &output).unwrap();
        handle.join().unwrap();

        assert_eq!(status.status, "completed");
        assert_eq!(status.metadata["prompt_id"], "wait-id");
        assert_eq!(std::fs::read(&output).unwrap(), b"png-bytes");
        let view_request = captured_view_request.lock().unwrap();
        assert!(view_request.starts_with(
            "GET /view?filename=edited+image.png&subfolder=nested+output&type=output HTTP/1.1"
        ));
    }

    #[test]
    fn rejects_missing_mode_inputs_and_invalid_numeric_options() {
        let missing_input = build_generation_workflow(
            &ImageGenerationRequest::new("stylize").mode(ImageGenerationMode::ImageToImage),
        )
        .unwrap_err();
        assert!(matches!(missing_input, DetectError::InvalidArgument(_)));

        let missing_mask = build_generation_workflow(
            &ImageGenerationRequest::new("repair")
                .mode(ImageGenerationMode::Inpaint)
                .input_image("input.png"),
        )
        .unwrap_err();
        assert!(matches!(missing_mask, DetectError::InvalidArgument(_)));

        let invalid_dimensions =
            build_generation_workflow(&ImageGenerationRequest::new("fox").size(0, 1024))
                .unwrap_err();
        assert!(matches!(
            invalid_dimensions,
            DetectError::InvalidDimensions {
                width: 0,
                height: 1024
            }
        ));

        let invalid_denoise =
            build_generation_workflow(&ImageGenerationRequest::new("fox").denoise(1.1))
                .unwrap_err();
        assert!(matches!(invalid_denoise, DetectError::InvalidArgument(_)));
    }

    fn read_mock_request(stream: &mut std::net::TcpStream) -> String {
        stream
            .set_read_timeout(Some(Duration::from_millis(200)))
            .unwrap();
        let mut bytes = Vec::new();
        let mut chunk = [0_u8; 4096];
        loop {
            match stream.read(&mut chunk) {
                Ok(0) => break,
                Ok(read) => {
                    bytes.extend_from_slice(&chunk[..read]);
                    if request_is_complete(&bytes) {
                        break;
                    }
                }
                Err(err)
                    if matches!(
                        err.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) =>
                {
                    break;
                }
                Err(err) => panic!("failed to read mock request: {err}"),
            }
        }
        String::from_utf8_lossy(&bytes).to_string()
    }

    fn request_is_complete(bytes: &[u8]) -> bool {
        let Some(header_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") else {
            return false;
        };
        let headers = String::from_utf8_lossy(&bytes[..header_end]);
        let content_length = headers
            .lines()
            .find_map(|line| line.strip_prefix("Content-Length: "))
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0);
        bytes.len() >= header_end + 4 + content_length
    }
}
