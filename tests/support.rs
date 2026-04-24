#![allow(dead_code)]

use std::f32::consts::PI;

use video_analysis_core::{
    AnalysisEvent, AudioBuffer, FramePosition, Observation, ObservationKind, OwnedAudioFrame,
    OwnedTextSegment, OwnedVideoFrame, PixelFormat, Scene, TextSegment, Timebase, Timestamp,
};
use video_analysis_dataset::{
    AnalysisDataset, DatasetRecord, FeatureRecord, FeatureValue, SceneRecord, TextSegmentRecord,
};

pub fn sine(freq_hz: f32, sample_rate: u32, seconds: f32) -> Vec<f32> {
    let samples = (sample_rate as f32 * seconds) as usize;
    (0..samples)
        .map(|index| {
            let t = index as f32 / sample_rate as f32;
            (2.0 * PI * freq_hz * t).sin()
        })
        .collect()
}

pub fn click_track(sample_rate: u32, bpm: f32, seconds: f32) -> Vec<f32> {
    let len = (sample_rate as f32 * seconds) as usize;
    let interval = (sample_rate as f32 * 60.0 / bpm).max(1.0) as usize;
    let mut samples = vec![0.0; len];
    for start in (0..len).step_by(interval) {
        for sample in samples.iter_mut().skip(start).take(8) {
            *sample = 1.0;
        }
    }
    samples
}

pub fn interleaved_stereo(left: &[f32], right: &[f32]) -> Vec<f32> {
    assert_eq!(left.len(), right.len(), "stereo channels must match");
    left.iter()
        .zip(right)
        .flat_map(|(left, right)| [*left, *right])
        .collect()
}

pub fn owned_f32_frame(
    timestamp: Timestamp,
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
) -> video_analysis_core::Result<OwnedAudioFrame> {
    OwnedAudioFrame::new(timestamp, sample_rate, channels, AudioBuffer::F32(samples))
}

pub fn assert_approx_eq(actual: f32, expected: f32, tolerance: f32) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {actual} to be within {tolerance} of {expected}"
    );
}

pub fn timestamp(seconds: i64) -> Timestamp {
    Timestamp::new(seconds, Timebase::new(1, 1))
}

pub fn frame_position(frame_index: u64) -> FramePosition {
    FramePosition {
        frame_index,
        timestamp: timestamp(frame_index as i64),
    }
}

pub fn scene(start: u64, end: u64) -> Scene {
    Scene {
        start: frame_position(start),
        end: frame_position(end),
    }
}

pub fn rgb_frame(width: u32, height: u32, frame_index: u64, rgb: [u8; 3]) -> OwnedVideoFrame {
    let mut data = Vec::with_capacity(width as usize * height as usize * 3);
    for _ in 0..(width as usize * height as usize) {
        data.extend_from_slice(&rgb);
    }
    OwnedVideoFrame {
        position: frame_position(frame_index),
        width,
        height,
        pixel_format: PixelFormat::Rgb24,
        stride: width as usize * 3,
        data,
    }
}

pub fn checkerboard_frame(width: u32, height: u32, frame_index: u64) -> OwnedVideoFrame {
    let mut data = Vec::with_capacity(width as usize * height as usize * 3);
    for y in 0..height {
        for x in 0..width {
            let value = if (x + y) % 2 == 0 { 255 } else { 0 };
            data.extend_from_slice(&[value, value, value]);
        }
    }
    OwnedVideoFrame {
        position: frame_position(frame_index),
        width,
        height,
        pixel_format: PixelFormat::Rgb24,
        stride: width as usize * 3,
        data,
    }
}

pub fn text_segment(index: u64, text: &str) -> TextSegment<'_> {
    TextSegment {
        segment_index: index,
        timestamp: Some(timestamp(index as i64)),
        text,
        language: Some("en"),
        is_final: true,
    }
}

pub fn owned_text_segment(index: u64, text: impl Into<String>) -> OwnedTextSegment {
    OwnedTextSegment::new(index, text).timestamp(timestamp(index as i64))
}

pub fn dataset_with_scene_text_and_feature() -> AnalysisDataset {
    let mut dataset = AnalysisDataset::empty();
    let scene = scene(0, 2);
    let text = text_segment(0, "Rust video analysis test fixture.");
    dataset.push(DatasetRecord::Scene(SceneRecord::from_scene(0, &scene)));
    dataset.push(DatasetRecord::TextSegment(TextSegmentRecord::from_segment(
        "transcript",
        &text,
    )));
    dataset.extend_observations([Observation::new("fixture", ObservationKind::Text)
        .at_timestamp(timestamp(0))
        .in_scene(0)
        .label("fixture")
        .text(text.text)]);
    dataset
        .extend_events([AnalysisEvent::new("fixture", "text:fixture").at_timestamp(timestamp(0))]);
    dataset.push(DatasetRecord::Feature(
        FeatureRecord::new("fixture.vector", FeatureValue::Vector(vec![1.0, 2.0, 3.0]))
            .scope("global")
            .timestamp(timestamp(0)),
    ));
    dataset
}
