//! Internal module support for video red cars.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use video_analysis_core::{
    BoundingBox, DetectError, ObservationKind, RealtimeVideoPipeline, Result,
};
use video_analysis_detectors::ContentDetector;
use video_analysis_ffmpeg::FfmpegVideoSource;
use video_analysis_ingest::VideoFrameSource;
use video_analysis_models::{
    normalize_predictions, DownloadedModel, ExternalCommandModel, HuggingFaceModelSpec, ModelTask,
    PredictionRepairOptions, VisionModelBackend,
};

use crate::workflow_support::{display_path, validate_local_file, write_json_report};
use crate::{
    CapabilityReport, ExternalCommandConfig, ObservationReport, RegionReport,
    VIDEO_RED_CARS_USE_CASE,
};

const DEFAULT_SCENE_THRESHOLD: f32 = 27.0;
const DEFAULT_MIN_SCENE_LEN: u64 = 15;
const DEFAULT_VISUAL_SAMPLE_EVERY: u64 = 30;
const DEFAULT_TRACK_IOU: f32 = 0.5;

#[derive(Debug, Clone)]
/// Data type for video red cars request.
pub struct VideoRedCarsRequest {
    /// The input value.
    pub input: PathBuf,
    /// The work dir value.
    pub work_dir: PathBuf,
    /// The output value.
    pub output: Option<PathBuf>,
    /// The scene threshold value.
    pub scene_threshold: f32,
    /// The min scene len value.
    pub min_scene_len: u64,
    /// The max frames value.
    pub max_frames: Option<u64>,
    /// The visual sample every value.
    pub visual_sample_every: u64,
    /// The vehicle detector command value.
    pub vehicle_detector_command: PathBuf,
    /// The vehicle detector args value.
    pub vehicle_detector_args: Vec<String>,
}

