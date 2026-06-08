//! Internal module support for image person edit.

use std::collections::BTreeMap;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use comfyui_latents::{LatentImageSize, LatentMask};
use comfyui_models::{ComfyModelRef, ComfyModelRole};
use image_analysis_comfyui::{
    build_generation_workflow, ComfyImageEditExecutor, ComfyUiClientOptions, ComfyWorkflowPreset,
    ImageGenerationMode, ImageGenerationRequest,
};
use image_analysis_core::{mask_tensor_from_luma, ImagePixelFormat, OwnedImage};
use image_analysis_io::{read_image, write_image};
use model_runtime::{DownloadedModel, HuggingFaceModelSpec, ModelBundle, ModelTask};
use serde::{Deserialize, Serialize};
use video_analysis_core::{
    BoundingBox, DetectError, FramePosition, PixelFormat, Result, Timebase, Timestamp,
};
use video_analysis_recognition::{
    normalize_predictions, ExternalCommandModel, PredictionRepairOptions, VisionModelBackend,
};

use crate::workflow_support::{display_path, validate_local_file, write_json_report};
use crate::{CapabilityReport, ExternalCommandConfig, RegionReport, IMAGE_PERSON_EDIT_USE_CASE};

#[derive(Debug, Clone)]
/// Data type for image person edit request.
pub struct ImagePersonEditRequest {
    /// The input value.
    pub input: PathBuf,
    /// The work dir value.
    pub work_dir: PathBuf,
    /// The output value.
    pub output: Option<PathBuf>,
    /// The prompt value.
    pub prompt: String,
    /// The negative prompt value.
    pub negative_prompt: String,
    /// The model value.
    pub model: String,
    /// The person detector command value.
    pub person_detector_command: PathBuf,
    /// The person detector args value.
    pub person_detector_args: Vec<String>,
    /// The editor command value.
    pub editor_command: Option<PathBuf>,
    /// The editor args value.
    pub editor_args: Vec<String>,
    /// Native or external person detector configuration.
    pub person_detector: PersonDetectorConfig,
    /// ComfyUI client options used when editor_command is not set.
    pub comfyui: ComfyUiClientOptions,
}

