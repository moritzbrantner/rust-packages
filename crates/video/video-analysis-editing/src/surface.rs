//! Library-owned runtime surface for `video-analysis-editing`.

use serde::Deserialize;
use video_analysis_core::runtime::{
    OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation, SurfaceRequest,
    SurfaceResponse,
};
use video_analysis_core::{FramePosition, PixelFormat, Timebase, Timestamp, VideoFrame};

use crate::{
    build_concat_plan, build_cut_plan, build_subtitle_plan, FrameEdit, FrameEditor,
    SubtitleOverlay, TimeSpan, TimelineClip, TransitionKind, TransitionSpec,
};

/// Returns the package surface exposed by every transport wrapper.
pub fn package_surface() -> PackageSurface {
    PackageSurface {
        library: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: RuntimeCapabilities::pure_rust(),
        operations: vec![
            operation(
                "describe",
                "Describe package",
                "Deterministic edit decision contracts for video-analysis outputs.",
                serde_json::json!({
                    "includeOperations": true
                }),
            ),
            operation(
                "video.editing.cutPlan",
                "Plan cuts",
                "Builds a deterministic edit decision list from scene and cut intervals.",
                serde_json::json!({
                    "input": {},
                    "mode": "deterministic"
                }),
            ),
            operation(
                "video.editing.concatPlan",
                "Plan concat",
                "Describes segment ordering, transitions, and output stream compatibility for concatenation.",
                serde_json::json!({
                    "input": {},
                    "mode": "deterministic"
                }),
            ),
            operation(
                "video.editing.subtitlePlan",
                "Plan subtitles",
                "Builds a subtitle overlay plan from transcript and scene timing records.",
                serde_json::json!({
                    "input": {},
                    "mode": "deterministic"
                }),
            ),
            operation(
                "video.editing.frameApply",
                "Apply frame edit",
                "Applies an in-memory deterministic frame edit pipeline and returns a capped output preview.",
                serde_json::json!({
                    "frame": sample_frame_json(),
                    "edits": [{"type": "flipHorizontal"}, {"type": "fadeToColor", "color": [0, 0, 0], "amount": 0.5}]
                }),
            ),
        ],
    }
}

fn operation(
    id: &str,
    name: &str,
    description: &str,
    example_request: serde_json::Value,
) -> SurfaceOperation {
    SurfaceOperation {
        id: OperationId::new(id),
        name: name.to_string(),
        description: Some(description.to_string()),
        input_schema: serde_json::json!({"type": "object", "additionalProperties": true}),
        output_schema: serde_json::json!({"type": "object"}),
        example_request,
        wasm_supported: true,
        server_supported: true,
    }
}

/// Runs one library-owned operation.
pub fn run_surface_operation(request: SurfaceRequest) -> Result<SurfaceResponse, String> {
    let surface = package_surface();
    let operation = request.operation.clone();
    let Some(surface_operation) = surface
        .operations
        .iter()
        .find(|candidate| candidate.id.as_str() == operation.as_str())
    else {
        return Err(format!(
            "unsupported operation `{}` for {}",
            operation.as_str(),
            env!("CARGO_PKG_NAME")
        ));
    };

    let value = match operation.as_str() {
        "describe" => describe_value(&surface, request.input),
        "video.editing.cutPlan" => cut_plan_value(request.input)?,
        "video.editing.concatPlan" => concat_plan_value(request.input)?,
        "video.editing.subtitlePlan" => subtitle_plan_value(request.input)?,
        "video.editing.frameApply" => frame_apply_value(request.input)?,
        _ => deterministic_operation_value(&surface, surface_operation, request.input),
    };

    Ok(SurfaceResponse {
        operation,
        value,
        diagnostics: Vec::new(),
        artifacts: Vec::new(),
    })
}

fn describe_value(surface: &PackageSurface, input: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "library": &surface.library,
        "version": &surface.version,
        "operationCount": surface.operations.len(),
        "operations": surface
            .operations
            .iter()
            .map(|operation| operation.id.as_str())
            .collect::<Vec<_>>(),
        "input": input
    })
}

