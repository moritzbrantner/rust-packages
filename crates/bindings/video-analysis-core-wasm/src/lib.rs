//! WASM bindings for browser-friendly video analysis core helpers.

use num_rational::Rational64;
use serde::Serialize;
use serde_wasm_bindgen::Serializer;
use video_analysis_core::{
    scenes_from_cuts, Cut, DetectError, FramePosition, FrameTimecode, PixelFormat, VideoFrame,
};
use wasm_bindgen::prelude::*;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RawFrameTimecode {
    frame_index: u32,
    seconds: f64,
    timecode: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RawRgb {
    r: u8,
    g: u8,
    b: u8,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RawMeanRgb {
    r: f32,
    g: f32,
    b: f32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RawVideoFrameAnalysis {
    width: u32,
    height: u32,
    pixel_format: &'static str,
    pixel_count: usize,
    frame_index: u32,
    seconds: f64,
    timecode: String,
    top_left: RawRgb,
    center: RawRgb,
    mean_rgb: RawMeanRgb,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RawScene {
    start_frame: u32,
    end_frame: u32,
    start_seconds: f64,
    end_seconds: f64,
}

#[wasm_bindgen(js_name = frameTimecode)]
/// Converts a frame index to seconds and SMPTE-like timecode text.
pub fn frame_timecode(
    frame_index: u32,
    fps_numerator: i32,
    fps_denominator: i32,
    precision: usize,
) -> Result<JsValue, JsValue> {
    to_js_value(&frame_timecode_data(
        frame_index,
        fps_numerator,
        fps_denominator,
        precision,
    )?)
}

#[wasm_bindgen(js_name = parseFrameTimecode)]
/// Parses a frame number, seconds value, or HH:MM:SS timecode string.
pub fn parse_frame_timecode(
    input: &str,
    fps_numerator: i32,
    fps_denominator: i32,
    precision: usize,
) -> Result<JsValue, JsValue> {
    let fps = fps(fps_numerator, fps_denominator)?;
    let parsed = FrameTimecode::parse(input, fps).map_err(into_js_error)?;
    to_js_value(&RawFrameTimecode {
        frame_index: parsed.frame_index as u32,
        seconds: parsed.seconds(),
        timecode: parsed.timecode(precision),
    })
}

#[wasm_bindgen(js_name = analyzeVideoFrame)]
/// Analyzes a packed RGB/BGR frame buffer and returns timing plus pixel summaries.
pub fn analyze_video_frame(
    data: Vec<u8>,
    width: u32,
    height: u32,
    pixel_format: &str,
    frame_index: u32,
    fps_numerator: i32,
    fps_denominator: i32,
    precision: usize,
) -> Result<JsValue, JsValue> {
    to_js_value(&analyze_video_frame_data(
        &data,
        width,
        height,
        pixel_format,
        frame_index,
        fps_numerator,
        fps_denominator,
        precision,
    )?)
}

#[wasm_bindgen(js_name = scenesFromCutFrames)]
/// Builds scene intervals from cut frame indexes and a total frame count.
pub fn scenes_from_cut_frames(
    cut_frames: Vec<u32>,
    total_frames: u32,
    fps_numerator: i32,
    fps_denominator: i32,
) -> Result<JsValue, JsValue> {
    let fps = fps(fps_numerator, fps_denominator)?;
    let cuts = cut_frames
        .into_iter()
        .map(|frame_index| Cut {
            position: FramePosition::from_frame_index(frame_index as u64, fps),
            detector: "wasm",
            score: None,
        })
        .collect::<Vec<_>>();
    let start_position = FramePosition::from_frame_index(0, fps);
    let last_position = FramePosition::from_frame_index(total_frames.saturating_sub(1) as u64, fps);
    let scenes = scenes_from_cuts(&cuts, start_position, last_position, true)
        .into_iter()
        .map(|scene| RawScene {
            start_frame: scene.start.frame_index as u32,
            end_frame: scene.end.frame_index as u32,
            start_seconds: scene.start.timestamp.seconds(),
            end_seconds: scene.end.timestamp.seconds(),
        })
        .collect::<Vec<_>>();
    to_js_value(&scenes)
}

fn frame_timecode_data(
    frame_index: u32,
    fps_numerator: i32,
    fps_denominator: i32,
    precision: usize,
) -> Result<RawFrameTimecode, JsValue> {
    let timecode =
        FrameTimecode::from_frames(frame_index as u64, fps(fps_numerator, fps_denominator)?);
    Ok(RawFrameTimecode {
        frame_index,
        seconds: timecode.seconds(),
        timecode: timecode.timecode(precision),
    })
}

fn analyze_video_frame_data(
    data: &[u8],
    width: u32,
    height: u32,
    pixel_format: &str,
    frame_index: u32,
    fps_numerator: i32,
    fps_denominator: i32,
    precision: usize,
) -> Result<RawVideoFrameAnalysis, JsValue> {
    let fps = fps(fps_numerator, fps_denominator)?;
    let parsed_format = parse_pixel_format(pixel_format)?;
    let frame = VideoFrame::packed(
        FramePosition::from_frame_index(frame_index as u64, fps),
        width,
        height,
        parsed_format,
        data,
        width as usize * 3,
    )
    .map_err(into_js_error)?;
    let mean_rgb = mean_rgb(&frame);
    let timecode = FrameTimecode::from_frames(frame_index as u64, fps);

    Ok(RawVideoFrameAnalysis {
        width,
        height,
        pixel_format: pixel_format_name(parsed_format),
        pixel_count: frame.pixel_count(),
        frame_index,
        seconds: timecode.seconds(),
        timecode: timecode.timecode(precision),
        top_left: raw_rgb(frame.pixel_rgb(0, 0)),
        center: raw_rgb(frame.pixel_rgb(width / 2, height / 2)),
        mean_rgb,
    })
}

fn mean_rgb(frame: &VideoFrame<'_>) -> RawMeanRgb {
    let mut r = 0_u64;
    let mut g = 0_u64;
    let mut b = 0_u64;
    for y in 0..frame.height {
        for x in 0..frame.width {
            let pixel = frame.pixel_rgb(x, y);
            r += pixel[0] as u64;
            g += pixel[1] as u64;
            b += pixel[2] as u64;
        }
    }
    let pixels = frame.pixel_count().max(1) as f32;
    RawMeanRgb {
        r: r as f32 / pixels,
        g: g as f32 / pixels,
        b: b as f32 / pixels,
    }
}

fn raw_rgb(pixel: [u8; 3]) -> RawRgb {
    RawRgb {
        r: pixel[0],
        g: pixel[1],
        b: pixel[2],
    }
}

fn parse_pixel_format(value: &str) -> Result<PixelFormat, JsValue> {
    match value {
        "rgb24" | "rgb" => Ok(PixelFormat::Rgb24),
        "bgr24" | "bgr" => Ok(PixelFormat::Bgr24),
        other => Err(JsValue::from_str(&format!(
            "unsupported pixel format `{other}`; expected `rgb24` or `bgr24`"
        ))),
    }
}

fn pixel_format_name(value: PixelFormat) -> &'static str {
    match value {
        PixelFormat::Rgb24 => "rgb24",
        PixelFormat::Bgr24 => "bgr24",
    }
}

fn fps(numerator: i32, denominator: i32) -> Result<Rational64, JsValue> {
    if numerator <= 0 || denominator <= 0 {
        return Err(JsValue::from_str(
            "fps numerator and denominator must be positive",
        ));
    }
    Ok(Rational64::new(numerator as i64, denominator as i64))
}

fn to_js_value<T: Serialize>(value: &T) -> Result<JsValue, JsValue> {
    value
        .serialize(&Serializer::json_compatible())
        .map_err(into_deserialize_js_error)
}

fn into_js_error(error: DetectError) -> JsValue {
    JsValue::from_str(&error.to_string())
}

fn into_deserialize_js_error(error: serde_wasm_bindgen::Error) -> JsValue {
    JsValue::from_str(&error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyzes_rgb_frame_summary() {
        let data = vec![255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255];
        let analysis = analyze_video_frame_data(&data, 2, 2, "rgb24", 12, 24, 1, 3).unwrap();

        assert_eq!(analysis.pixel_count, 4);
        assert_eq!(analysis.timecode, "00:00:00.500");
        assert_eq!(analysis.top_left.r, 255);
        assert_eq!(analysis.center.r, 255);
        assert!((analysis.mean_rgb.r - 127.5).abs() < 0.001);
    }

    #[test]
    fn computes_frame_timecode() {
        let timecode = frame_timecode_data(48, 24, 1, 2).unwrap();
        assert_eq!(timecode.seconds, 2.0);
        assert_eq!(timecode.timecode, "00:00:02.00");
    }
}