impl Default for ImagePersonEditRequest {
    fn default() -> Self {
        Self {
            input: PathBuf::from("input.png"),
            work_dir: PathBuf::from("use-case-output/image-person-edit"),
            output: None,
            prompt: "replace the detected person".to_string(),
            negative_prompt: String::new(),
            model: "flux1-dev.safetensors".to_string(),
            person_detector_command: PathBuf::from("python3"),
            person_detector_args: Vec::new(),
            editor_command: None,
            editor_args: Vec::new(),
            person_detector: PersonDetectorConfig::default(),
            comfyui: ComfyUiClientOptions::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for image person edit run request.
pub struct ImagePersonEditRunRequest {
    /// The input value.
    pub input: PathBuf,
    /// The work dir value.
    pub work_dir: Option<PathBuf>,
    /// The prompt value.
    pub prompt: String,
    #[serde(default)]
    /// The negative prompt value.
    pub negative_prompt: String,
    /// The model value.
    pub model: String,
    /// The person detector value.
    pub person_detector: ExternalCommandConfig,
    /// The editor value.
    pub editor: Option<ExternalCommandConfig>,
}

impl ImagePersonEditRunRequest {
    /// Validates this value.
    pub fn validate(&self) -> Result<()> {
        validate_local_file(&self.input)?;
        if self.prompt.trim().is_empty() {
            return Err(DetectError::InvalidArgument(
                "prompt is required".to_string(),
            ));
        }
        if self.model.trim().is_empty() {
            return Err(DetectError::InvalidArgument(
                "model is required".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for image person edit source report.
pub struct ImagePersonEditSourceReport {
    /// The local image value.
    pub local_image: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for image person edit asset report.
pub struct ImagePersonEditAssetReport {
    /// The work dir value.
    pub work_dir: String,
    /// The report path value.
    pub report_path: String,
    /// The person mask value.
    pub person_mask: String,
    /// The workflow JSON value.
    pub workflow_json: String,
    /// The edited image value.
    pub edited_image: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for person detection report.
pub struct PersonDetectionReport {
    /// Label assigned to this value.
    pub label: String,
    /// Score assigned to this value.
    pub score: Option<f32>,
    /// The region value.
    pub region: RegionReport,
    /// The attributes value.
    pub attributes: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for image edit execution report.
pub struct ImageEditExecutionReport {
    /// The status value.
    pub status: String,
    /// The output image value.
    pub output_image: Option<String>,
    /// The message value.
    pub message: Option<String>,
    #[serde(default)]
    /// Metadata associated with this value.
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
/// Detector selection for image person edit.
pub enum PersonDetectorConfig {
    /// Use an ONNX object detector loaded from a model bundle directory.
    Onnx {
        /// Model bundle directory or manifest path.
        bundle_dir: PathBuf,
    },
    /// Use the legacy external-command JSON protocol.
    ExternalCommand {
        /// Command path.
        command: PathBuf,
        /// Command arguments.
        args: Vec<String>,
    },
    /// Test-only deterministic detections.
    Stub {
        /// Detections to return.
        detections: Vec<PersonDetectionReport>,
    },
}

impl Default for PersonDetectorConfig {
    fn default() -> Self {
        Self::ExternalCommand {
            command: PathBuf::new(),
            args: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for image person edit report.
pub struct ImagePersonEditReport {
    #[serde(alias = "use_case")]
    /// The workflow value.
    pub workflow: String,
    /// The source value.
    pub source: ImagePersonEditSourceReport,
    /// The assets value.
    pub assets: ImagePersonEditAssetReport,
    /// The capabilities value.
    pub capabilities: CapabilityReport,
    /// The detections value.
    pub detections: Vec<PersonDetectionReport>,
    /// The editing value.
    pub editing: ImageEditExecutionReport,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for image edit command request.
pub struct ImageEditCommandRequest {
    /// The prompt value.
    pub prompt: String,
    /// The negative prompt value.
    pub negative_prompt: String,
    /// The model value.
    pub model: String,
    /// The input image value.
    pub input_image: String,
    /// The mask image value.
    pub mask_image: String,
    /// The output image value.
    pub output_image: String,
    /// The workflow path value.
    pub workflow_path: String,
    /// The workflow value.
    pub workflow: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for image edit command response.
pub struct ImageEditCommandResponse {
    /// The status value.
    pub status: String,
    /// The output image value.
    pub output_image: Option<String>,
    /// The message value.
    pub message: Option<String>,
    #[serde(default)]
    /// Metadata associated with this value.
    pub metadata: BTreeMap<String, String>,
}

/// Runs image person edit.
pub fn run_image_person_edit(args: ImagePersonEditRequest) -> Result<ImagePersonEditReport> {
    validate_local_file(&args.input)?;
    if args.prompt.trim().is_empty() {
        return Err(DetectError::InvalidArgument(
            "prompt is required".to_string(),
        ));
    }
    if args.model.trim().is_empty() {
        return Err(DetectError::InvalidArgument(
            "model is required".to_string(),
        ));
    }
    std::fs::create_dir_all(&args.work_dir)?;

    let report_path = args
        .output
        .clone()
        .unwrap_or_else(|| args.work_dir.join("analysis.json"));
    let edited_image_path = args.work_dir.join("edited.png");
    let mask_path = args.work_dir.join("person_mask.png");
    let workflow_path = args.work_dir.join("edit-workflow.json");

    let image = read_image(&args.input)?;
    let frame = owned_image_to_video_frame(&image)?;
    let detector_config = resolve_person_detector_config(&args);
    let detections = detect_people(&frame, &detector_config)?;
    if detections.is_empty() {
        return Err(DetectError::Source(
            "person detection did not return any persons".to_string(),
        ));
    }

    let mask = person_mask(&image, &detections);
    write_image(&mask_path, &mask)?;
    let _latent_mask = LatentMask::from_image_mask(
        mask_tensor_from_luma(&mask.as_view())?,
        LatentImageSize::new(mask.width, mask.height)?,
    )?;
    let checkpoint = ComfyModelRef::new(ComfyModelRole::Checkpoint, args.model.clone());

    let workflow = build_generation_workflow(
        &ImageGenerationRequest::new(&args.prompt)
            .preset(ComfyWorkflowPreset::FluxInpaint)
            .mode(ImageGenerationMode::Inpaint)
            .negative_prompt(&args.negative_prompt)
            .checkpoint_ref(checkpoint)
            .input_image(display_path(&args.input))
            .mask_image(display_path(&mask_path))
            .output_prefix("edited"),
    )?;
    write_json_report(&workflow_path, &workflow)?;

    let editing = if let Some(command) = &args.editor_command {
        execute_editor(
            command,
            &args.editor_args,
            ImageEditCommandRequest {
                prompt: args.prompt.clone(),
                negative_prompt: args.negative_prompt.clone(),
                model: args.model.clone(),
                input_image: display_path(&args.input),
                mask_image: display_path(&mask_path),
                output_image: display_path(&edited_image_path),
                workflow_path: display_path(&workflow_path),
                workflow: serde_json::to_value(&workflow).map_err(|err| {
                    DetectError::Source(format!("failed to encode workflow request: {err}"))
                })?,
            },
        )?
    } else {
        let status = ComfyImageEditExecutor::new(args.comfyui.clone())
            .execute(&workflow, &edited_image_path)?;
        ImageEditExecutionReport {
            status: status.status,
            output_image: status.output_image,
            message: status.message,
            metadata: status.metadata,
        }
    };

    Ok(ImagePersonEditReport {
        workflow: IMAGE_PERSON_EDIT_USE_CASE.to_string(),
        source: ImagePersonEditSourceReport {
            local_image: display_path(&args.input),
        },
        assets: ImagePersonEditAssetReport {
            work_dir: display_path(&args.work_dir),
            report_path: display_path(&report_path),
            person_mask: display_path(&mask_path),
            workflow_json: display_path(&workflow_path),
            edited_image: display_path(&edited_image_path),
        },
        capabilities: CapabilityReport {
            completed: vec![
                "person_detection".to_string(),
                "mask_generation".to_string(),
                "workflow_generation".to_string(),
            ],
            skipped: if args.editor_command.is_some() || args.comfyui.base_url.is_some() {
                Vec::new()
            } else {
                vec!["editor execution: set COMFYUI_URL or pass --editor-command".to_string()]
            },
        },
        detections,
        editing,
    })
}

/// Runs image person edit workflow.
pub fn run_image_person_edit_workflow(
    request: ImagePersonEditRunRequest,
    work_dir: PathBuf,
    report_path: PathBuf,
) -> Result<ImagePersonEditReport> {
    request.validate()?;
    let report = run_image_person_edit(ImagePersonEditRequest {
        input: request.input,
        work_dir,
        output: Some(report_path.clone()),
        prompt: request.prompt,
        negative_prompt: request.negative_prompt,
        model: request.model,
        person_detector_command: request.person_detector.command,
        person_detector_args: request.person_detector.args,
        editor_command: request.editor.as_ref().map(|config| config.command.clone()),
        editor_args: request.editor.map(|config| config.args).unwrap_or_default(),
        person_detector: PersonDetectorConfig::default(),
        comfyui: ComfyUiClientOptions::default(),
    })?;
    write_image_person_edit_report(&report_path, &report)?;
    Ok(report)
}

/// Writes image person edit report.
pub fn write_image_person_edit_report(path: &Path, report: &ImagePersonEditReport) -> Result<()> {
    write_json_report(path, report)
}

fn resolve_person_detector_config(args: &ImagePersonEditRequest) -> PersonDetectorConfig {
    match &args.person_detector {
        PersonDetectorConfig::ExternalCommand {
            command,
            args: command_args,
        } if command.as_os_str().is_empty()
            && !args.person_detector_command.as_os_str().is_empty() =>
        {
            PersonDetectorConfig::ExternalCommand {
                command: args.person_detector_command.clone(),
                args: args.person_detector_args.clone(),
            }
        }
        other => other.clone(),
    }
}

fn detect_people(
    frame: &video_analysis_core::OwnedVideoFrame,
    config: &PersonDetectorConfig,
) -> Result<Vec<PersonDetectionReport>> {
    let raw = match config {
        PersonDetectorConfig::Onnx { bundle_dir } => {
            let bundle = ModelBundle::load(bundle_dir).map_err(model_runtime_error)?;
            let mut detector = video_analysis_recognition::OnnxObjectDetector::from_bundle(bundle)?;
            detector.predict_frame(&frame.as_frame())?
        }
        PersonDetectorConfig::ExternalCommand { command, args } => {
            if command.as_os_str().is_empty() {
                return Err(DetectError::InvalidArgument(
                    "person detector requires an ONNX bundle, stub detections, or external command"
                        .to_string(),
                ));
            }
            let mut detector = ExternalCommandModel::new(
                command,
                DownloadedModel {
                    spec: HuggingFaceModelSpec::new("person-detector", ModelTask::ObjectDetection)
                        .name("person-detector"),
                    files: BTreeMap::new(),
                },
            )
            .args(args.iter().cloned());
            detector.predict_frame(&frame.as_frame())?
        }
        PersonDetectorConfig::Stub { detections } => return Ok(detections.clone()),
    };
    Ok(normalize_predictions(
        raw,
        &ModelTask::ObjectDetection,
        Some((frame.width, frame.height)),
        PredictionRepairOptions::default(),
    )
    .into_iter()
    .filter_map(|prediction| {
        let label = prediction.label?;
        if label != "person" {
            return None;
        }
        Some(PersonDetectionReport {
            label,
            score: prediction.score,
            region: region_report(prediction.region?),
            attributes: prediction.attributes,
        })
    })
    .collect())
}

fn model_runtime_error(error: model_runtime::ModelRuntimeError) -> DetectError {
    match error {
        model_runtime::ModelRuntimeError::InvalidArgument(message) => {
            DetectError::InvalidArgument(message)
        }
        model_runtime::ModelRuntimeError::Source(message) => DetectError::Source(message),
        model_runtime::ModelRuntimeError::Io(error) => DetectError::Io(error),
    }
}

fn owned_image_to_video_frame(image: &OwnedImage) -> Result<video_analysis_core::OwnedVideoFrame> {
    let data = match image.pixel_format {
        ImagePixelFormat::Rgb24 => image.data.clone(),
        ImagePixelFormat::Bgr24 => image
            .data
            .chunks_exact(3)
            .flat_map(|chunk| [chunk[2], chunk[1], chunk[0]])
            .collect(),
        ImagePixelFormat::Gray8 => image
            .data
            .iter()
            .flat_map(|value| [*value, *value, *value])
            .collect(),
    };
    Ok(video_analysis_core::OwnedVideoFrame {
        position: FramePosition {
            frame_index: 0,
            timestamp: Timestamp::new(0, Timebase::new(1, 1)),
        },
        width: image.width,
        height: image.height,
        pixel_format: PixelFormat::Rgb24,
        data,
        stride: image.width as usize * 3,
    })
}

fn person_mask(image: &OwnedImage, detections: &[PersonDetectionReport]) -> OwnedImage {
    let mut mask = vec![0_u8; image.width as usize * image.height as usize];
    for detection in detections {
        let expanded = expand_region(detection.region.clone(), image.width, image.height, 8);
        for y in expanded.y..expanded.y + expanded.height {
            for x in expanded.x..expanded.x + expanded.width {
                mask[y as usize * image.width as usize + x as usize] = 255;
            }
        }
    }
    OwnedImage::new_gray(image.width, image.height, mask).expect("mask dimensions are valid")
}

fn expand_region(region: RegionReport, width: u32, height: u32, padding: u32) -> RegionReport {
    let x0 = region.x.saturating_sub(padding);
    let y0 = region.y.saturating_sub(padding);
    let x1 = region
        .x
        .saturating_add(region.width)
        .saturating_add(padding)
        .min(width);
    let y1 = region
        .y
        .saturating_add(region.height)
        .saturating_add(padding)
        .min(height);
    RegionReport {
        x: x0,
        y: y0,
        width: x1.saturating_sub(x0),
        height: y1.saturating_sub(y0),
    }
}

fn execute_editor(
    command: &Path,
    args: &[String],
    request: ImageEditCommandRequest,
) -> Result<ImageEditExecutionReport> {
    let payload = serde_json::to_vec(&request)
        .map_err(|err| DetectError::Source(format!("failed to encode editor request: {err}")))?;
    let mut child = Command::new(command)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| DetectError::Source("editor stdin unavailable".to_string()))?;
    let write_result = stdin.write_all(&payload);
    drop(stdin);
    let output = child.wait_with_output()?;
    if let Err(err) = write_result {
        if err.kind() != io::ErrorKind::BrokenPipe {
            return Err(err.into());
        }
    }
    if !output.status.success() {
        return Err(DetectError::Source(format!(
            "editor command `{}` failed: {}",
            command.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let response: ImageEditCommandResponse = serde_json::from_slice(&output.stdout)
        .map_err(|err| DetectError::Source(format!("invalid editor response: {err}")))?;
    Ok(ImageEditExecutionReport {
        status: response.status,
        output_image: response.output_image,
        message: response.message,
        metadata: response.metadata,
    })
}

fn region_report(region: BoundingBox) -> RegionReport {
    RegionReport {
        x: region.x,
        y: region.y,
        width: region.width,
        height: region.height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_image() -> OwnedImage {
        OwnedImage::new_rgb(32, 32, vec![0; 32 * 32 * 3]).unwrap()
    }

    #[test]
    fn image_person_edit_unions_person_boxes_into_mask() {
        let mask = person_mask(
            &test_image(),
            &[
                PersonDetectionReport {
                    label: "person".to_string(),
                    score: Some(0.9),
                    region: RegionReport {
                        x: 10,
                        y: 10,
                        width: 4,
                        height: 4,
                    },
                    attributes: BTreeMap::new(),
                },
                PersonDetectionReport {
                    label: "person".to_string(),
                    score: Some(0.8),
                    region: RegionReport {
                        x: 16,
                        y: 10,
                        width: 4,
                        height: 4,
                    },
                    attributes: BTreeMap::new(),
                },
            ],
        );
        assert_eq!(mask.pixel_format, ImagePixelFormat::Gray8);
        assert!(mask.data.contains(&255));
    }

    #[test]
    fn image_person_edit_writes_flux_workflow_with_prompt_model_and_mask() {
        let workflow = build_generation_workflow(
            &ImageGenerationRequest::new("replace the person with a robot")
                .preset(ComfyWorkflowPreset::FluxInpaint)
                .mode(ImageGenerationMode::Inpaint)
                .checkpoint("flux1-dev.safetensors")
                .input_image("input.png")
                .mask_image("person_mask.png"),
        )
        .unwrap();
        let json = serde_json::to_string(&workflow).unwrap();
        assert!(json.contains("UNETLoader"));
        assert!(json.contains("flux1-dev.safetensors"));
        assert!(json.contains("person_mask.png"));
    }

    #[test]
    fn image_person_edit_converts_person_mask_into_latent_mask() {
        let mask = person_mask(
            &test_image(),
            &[PersonDetectionReport {
                label: "person".to_string(),
                score: Some(0.9),
                region: RegionReport {
                    x: 8,
                    y: 8,
                    width: 8,
                    height: 8,
                },
                attributes: BTreeMap::new(),
            }],
        );
        let latent_mask = LatentMask::from_image_mask(
            mask_tensor_from_luma(&mask.as_view()).unwrap(),
            LatentImageSize::new(mask.width, mask.height).unwrap(),
        )
        .unwrap();
        assert_eq!(latent_mask.tensor().shape().dimensions(), &[4, 4]);
        assert!(latent_mask
            .tensor()
            .values()
            .iter()
            .any(|value| *value > 0.0));
    }

    #[test]
    fn report_roundtrips_for_each_new_use_case() {
        let report = ImagePersonEditReport {
            workflow: IMAGE_PERSON_EDIT_USE_CASE.to_string(),
            source: ImagePersonEditSourceReport {
                local_image: "input.png".to_string(),
            },
            assets: ImagePersonEditAssetReport {
                work_dir: "work".to_string(),
                report_path: "analysis.json".to_string(),
                person_mask: "person_mask.png".to_string(),
                workflow_json: "edit-workflow.json".to_string(),
                edited_image: "edited.png".to_string(),
            },
            capabilities: CapabilityReport {
                completed: vec!["person_detection".to_string()],
                skipped: Vec::new(),
            },
            detections: vec![PersonDetectionReport {
                label: "person".to_string(),
                score: Some(0.9),
                region: RegionReport {
                    x: 1,
                    y: 2,
                    width: 3,
                    height: 4,
                },
                attributes: BTreeMap::new(),
            }],
            editing: ImageEditExecutionReport {
                status: "planned".to_string(),
                output_image: Some("edited.png".to_string()),
                message: Some("planned".to_string()),
                metadata: BTreeMap::new(),
            },
        };
        let value = serde_json::to_vec(&report).unwrap();
        let decoded: ImagePersonEditReport = serde_json::from_slice(&value).unwrap();
        assert_eq!(decoded, report);
    }
}