fn deterministic_operation_value(
    surface: &PackageSurface,
    operation: &SurfaceOperation,
    input: serde_json::Value,
) -> serde_json::Value {
    let operation_id = operation.id.as_str();
    serde_json::json!({
        "library": &surface.library,
        "version": &surface.version,
        "operation": operation_id,
        "name": &operation.name,
        "description": &operation.description,
        "deterministic": true,
        "externalToolsRequired": false,
        "request": input,
        "plan": {
            "accepts": "JSON request metadata for the operation-specific package surface",
            "produces": "A deterministic summary or execution plan owned by the Rust library",
            "operationFamily": operation_family(operation_id),
        }
    })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CutPlanRequest {
    source_id: Option<String>,
    intervals: Option<Vec<TimeSpanRequest>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConcatPlanRequest {
    clips: Option<Vec<TimelineClipRequest>>,
    transitions: Option<Vec<TransitionRequest>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SubtitlePlanRequest {
    clips: Option<Vec<TimelineClipRequest>>,
    subtitles: Option<Vec<SubtitleRequest>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TimeSpanRequest {
    start_seconds: f64,
    end_seconds: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TimelineClipRequest {
    source_id: String,
    source_start_seconds: f64,
    source_end_seconds: f64,
    timeline_start_seconds: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TransitionRequest {
    from_clip: usize,
    to_clip: usize,
    duration_seconds: f64,
    kind: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SubtitleRequest {
    start_seconds: f64,
    end_seconds: f64,
    text: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FrameApplyRequest {
    frame: FramePayload,
    edits: Vec<FrameEditRequest>,
    preview_limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FramePayload {
    width: u32,
    height: u32,
    pixel_format: String,
    stride: Option<usize>,
    data: Vec<u8>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FrameEditRequest {
    #[serde(rename = "type")]
    kind: String,
    x: Option<u32>,
    y: Option<u32>,
    width: Option<u32>,
    height: Option<u32>,
    radius: Option<u32>,
    brightness: Option<i16>,
    contrast: Option<f32>,
    clockwise_turns: Option<u8>,
    color: Option<[u8; 3]>,
    amount: Option<f32>,
}

fn cut_plan_value(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let request: CutPlanRequest =
        serde_json::from_value(input).map_err(|error| format!("invalid request: {error}"))?;
    let intervals = request
        .intervals
        .unwrap_or_default()
        .into_iter()
        .map(|span| TimeSpan::new(span.start_seconds, span.end_seconds))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    let edl = build_cut_plan(
        request.source_id.unwrap_or_else(|| "source".to_string()),
        &intervals,
    )
    .map_err(|error| error.to_string())?;
    Ok(edl_value(&edl))
}

fn concat_plan_value(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let request: ConcatPlanRequest =
        serde_json::from_value(input).map_err(|error| format!("invalid request: {error}"))?;
    let clips = parse_clips(request.clips.unwrap_or_default())?;
    let transitions = request
        .transitions
        .unwrap_or_default()
        .into_iter()
        .map(parse_transition)
        .collect::<Result<Vec<_>, _>>()?;
    let edl = build_concat_plan(clips, transitions).map_err(|error| error.to_string())?;
    Ok(edl_value(&edl))
}

fn subtitle_plan_value(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let request: SubtitlePlanRequest =
        serde_json::from_value(input).map_err(|error| format!("invalid request: {error}"))?;
    let clips = parse_clips(request.clips.unwrap_or_default())?;
    let subtitles = request
        .subtitles
        .unwrap_or_default()
        .into_iter()
        .map(|subtitle| {
            SubtitleOverlay::new(subtitle.start_seconds, subtitle.end_seconds, subtitle.text)
                .map_err(|error| error.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let edl = build_subtitle_plan(clips, subtitles).map_err(|error| error.to_string())?;
    Ok(edl_value(&edl))
}

fn frame_apply_value(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let request: FrameApplyRequest =
        serde_json::from_value(input).map_err(|error| format!("invalid request: {error}"))?;
    let frame = request.frame.frame()?;
    let mut editor = FrameEditor::new();
    for edit in request.edits {
        editor = editor.edit(edit.edit()?);
    }
    let output = editor.apply(&frame).map_err(|error| error.to_string())?;
    let preview_limit = request.preview_limit.unwrap_or(32).min(512);
    Ok(serde_json::json!({
        "deterministic": true,
        "externalToolsRequired": false,
        "width": output.width,
        "height": output.height,
        "pixelFormat": pixel_format_name(output.pixel_format),
        "stride": output.stride,
        "dataLength": output.data.len(),
        "dataPreview": output.data.iter().take(preview_limit).copied().collect::<Vec<_>>(),
        "truncated": output.data.len() > preview_limit
    }))
}

fn parse_clips(requests: Vec<TimelineClipRequest>) -> Result<Vec<TimelineClip>, String> {
    requests
        .into_iter()
        .map(|clip| {
            TimelineClip::new(
                clip.source_id,
                clip.source_start_seconds,
                clip.source_end_seconds,
                clip.timeline_start_seconds,
            )
            .map_err(|error| error.to_string())
        })
        .collect()
}

fn parse_transition(request: TransitionRequest) -> Result<TransitionSpec, String> {
    TransitionSpec::new(
        request.from_clip,
        request.to_clip,
        request.duration_seconds,
        transition_kind(&request.kind)?,
    )
    .map_err(|error| error.to_string())
}

fn transition_kind(value: &str) -> Result<TransitionKind, String> {
    match value {
        "cut" => Ok(TransitionKind::Cut),
        "crossFade" | "cross_fade" => Ok(TransitionKind::CrossFade),
        other => Err(format!("unsupported transition kind `{other}`")),
    }
}

fn edl_value(edl: &crate::EditDecisionList) -> serde_json::Value {
    serde_json::json!({
        "deterministic": true,
        "externalToolsRequired": false,
        "clipCount": edl.clips.len(),
        "transitionCount": edl.transitions.len(),
        "subtitleCount": edl.subtitles.len(),
        "durationSeconds": edl.duration_seconds(),
        "clips": edl.clips.iter().map(|clip| serde_json::json!({
            "sourceId": clip.source_id,
            "sourceStartSeconds": clip.source_start_seconds,
            "sourceEndSeconds": clip.source_end_seconds,
            "timelineStartSeconds": clip.timeline_start_seconds,
            "timelineEndSeconds": clip.timeline_end_seconds()
        })).collect::<Vec<_>>(),
        "transitions": edl.transitions.iter().map(|transition| serde_json::json!({
            "fromClip": transition.from_clip,
            "toClip": transition.to_clip,
            "durationSeconds": transition.duration_seconds,
            "kind": match transition.kind {
                TransitionKind::Cut => "cut",
                TransitionKind::CrossFade => "crossFade",
            }
        })).collect::<Vec<_>>(),
        "subtitles": edl.subtitles.iter().map(|subtitle| serde_json::json!({
            "startSeconds": subtitle.start_seconds,
            "endSeconds": subtitle.end_seconds,
            "text": subtitle.text
        })).collect::<Vec<_>>()
    })
}

impl FramePayload {
    fn frame(&self) -> Result<VideoFrame<'_>, String> {
        let pixel_format = parse_pixel_format(&self.pixel_format)?;
        let stride = self.stride.unwrap_or(self.width as usize * 3);
        if stride < self.width as usize * 3 {
            return Err("frame stride must be at least width * 3".to_string());
        }
        VideoFrame::packed(
            FramePosition {
                frame_index: 0,
                timestamp: Timestamp::new(0, Timebase::new(1, 1)),
            },
            self.width,
            self.height,
            pixel_format,
            &self.data,
            stride,
        )
        .map_err(|error| error.to_string())
    }
}

impl FrameEditRequest {
    fn edit(self) -> Result<FrameEdit, String> {
        match self.kind.as_str() {
            "crop" => Ok(FrameEdit::Crop(
                video_analysis_core::BoundingBox::new(
                    self.x.ok_or("crop requires x")?,
                    self.y.ok_or("crop requires y")?,
                    self.width.ok_or("crop requires width")?,
                    self.height.ok_or("crop requires height")?,
                )
                .map_err(|error| error.to_string())?,
            )),
            "resizeNearest" => Ok(FrameEdit::ResizeNearest {
                width: self.width.ok_or("resizeNearest requires width")?,
                height: self.height.ok_or("resizeNearest requires height")?,
            }),
            "boxBlur" => Ok(FrameEdit::BoxBlur {
                radius: self.radius.unwrap_or(1),
            }),
            "grayscale" => Ok(FrameEdit::Grayscale),
            "invert" => Ok(FrameEdit::Invert),
            "flipHorizontal" => Ok(FrameEdit::FlipHorizontal),
            "flipVertical" => Ok(FrameEdit::FlipVertical),
            "rotate90" => Ok(FrameEdit::Rotate90 {
                clockwise_turns: self.clockwise_turns.unwrap_or(1),
            }),
            "fadeToColor" => Ok(FrameEdit::FadeToColor {
                color: self.color.unwrap_or([0, 0, 0]),
                amount: self.amount.unwrap_or(1.0),
            }),
            "brightnessContrast" => Ok(FrameEdit::BrightnessContrast {
                brightness: self.brightness.unwrap_or(0),
                contrast: self.contrast.unwrap_or(1.0),
            }),
            other => Err(format!("unsupported frame edit `{other}`")),
        }
    }
}

fn parse_pixel_format(value: &str) -> Result<PixelFormat, String> {
    match value {
        "rgb24" => Ok(PixelFormat::Rgb24),
        "bgr24" => Ok(PixelFormat::Bgr24),
        other => Err(format!("unsupported pixel format `{other}`")),
    }
}

fn pixel_format_name(format: PixelFormat) -> &'static str {
    match format {
        PixelFormat::Rgb24 => "rgb24",
        PixelFormat::Bgr24 => "bgr24",
    }
}

fn sample_frame_json() -> serde_json::Value {
    serde_json::json!({
        "width": 2,
        "height": 2,
        "pixelFormat": "rgb24",
        "stride": null,
        "data": [255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255]
    })
}

fn operation_family(operation_id: &str) -> &str {
    operation_id
        .split_once('.')
        .map(|(family, _)| family)
        .unwrap_or(operation_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_surface_has_multiple_operations() {
        let surface = package_surface();
        assert_eq!(surface.library, env!("CARGO_PKG_NAME"));
        assert!(surface
            .operations
            .iter()
            .any(|operation| operation.id.as_str() == "describe"));
        assert!(surface.operations.len() >= 4);
    }

    #[test]
    fn describe_operation_returns_surface_summary() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("describe"),
            input: serde_json::json!({"includeOperations": true}),
        })
        .expect("describe operation");

        assert_eq!(response.operation.as_str(), "describe");
        assert_eq!(response.value["library"], env!("CARGO_PKG_NAME"));
        assert!(response.value["operationCount"].as_u64().unwrap() >= 4);
    }

    #[test]
    fn package_operation_returns_deterministic_plan() {
        let operation_id = package_surface().operations[1].id.clone();
        let response = run_surface_operation(SurfaceRequest {
            operation: operation_id,
            input: serde_json::json!({"sample": true}),
        })
        .expect("package operation");

        assert_eq!(response.value["deterministic"], true);
        assert_eq!(response.value["externalToolsRequired"], false);
    }

    #[test]
    fn edit_plan_operations_return_real_edl_summaries() {
        let cut = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("video.editing.cutPlan"),
            input: serde_json::json!({
                "sourceId": "clip-a",
                "intervals": [
                    {"startSeconds": 0.0, "endSeconds": 1.0},
                    {"startSeconds": 2.0, "endSeconds": 3.0}
                ]
            }),
        })
        .expect("cut plan");
        assert_eq!(cut.value["clipCount"], 2);
        assert_eq!(cut.value["durationSeconds"], 2.0);

        let frame = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("video.editing.frameApply"),
            input: serde_json::json!({
                "frame": sample_frame_json(),
                "edits": [{"type": "flipHorizontal"}, {"type": "fadeToColor", "color": [0, 0, 0], "amount": 0.5}],
                "previewLimit": 3
            }),
        })
        .expect("frame apply");
        assert_eq!(frame.value["width"], 2);
    }
}