impl Default for VideoRedCarsRequest {
    fn default() -> Self {
        Self {
            input: PathBuf::from("input.mp4"),
            work_dir: PathBuf::from("use-case-output/video-red-cars"),
            output: None,
            scene_threshold: DEFAULT_SCENE_THRESHOLD,
            min_scene_len: DEFAULT_MIN_SCENE_LEN,
            max_frames: None,
            visual_sample_every: DEFAULT_VISUAL_SAMPLE_EVERY,
            vehicle_detector_command: PathBuf::from("python3"),
            vehicle_detector_args: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for video red cars run request.
pub struct VideoRedCarsRunRequest {
    /// The input value.
    pub input: PathBuf,
    /// The work dir value.
    pub work_dir: Option<PathBuf>,
    #[serde(default = "default_scene_threshold")]
    /// The scene threshold value.
    pub scene_threshold: f32,
    #[serde(default = "default_min_scene_len")]
    /// The min scene len value.
    pub min_scene_len: u64,
    /// The max frames value.
    pub max_frames: Option<u64>,
    #[serde(default = "default_visual_sample_every")]
    /// The visual sample every value.
    pub visual_sample_every: u64,
    /// The vehicle detector value.
    pub vehicle_detector: ExternalCommandConfig,
}

impl VideoRedCarsRunRequest {
    /// Validates this value.
    pub fn validate(&self) -> Result<()> {
        validate_local_file(&self.input)?;
        validate_analysis_options(
            self.scene_threshold,
            self.min_scene_len,
            self.visual_sample_every,
        )?;
        if self.vehicle_detector.command.as_os_str().is_empty() {
            return Err(DetectError::InvalidArgument(
                "vehicle_detector.command is required".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for video red cars report.
pub struct VideoRedCarsReport {
    #[serde(alias = "use_case")]
    /// The workflow value.
    pub workflow: String,
    /// The source value.
    pub source: VideoRedCarsSourceReport,
    /// The assets value.
    pub assets: VideoRedCarsAssetReport,
    /// The capabilities value.
    pub capabilities: CapabilityReport,
    /// The video value.
    pub video: VideoRedCarsVideoReport,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for video red cars source report.
pub struct VideoRedCarsSourceReport {
    /// The local video value.
    pub local_video: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for video red cars asset report.
pub struct VideoRedCarsAssetReport {
    /// The work dir value.
    pub work_dir: String,
    /// The report path value.
    pub report_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for video red cars video report.
pub struct VideoRedCarsVideoReport {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// The frame rate value.
    pub frame_rate: String,
    /// Duration in seconds.
    pub duration_seconds: Option<f64>,
    /// The frames processed value.
    pub frames_processed: u64,
    /// The scenes value.
    pub scenes: Vec<VideoRedCarsSceneReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for video red cars scene report.
pub struct VideoRedCarsSceneReport {
    /// The index value.
    pub index: u64,
    /// The start frame value.
    pub start_frame: u64,
    /// The end frame value.
    pub end_frame: u64,
    /// The start seconds value.
    pub start_seconds: f64,
    /// The end seconds value.
    pub end_seconds: f64,
    /// The red car count value.
    pub red_car_count: u64,
    /// The peak visible red cars value.
    pub peak_visible_red_cars: u64,
    /// The sampled frames value.
    pub sampled_frames: u64,
    /// The red car observations value.
    pub red_car_observations: Vec<ObservationReport>,
}

#[derive(Debug, Clone)]
struct RedCarObservation {
    frame_index: u64,
    observation: ObservationReport,
    region: BoundingBox,
}

/// Runs video red cars.
pub fn run_video_red_cars(args: VideoRedCarsRequest) -> Result<VideoRedCarsReport> {
    validate_local_file(&args.input)?;
    validate_analysis_options(
        args.scene_threshold,
        args.min_scene_len,
        args.visual_sample_every,
    )?;
    if args.vehicle_detector_command.as_os_str().is_empty() {
        return Err(DetectError::InvalidArgument(
            "vehicle_detector_command is required".to_string(),
        ));
    }

    std::fs::create_dir_all(&args.work_dir)?;
    let report_path = args
        .output
        .clone()
        .unwrap_or_else(|| args.work_dir.join("analysis.json"));

    let mut source = FfmpegVideoSource::open(&args.input)?;
    let metadata = source.metadata().clone();
    let mut pipeline = RealtimeVideoPipeline::builder()
        .scene_detector(ContentDetector::new(
            args.scene_threshold,
            args.min_scene_len,
        ))
        .start_in_scene(true)
        .build()?;
    let mut detector = ExternalCommandModel::new(
        &args.vehicle_detector_command,
        downloaded_external_model("vehicle-detector", ModelTask::ObjectDetection),
    )
    .args(args.vehicle_detector_args.clone());
    let accepted_labels = accepted_vehicle_labels();
    let mut red_samples = Vec::new();
    let mut sampled_frame_indices = Vec::new();

    while let Some(frame) = source.next_video_frame()? {
        let frame_ref = frame.as_frame();
        let frame_index = frame.position.frame_index;
        if frame_index % args.visual_sample_every == 0 {
            sampled_frame_indices.push(frame_index);
            red_samples.extend(red_observations_for_frame(
                &mut detector,
                &frame_ref,
                &accepted_labels,
            )?);
        }
        pipeline.process_frame(frame)?;
        if args
            .max_frames
            .map(|limit| pipeline.frames_processed() >= limit)
            .unwrap_or(false)
        {
            break;
        }
    }

    let result = pipeline.finish_analysis()?;
    let scenes = result
        .detection
        .scenes
        .iter()
        .enumerate()
        .map(|(index, scene)| {
            let red_car_observations = red_samples
                .iter()
                .filter(|sample| sample.frame_index >= scene.start.frame_index)
                .filter(|sample| sample.frame_index <= scene.end.frame_index)
                .cloned()
                .collect::<Vec<_>>();
            let sampled_frames = sampled_frame_indices
                .iter()
                .filter(|frame_index| **frame_index >= scene.start.frame_index)
                .filter(|frame_index| **frame_index <= scene.end.frame_index)
                .count() as u64;
            let peak_visible_red_cars = peak_visible_red_cars(&red_car_observations);
            VideoRedCarsSceneReport {
                index: index as u64,
                start_frame: scene.start.frame_index,
                end_frame: scene.end.frame_index,
                start_seconds: scene.start.timestamp.seconds(),
                end_seconds: scene.end.timestamp.seconds(),
                red_car_count: count_distinct_tracks(
                    &red_car_observations,
                    args.visual_sample_every,
                    DEFAULT_TRACK_IOU,
                ) as u64,
                peak_visible_red_cars,
                sampled_frames,
                red_car_observations: red_car_observations
                    .into_iter()
                    .map(|sample| sample.observation)
                    .collect(),
            }
        })
        .collect();

    Ok(VideoRedCarsReport {
        workflow: VIDEO_RED_CARS_USE_CASE.to_string(),
        source: VideoRedCarsSourceReport {
            local_video: display_path(&args.input),
        },
        assets: VideoRedCarsAssetReport {
            work_dir: display_path(&args.work_dir),
            report_path: display_path(&report_path),
        },
        capabilities: CapabilityReport {
            completed: vec![
                "scene_detection".to_string(),
                "vehicle_detection".to_string(),
                "red_car_counting".to_string(),
            ],
            skipped: Vec::new(),
        },
        video: VideoRedCarsVideoReport {
            width: metadata.width,
            height: metadata.height,
            frame_rate: format!(
                "{}/{}",
                metadata.frame_rate.numer(),
                metadata.frame_rate.denom()
            ),
            duration_seconds: metadata.duration_seconds,
            frames_processed: result.frames_processed,
            scenes,
        },
    })
}

/// Runs video red cars workflow.
pub fn run_video_red_cars_workflow(
    request: VideoRedCarsRunRequest,
    work_dir: PathBuf,
    report_path: PathBuf,
) -> Result<VideoRedCarsReport> {
    request.validate()?;
    let report = run_video_red_cars(VideoRedCarsRequest {
        input: request.input,
        work_dir,
        output: Some(report_path.clone()),
        scene_threshold: request.scene_threshold,
        min_scene_len: request.min_scene_len,
        max_frames: request.max_frames,
        visual_sample_every: request.visual_sample_every,
        vehicle_detector_command: request.vehicle_detector.command,
        vehicle_detector_args: request.vehicle_detector.args,
    })?;
    write_video_red_cars_report(&report_path, &report)?;
    Ok(report)
}

/// Writes video red cars report.
pub fn write_video_red_cars_report(path: &Path, report: &VideoRedCarsReport) -> Result<()> {
    write_json_report(path, report)
}

fn default_scene_threshold() -> f32 {
    DEFAULT_SCENE_THRESHOLD
}

fn default_min_scene_len() -> u64 {
    DEFAULT_MIN_SCENE_LEN
}

fn default_visual_sample_every() -> u64 {
    DEFAULT_VISUAL_SAMPLE_EVERY
}

fn validate_analysis_options(
    scene_threshold: f32,
    min_scene_len: u64,
    visual_sample_every: u64,
) -> Result<()> {
    if !scene_threshold.is_finite() || scene_threshold <= 0.0 {
        return Err(DetectError::InvalidArgument(
            "scene_threshold must be finite and positive".to_string(),
        ));
    }
    if min_scene_len == 0 {
        return Err(DetectError::InvalidArgument(
            "min_scene_len must be positive".to_string(),
        ));
    }
    if visual_sample_every == 0 {
        return Err(DetectError::InvalidArgument(
            "visual_sample_every must be positive".to_string(),
        ));
    }
    Ok(())
}

fn downloaded_external_model(name: &str, task: ModelTask) -> DownloadedModel {
    DownloadedModel {
        spec: HuggingFaceModelSpec::new(name, task).name(name),
        files: BTreeMap::new(),
    }
}

fn accepted_vehicle_labels() -> BTreeSet<String> {
    ["car", "vehicle", "automobile"]
        .into_iter()
        .map(str::to_string)
        .collect()
}

fn red_observations_for_frame(
    detector: &mut ExternalCommandModel,
    frame: &video_analysis_core::VideoFrame<'_>,
    accepted_labels: &BTreeSet<String>,
) -> Result<Vec<RedCarObservation>> {
    let raw = detector.predict_frame(frame)?;
    Ok(normalize_predictions(
        raw,
        &ModelTask::ObjectDetection,
        Some((frame.width, frame.height)),
        PredictionRepairOptions::default(),
    )
    .into_iter()
    .filter(|prediction| prediction.kind == ObservationKind::Object)
    .filter_map(|prediction| {
        let label = prediction.label.clone()?;
        accepted_labels
            .contains(&label.to_ascii_lowercase())
            .then_some((label, prediction))
    })
    .filter_map(|(label, prediction)| {
        let region = prediction.region?;
        is_red_vehicle(frame, region, &prediction.attributes).then_some((label, prediction, region))
    })
    .map(|(label, prediction, region)| RedCarObservation {
        frame_index: frame.position.frame_index,
        observation: ObservationReport {
            timestamp_seconds: Some(frame.position.timestamp.seconds()),
            frame_index: Some(frame.position.frame_index),
            scene_index: None,
            analyzer: "vehicle_red_car_detector".to_string(),
            kind: "object".to_string(),
            label: Some(label),
            text: prediction.text,
            score: prediction.score,
            region: Some(region_report(region)),
            track_id: None,
            attributes: prediction.attributes,
        },
        region,
    })
    .collect())
}

fn is_red_vehicle(
    frame: &video_analysis_core::VideoFrame<'_>,
    region: BoundingBox,
    attributes: &BTreeMap<String, String>,
) -> bool {
    if attributes
        .get("color")
        .is_some_and(|value| value.eq_ignore_ascii_case("red"))
    {
        return true;
    }
    if attributes
        .get("dominant_color")
        .is_some_and(|value| value.eq_ignore_ascii_case("red"))
    {
        return true;
    }

    let [r, g, b] = mean_rgb(frame, region);
    r >= 96.0 && r >= 1.35 * g && r >= 1.35 * b
}

fn mean_rgb(frame: &video_analysis_core::VideoFrame<'_>, region: BoundingBox) -> [f32; 3] {
    let mut sums = [0_u64; 3];
    let mut pixels = 0_u64;
    for y in region.y..region.y.saturating_add(region.height).min(frame.height) {
        for x in region.x..region.x.saturating_add(region.width).min(frame.width) {
            let pixel = frame.pixel_rgb(x, y);
            sums[0] += pixel[0] as u64;
            sums[1] += pixel[1] as u64;
            sums[2] += pixel[2] as u64;
            pixels += 1;
        }
    }
    if pixels == 0 {
        return [0.0, 0.0, 0.0];
    }
    [
        sums[0] as f32 / pixels as f32,
        sums[1] as f32 / pixels as f32,
        sums[2] as f32 / pixels as f32,
    ]
}

fn count_distinct_tracks(
    observations: &[RedCarObservation],
    visual_sample_every: u64,
    min_iou: f32,
) -> usize {
    let mut tracks: Vec<(u64, BoundingBox)> = Vec::new();
    let max_gap = visual_sample_every.max(1) * 2;
    for observation in observations {
        let mut best_index = None;
        let mut best_iou = 0.0_f32;
        for (index, (last_frame, last_region)) in tracks.iter().enumerate() {
            if observation.frame_index.saturating_sub(*last_frame) > max_gap {
                continue;
            }
            let iou = bbox_iou(*last_region, observation.region);
            if iou >= min_iou && iou > best_iou {
                best_iou = iou;
                best_index = Some(index);
            }
        }
        if let Some(index) = best_index {
            tracks[index] = (observation.frame_index, observation.region);
        } else {
            tracks.push((observation.frame_index, observation.region));
        }
    }
    tracks.len()
}

fn peak_visible_red_cars(observations: &[RedCarObservation]) -> u64 {
    let mut counts = BTreeMap::<u64, u64>::new();
    for observation in observations {
        *counts.entry(observation.frame_index).or_default() += 1;
    }
    counts.into_values().max().unwrap_or(0)
}

fn bbox_iou(left: BoundingBox, right: BoundingBox) -> f32 {
    let left_x1 = left.x + left.width;
    let left_y1 = left.y + left.height;
    let right_x1 = right.x + right.width;
    let right_y1 = right.y + right.height;

    let ix0 = left.x.max(right.x);
    let iy0 = left.y.max(right.y);
    let ix1 = left_x1.min(right_x1);
    let iy1 = left_y1.min(right_y1);
    if ix1 <= ix0 || iy1 <= iy0 {
        return 0.0;
    }
    let intersection = (ix1 - ix0) as f32 * (iy1 - iy0) as f32;
    let left_area = left.width as f32 * left.height as f32;
    let right_area = right.width as f32 * right.height as f32;
    intersection / (left_area + right_area - intersection)
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
    use video_analysis_core::{FramePosition, PixelFormat, Timebase, Timestamp};

    fn test_frame(rgb: [u8; 3]) -> video_analysis_core::OwnedVideoFrame {
        let mut data = Vec::new();
        for _ in 0..(8 * 8) {
            data.extend_from_slice(&rgb);
        }
        video_analysis_core::OwnedVideoFrame {
            position: FramePosition {
                frame_index: 0,
                timestamp: Timestamp::new(0, Timebase::new(1, 30)),
            },
            width: 8,
            height: 8,
            pixel_format: PixelFormat::Rgb24,
            data,
            stride: 8 * 3,
        }
    }

    fn red_observation(frame_index: u64, region: BoundingBox) -> RedCarObservation {
        RedCarObservation {
            frame_index,
            observation: ObservationReport {
                timestamp_seconds: Some(frame_index as f64 / 30.0),
                frame_index: Some(frame_index),
                scene_index: Some(0),
                analyzer: "test".to_string(),
                kind: "object".to_string(),
                label: Some("car".to_string()),
                text: None,
                score: Some(0.9),
                region: Some(region_report(region)),
                track_id: None,
                attributes: BTreeMap::new(),
            },
            region,
        }
    }

    #[test]
    fn video_red_cars_classifies_red_boxes_correctly() {
        let frame = test_frame([220, 20, 20]);
        assert!(is_red_vehicle(
            &frame.as_frame(),
            BoundingBox::new(0, 0, 8, 8).unwrap(),
            &BTreeMap::new()
        ));

        let blue = test_frame([20, 20, 220]);
        assert!(!is_red_vehicle(
            &blue.as_frame(),
            BoundingBox::new(0, 0, 8, 8).unwrap(),
            &BTreeMap::new()
        ));
    }

    #[test]
    fn video_red_cars_counts_distinct_tracks_per_scene() {
        let observations = vec![
            red_observation(0, BoundingBox::new(0, 0, 4, 4).unwrap()),
            red_observation(30, BoundingBox::new(1, 0, 4, 4).unwrap()),
            red_observation(60, BoundingBox::new(20, 0, 4, 4).unwrap()),
        ];

        assert_eq!(
            count_distinct_tracks(
                &observations,
                DEFAULT_VISUAL_SAMPLE_EVERY,
                DEFAULT_TRACK_IOU
            ),
            2
        );
        assert_eq!(peak_visible_red_cars(&observations), 1);
    }

    #[test]
    fn report_roundtrips_for_each_new_use_case() {
        let report = VideoRedCarsReport {
            workflow: VIDEO_RED_CARS_USE_CASE.to_string(),
            source: VideoRedCarsSourceReport {
                local_video: "input.mp4".to_string(),
            },
            assets: VideoRedCarsAssetReport {
                work_dir: "work".to_string(),
                report_path: "analysis.json".to_string(),
            },
            capabilities: CapabilityReport {
                completed: vec!["scene_detection".to_string()],
                skipped: Vec::new(),
            },
            video: VideoRedCarsVideoReport {
                width: 64,
                height: 64,
                frame_rate: "30/1".to_string(),
                duration_seconds: Some(1.0),
                frames_processed: 30,
                scenes: vec![VideoRedCarsSceneReport {
                    index: 0,
                    start_frame: 0,
                    end_frame: 29,
                    start_seconds: 0.0,
                    end_seconds: 1.0,
                    red_car_count: 1,
                    peak_visible_red_cars: 1,
                    sampled_frames: 1,
                    red_car_observations: Vec::new(),
                }],
            },
        };

        let value = serde_json::to_vec(&report).unwrap();
        let decoded: VideoRedCarsReport = serde_json::from_slice(&value).unwrap();
        assert_eq!(decoded, report);
    }
}
