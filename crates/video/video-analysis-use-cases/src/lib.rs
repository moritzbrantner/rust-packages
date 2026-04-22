use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use text_analysis_features::TranscriptHeuristicAnalyzer;
use text_analysis_transcription::{
    segment_to_owned_text_segment, Transcriber, TranscriptSegment, WhisperCliTranscriber,
};
use video_analysis_core::{
    AnalysisEvent, AudioAnalyzer, AudioBuffer, AudioFrame, BoundingBox, DetectError, Observation,
    ObservationKind, RealtimeVideoPipeline, Result, VideoAnalyzer, VideoFrame,
};
use video_analysis_data::{
    BucketAggregator, BucketConfig, DataBucket, DataRecord, DataStreamKind, StreamSummary,
};
use video_analysis_detectors::ContentDetector;
use video_analysis_ffmpeg::{FfmpegAudioSource, FfmpegAudioSourceOptions, FfmpegVideoSource};
use video_analysis_ingest::{AudioFrameSource, VideoFrameSource};
use video_analysis_models::{
    DownloadedModel, ExternalCommandModel, HuggingFaceModelSpec, ModelTask, ModelTextAnalyzer,
    ModelVideoAnalyzer,
};

pub const YOUTUBE_VIDEO_USE_CASE: &str = "youtube-video";

pub mod youtube {
    pub use super::{
        run_youtube_video, write_youtube_video_report, AssetReport, AudioReport, CapabilityReport,
        DataBucketReport, EventReport, ObservationReport, RegionReport, SceneReport, SourceReport,
        StreamBucketReport, TextReport, TranscriptSegmentReport, TranscriptionReport, VideoReport,
        YoutubeVideoReport, YoutubeVideoRequest, YOUTUBE_VIDEO_USE_CASE,
    };
}

#[derive(Debug, Clone)]
pub struct YoutubeVideoRequest {
    pub url: Option<String>,
    pub input: Option<PathBuf>,
    pub work_dir: PathBuf,
    pub output: Option<PathBuf>,
    pub scene_threshold: f32,
    pub min_scene_len: u64,
    pub max_frames: Option<u64>,
    pub visual_sample_every: u64,
    pub skip_transcription: bool,
    pub transcriber_command: Option<PathBuf>,
    pub transcriber_args: Vec<String>,
    pub object_command: Option<PathBuf>,
    pub object_args: Vec<String>,
    pub ocr_command: Option<PathBuf>,
    pub ocr_args: Vec<String>,
    pub text_command: Option<PathBuf>,
    pub text_args: Vec<String>,
}

impl Default for YoutubeVideoRequest {
    fn default() -> Self {
        Self {
            url: None,
            input: None,
            work_dir: PathBuf::from("use-case-output/youtube-video"),
            output: None,
            scene_threshold: 27.0,
            min_scene_len: 15,
            max_frames: None,
            visual_sample_every: 30,
            skip_transcription: false,
            transcriber_command: None,
            transcriber_args: Vec::new(),
            object_command: None,
            object_args: Vec::new(),
            ocr_command: None,
            ocr_args: Vec::new(),
            text_command: None,
            text_args: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YoutubeVideoReport {
    pub use_case: String,
    pub source: SourceReport,
    pub assets: AssetReport,
    pub capabilities: CapabilityReport,
    pub video: VideoReport,
    pub transcription: TranscriptionReport,
    pub audio: AudioReport,
    pub text: TextReport,
    pub data_buckets: Vec<DataBucketReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceReport {
    pub url: Option<String>,
    pub local_video: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetReport {
    pub work_dir: String,
    pub report_path: String,
    pub audio_wav: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityReport {
    pub completed: Vec<String>,
    pub skipped: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoReport {
    pub width: u32,
    pub height: u32,
    pub frame_rate: String,
    pub duration_seconds: Option<f64>,
    pub frames_processed: u64,
    pub scenes: Vec<SceneReport>,
    pub observations: Vec<ObservationReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneReport {
    pub index: u64,
    pub start_frame: u64,
    pub end_frame: u64,
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub observations: Vec<ObservationReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationReport {
    pub timestamp_seconds: Option<f64>,
    pub frame_index: Option<u64>,
    pub scene_index: Option<u64>,
    pub analyzer: String,
    pub kind: String,
    pub label: Option<String>,
    pub text: Option<String>,
    pub score: Option<f32>,
    pub region: Option<RegionReport>,
    pub track_id: Option<String>,
    pub attributes: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionReport {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionReport {
    pub status: String,
    pub text: Option<String>,
    pub segments: Vec<TranscriptSegmentReport>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptSegmentReport {
    pub index: u64,
    pub start_seconds: Option<f64>,
    pub end_seconds: Option<f64>,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioReport {
    pub status: String,
    pub frames_processed: u64,
    pub events: Vec<EventReport>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextReport {
    pub status: String,
    pub segments_processed: u64,
    pub events: Vec<EventReport>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventReport {
    pub timestamp_seconds: Option<f64>,
    pub analyzer: String,
    pub label: String,
    pub score: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataBucketReport {
    pub bucket_index: u64,
    pub records: u64,
    pub estimated_bytes: u64,
    pub streams: Vec<StreamBucketReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamBucketReport {
    pub stream_id: String,
    pub records: u64,
    pub estimated_bytes: u64,
    pub payload_counts: BTreeMap<String, u64>,
    pub video_frames: u64,
    pub audio_frames: u64,
    pub text_segments: u64,
}

pub fn run_youtube_video(args: YoutubeVideoRequest) -> Result<YoutubeVideoReport> {
    fs::create_dir_all(&args.work_dir)?;
    let report_path = args
        .output
        .clone()
        .unwrap_or_else(|| args.work_dir.join("analysis.json"));

    let mut completed = Vec::new();
    let mut skipped = Vec::new();

    let video_path = match &args.input {
        Some(path) => path.clone(),
        None => {
            let url = args
                .url
                .as_deref()
                .ok_or_else(|| DetectError::InvalidArgument("missing --url".to_string()))?;
            let path = download_youtube_video(url, &args.work_dir)?;
            completed.push("youtube_download".to_string());
            path
        }
    };

    let mut data_buckets = Vec::new();
    let mut aggregator = BucketAggregator::new(BucketConfig::record_count(1_000)?)?;
    let (video_report, observations) =
        analyze_video(&args, &video_path, &mut aggregator, &mut data_buckets)?;
    completed.push("scene_detection".to_string());
    if args.object_command.is_some() {
        completed.push("object_person_detection".to_string());
    } else {
        skipped.push("object/person detection: pass --object-command".to_string());
    }
    if args.ocr_command.is_some() {
        completed.push("ocr".to_string());
    } else {
        skipped.push("on-screen text OCR: pass --ocr-command".to_string());
    }

    let (transcription, audio_wav) = transcribe_video(&args, &video_path);
    match transcription.status.as_str() {
        "completed" => completed.push("transcription".to_string()),
        _ => skipped.push(format!(
            "transcription: {}",
            transcription.message.as_deref().unwrap_or("not available")
        )),
    }

    let audio_report = match analyze_audio(&video_path, &mut aggregator, &mut data_buckets) {
        Ok(report) => {
            completed.push("audio_analysis".to_string());
            report
        }
        Err(err) => AudioReport {
            status: "skipped".to_string(),
            frames_processed: 0,
            events: Vec::new(),
            message: Some(err.to_string()),
        },
    };
    if audio_report.status != "completed" {
        skipped.push(format!(
            "audio analysis: {}",
            audio_report.message.as_deref().unwrap_or("not available")
        ));
    }

    let text_report =
        match analyze_transcript_text(&args, &transcription, &mut aggregator, &mut data_buckets) {
            Ok(report) => {
                if report.status == "completed" {
                    completed.push("transcript_text_analysis".to_string());
                }
                report
            }
            Err(err) => TextReport {
                status: "skipped".to_string(),
                segments_processed: 0,
                events: Vec::new(),
                message: Some(err.to_string()),
            },
        };
    if text_report.status != "completed" {
        skipped.push(format!(
            "transcript text analysis: {}",
            text_report.message.as_deref().unwrap_or("not available")
        ));
    }

    if let Some(bucket) = aggregator.finish() {
        data_buckets.push(bucket_report(&bucket));
    }

    let report = YoutubeVideoReport {
        use_case: YOUTUBE_VIDEO_USE_CASE.to_string(),
        source: SourceReport {
            url: args.url.clone(),
            local_video: display_path(&video_path),
        },
        assets: AssetReport {
            work_dir: display_path(&args.work_dir),
            report_path: display_path(&report_path),
            audio_wav: audio_wav.as_ref().map(|path| display_path(path)),
        },
        capabilities: CapabilityReport { completed, skipped },
        video: video_report_with_observations(video_report, observations),
        transcription,
        audio: audio_report,
        text: text_report,
        data_buckets,
    };

    Ok(report)
}

pub fn write_youtube_video_report(path: &Path, report: &YoutubeVideoReport) -> Result<()> {
    write_json_report(path, report)
}

fn download_youtube_video(url: &str, work_dir: &Path) -> Result<PathBuf> {
    let command = PathBuf::from("yt-dlp");
    if resolve_command(&command).is_none() {
        return Err(DetectError::Source(
            "yt-dlp is required for --url downloads".to_string(),
        ));
    }

    let output_template = work_dir.join("youtube-video.%(ext)s");
    let output = Command::new("yt-dlp")
        .arg("--no-playlist")
        .arg("--merge-output-format")
        .arg("mp4")
        .arg("--print")
        .arg("after_move:filepath")
        .arg("-o")
        .arg(&output_template)
        .arg(url)
        .output()?;

    if !output.status.success() {
        return Err(DetectError::Source(format!(
            "yt-dlp failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }

    for line in String::from_utf8_lossy(&output.stdout).lines().rev() {
        let path = PathBuf::from(line.trim());
        if path.exists() {
            return Ok(path);
        }
    }

    find_downloaded_video(work_dir).ok_or_else(|| {
        DetectError::Source("yt-dlp completed but no downloaded video was found".to_string())
    })
}

fn find_downloaded_video(work_dir: &Path) -> Option<PathBuf> {
    let mut candidates = fs::read_dir(work_dir)
        .ok()?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.file_stem()
                .and_then(|value| value.to_str())
                .map(|stem| stem == "youtube-video")
                .unwrap_or(false)
        })
        .filter(|path| {
            matches!(
                path.extension().and_then(|value| value.to_str()),
                Some("mp4" | "mkv" | "webm" | "mov")
            )
        })
        .collect::<Vec<_>>();
    candidates.sort();
    candidates.pop()
}

fn analyze_video(
    args: &YoutubeVideoRequest,
    video_path: &Path,
    aggregator: &mut BucketAggregator,
    data_buckets: &mut Vec<DataBucketReport>,
) -> Result<(VideoReport, Vec<ObservationReport>)> {
    let mut source = FfmpegVideoSource::open(video_path)?;
    let metadata = source.metadata().clone();
    let mut pipeline = build_video_pipeline(args)?;

    while let Some(frame) = source.next_video_frame()? {
        let frame_ref = frame.as_frame();
        let record = DataRecord::video_frame("video", frame.position.frame_index, &frame_ref);
        push_record(aggregator, record, data_buckets)?;
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
    let observations = result
        .observations
        .iter()
        .map(observation_report)
        .collect::<Vec<_>>();
    let scenes = result
        .detection
        .scenes
        .iter()
        .enumerate()
        .map(|(index, scene)| {
            let scene_observations = observations
                .iter()
                .filter(|observation| observation.scene_index == Some(index as u64))
                .cloned()
                .collect();
            SceneReport {
                index: index as u64,
                start_frame: scene.start.frame_index,
                end_frame: scene.end.frame_index,
                start_seconds: scene.start.timestamp.seconds(),
                end_seconds: scene.end.timestamp.seconds(),
                observations: scene_observations,
            }
        })
        .collect::<Vec<_>>();

    Ok((
        VideoReport {
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
            observations: Vec::new(),
        },
        observations,
    ))
}

fn build_video_pipeline(args: &YoutubeVideoRequest) -> Result<RealtimeVideoPipeline> {
    let mut builder = RealtimeVideoPipeline::builder()
        .scene_detector(ContentDetector::new(
            args.scene_threshold,
            args.min_scene_len,
        ))
        .start_in_scene(true);

    if let Some(command) = &args.object_command {
        let backend = ExternalCommandModel::new(
            command,
            downloaded_external_model("object-command", ModelTask::ObjectDetection),
        )
        .args(args.object_args.clone());
        builder = builder.video_analyzer(SampledVideoAnalyzer::new(
            ModelVideoAnalyzer::new("object_person_detector", backend),
            args.visual_sample_every,
        ));
    }

    if let Some(command) = &args.ocr_command {
        let backend = ExternalCommandModel::new(
            command,
            downloaded_external_model("ocr-command", ModelTask::Custom("ocr".to_string())),
        )
        .args(args.ocr_args.clone());
        builder = builder.video_analyzer(SampledVideoAnalyzer::new(
            ModelVideoAnalyzer::new("ocr", backend),
            args.visual_sample_every,
        ));
    }

    builder.build()
}

fn transcribe_video(
    args: &YoutubeVideoRequest,
    video_path: &Path,
) -> (TranscriptionReport, Option<PathBuf>) {
    if args.skip_transcription {
        return (
            TranscriptionReport {
                status: "skipped".to_string(),
                text: None,
                segments: Vec::new(),
                message: Some("disabled by --skip-transcription".to_string()),
            },
            None,
        );
    }

    let command = match &args.transcriber_command {
        Some(command) => command.clone(),
        None => {
            let default = PathBuf::from("whisper");
            if resolve_command(&default).is_none() {
                return (
                    TranscriptionReport {
                        status: "skipped".to_string(),
                        text: None,
                        segments: Vec::new(),
                        message: Some("install whisper or pass --transcriber-command".to_string()),
                    },
                    None,
                );
            }
            default
        }
    };

    match extract_audio_wav(video_path, &args.work_dir)
        .and_then(|audio_path| run_whisper_command(&command, &args.transcriber_args, &audio_path))
    {
        Ok((report, audio_path)) => (report, Some(audio_path)),
        Err(err) => (
            TranscriptionReport {
                status: "skipped".to_string(),
                text: None,
                segments: Vec::new(),
                message: Some(err.to_string()),
            },
            None,
        ),
    }
}

fn extract_audio_wav(video_path: &Path, work_dir: &Path) -> Result<PathBuf> {
    let path = work_dir.join("audio.wav");
    let status = Command::new("ffmpeg")
        .arg("-y")
        .arg("-v")
        .arg("error")
        .arg("-i")
        .arg(video_path)
        .arg("-vn")
        .arg("-ac")
        .arg("1")
        .arg("-ar")
        .arg("16000")
        .arg("-f")
        .arg("wav")
        .arg(&path)
        .status()?;
    if !status.success() {
        return Err(DetectError::Source(
            "ffmpeg failed to extract transcription audio".to_string(),
        ));
    }
    Ok(path)
}

fn run_whisper_command(
    command: &Path,
    extra_args: &[String],
    audio_path: &Path,
) -> Result<(TranscriptionReport, PathBuf)> {
    let output_dir = audio_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("transcript");
    fs::create_dir_all(&output_dir)?;

    let mut transcriber = WhisperCliTranscriber::new(command)
        .args(extra_args.iter().cloned())
        .output_dir(output_dir);
    let parsed = transcriber
        .transcribe(audio_path)
        .map_err(|err| DetectError::Source(err.to_string()))?;
    let segments = parsed
        .segments
        .into_iter()
        .map(|segment| TranscriptSegmentReport {
            index: segment.index,
            start_seconds: segment.start_seconds,
            end_seconds: segment.end_seconds,
            text: segment.text.trim().to_string(),
        })
        .collect::<Vec<_>>();
    Ok((
        TranscriptionReport {
            status: "completed".to_string(),
            text: parsed.text.map(|text| text.trim().to_string()),
            segments,
            message: parsed.source,
        },
        audio_path.to_path_buf(),
    ))
}

fn analyze_audio(
    video_path: &Path,
    aggregator: &mut BucketAggregator,
    data_buckets: &mut Vec<DataBucketReport>,
) -> Result<AudioReport> {
    let mut source = FfmpegAudioSource::open_path_with_options(
        video_path,
        FfmpegAudioSourceOptions::recorded().samples_per_chunk(16_384),
    )?;
    let mut pipeline = video_analysis_core::AudioPipeline::builder()
        .analyzer(AudioEnergyAnalyzer::default())
        .build()?;
    while let Some(frame) = source.next_audio_frame()? {
        let frame_ref = frame.as_frame()?;
        let record = DataRecord::audio_frame("audio", pipeline.frames_processed(), &frame_ref);
        push_record(aggregator, record, data_buckets)?;
        pipeline.process_frame(frame)?;
    }

    let result = pipeline.finish_analysis()?;
    Ok(AudioReport {
        status: "completed".to_string(),
        frames_processed: result.frames_processed,
        events: result.events.iter().map(event_report).collect(),
        message: None,
    })
}

fn analyze_transcript_text(
    args: &YoutubeVideoRequest,
    transcription: &TranscriptionReport,
    aggregator: &mut BucketAggregator,
    data_buckets: &mut Vec<DataBucketReport>,
) -> Result<TextReport> {
    if transcription.status != "completed" {
        return Ok(TextReport {
            status: "skipped".to_string(),
            segments_processed: 0,
            events: Vec::new(),
            message: Some("no transcript available".to_string()),
        });
    }

    let mut builder =
        video_analysis_core::TextPipeline::builder().analyzer(TranscriptHeuristicAnalyzer);
    if let Some(command) = &args.text_command {
        let backend = ExternalCommandModel::new(
            command,
            downloaded_external_model(
                "text-command",
                ModelTask::Custom("text_classification".to_string()),
            ),
        )
        .args(args.text_args.clone());
        builder = builder.analyzer(ModelTextAnalyzer::new("transcript_text_model", backend));
    }
    let mut pipeline = builder.build()?;

    for segment in &transcription.segments {
        let owned = segment_to_owned_text_segment(&TranscriptSegment {
            index: segment.index,
            start_seconds: segment.start_seconds,
            end_seconds: segment.end_seconds,
            text: segment.text.clone(),
            language: None,
            speaker: None,
            confidence: None,
            is_final: true,
        });
        let segment_ref = owned.as_segment();
        push_record(
            aggregator,
            DataRecord::text_segment("transcript", &segment_ref),
            data_buckets,
        )?;
        pipeline.process_segment(owned)?;
    }

    let result = pipeline.finish_analysis()?;
    Ok(TextReport {
        status: "completed".to_string(),
        segments_processed: result.segments_processed,
        events: result.events.iter().map(event_report).collect(),
        message: None,
    })
}

#[derive(Default)]
struct AudioEnergyAnalyzer {
    current_label: Option<String>,
}

impl AudioAnalyzer for AudioEnergyAnalyzer {
    fn name(&self) -> &str {
        "audio_energy"
    }

    fn process_frame(&mut self, frame: &AudioFrame<'_>) -> Result<Vec<AnalysisEvent>> {
        let rms = rms(frame.data);
        let label = if rms < 0.01 {
            "audio:silence"
        } else if rms > 0.2 {
            "audio:loud"
        } else {
            "audio:active"
        };
        if self.current_label.as_deref() == Some(label) {
            return Ok(Vec::new());
        }
        self.current_label = Some(label.to_string());
        Ok(vec![AnalysisEvent::new(self.name(), label)
            .at_timestamp(frame.timestamp)
            .score(rms)])
    }
}

fn rms(buffer: &AudioBuffer) -> f32 {
    let (sum, count) = match buffer {
        AudioBuffer::U8(values) => values.iter().fold((0.0, 0_usize), |(sum, count), value| {
            let sample = (*value as f32 - 128.0) / 128.0;
            (sum + sample * sample, count + 1)
        }),
        AudioBuffer::I16(values) => values.iter().fold((0.0, 0_usize), |(sum, count), value| {
            let sample = *value as f32 / i16::MAX as f32;
            (sum + sample * sample, count + 1)
        }),
        AudioBuffer::I32(values) => values.iter().fold((0.0, 0_usize), |(sum, count), value| {
            let sample = *value as f32 / i32::MAX as f32;
            (sum + sample * sample, count + 1)
        }),
        AudioBuffer::F32(values) => values.iter().fold((0.0, 0_usize), |(sum, count), value| {
            (sum + value * value, count + 1)
        }),
    };
    if count == 0 {
        0.0
    } else {
        (sum / count as f32).sqrt()
    }
}

struct SampledVideoAnalyzer<A> {
    inner: A,
    every: u64,
}

impl<A> SampledVideoAnalyzer<A> {
    fn new(inner: A, every: u64) -> Self {
        Self {
            inner,
            every: every.max(1),
        }
    }
}

impl<A: VideoAnalyzer> VideoAnalyzer for SampledVideoAnalyzer<A> {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn process_frame(&mut self, frame: &VideoFrame<'_>) -> Result<Vec<Observation>> {
        if frame.position.frame_index.is_multiple_of(self.every) {
            self.inner.process_frame(frame)
        } else {
            Ok(Vec::new())
        }
    }

    fn finish(
        &mut self,
        last_position: Option<video_analysis_core::FramePosition>,
    ) -> Result<Vec<Observation>> {
        self.inner.finish(last_position)
    }
}

fn downloaded_external_model(name: &str, task: ModelTask) -> DownloadedModel {
    DownloadedModel {
        spec: HuggingFaceModelSpec::new(name, task).name(name),
        files: BTreeMap::new(),
    }
}

fn observation_report(observation: &Observation) -> ObservationReport {
    ObservationReport {
        timestamp_seconds: observation.timestamp.map(|timestamp| timestamp.seconds()),
        frame_index: observation.frame.map(|position| position.frame_index),
        scene_index: observation.scene_index,
        analyzer: observation.analyzer.clone(),
        kind: observation_kind_name(&observation.kind),
        label: observation.label.clone(),
        text: observation.text.clone(),
        score: observation.score,
        region: observation.region.map(region_report),
        track_id: observation.track_id.clone(),
        attributes: observation.attributes.clone(),
    }
}

fn observation_kind_name(kind: &ObservationKind) -> String {
    match kind {
        ObservationKind::Text => "text".to_string(),
        ObservationKind::Face => "face".to_string(),
        ObservationKind::Object => "object".to_string(),
        ObservationKind::Scene => "scene".to_string(),
        ObservationKind::Custom(value) => value.clone(),
    }
}

fn region_report(region: BoundingBox) -> RegionReport {
    RegionReport {
        x: region.x,
        y: region.y,
        width: region.width,
        height: region.height,
    }
}

fn event_report(event: &AnalysisEvent) -> EventReport {
    EventReport {
        timestamp_seconds: event.timestamp.map(|timestamp| timestamp.seconds()),
        analyzer: event.analyzer.clone(),
        label: event.label.clone(),
        score: event.score,
    }
}

fn bucket_report(bucket: &DataBucket) -> DataBucketReport {
    DataBucketReport {
        bucket_index: bucket.bucket_index,
        records: bucket.records,
        estimated_bytes: bucket.estimated_bytes,
        streams: bucket
            .streams
            .iter()
            .map(|(stream_id, summary)| stream_bucket_report(stream_id, summary))
            .collect(),
    }
}

fn push_record(
    aggregator: &mut BucketAggregator,
    record: DataRecord<'_>,
    data_buckets: &mut Vec<DataBucketReport>,
) -> Result<()> {
    for bucket in aggregator.push(record)? {
        data_buckets.push(bucket_report(&bucket));
    }
    Ok(())
}

fn stream_bucket_report(stream_id: &str, summary: &StreamSummary) -> StreamBucketReport {
    StreamBucketReport {
        stream_id: stream_id.to_string(),
        records: summary.records,
        estimated_bytes: summary.estimated_bytes,
        payload_counts: summary
            .payload_counts
            .iter()
            .map(|(kind, count)| (data_stream_kind_name(*kind), *count))
            .collect(),
        video_frames: summary.video.frames,
        audio_frames: summary.audio.frames,
        text_segments: summary.text.segments,
    }
}

fn data_stream_kind_name(kind: DataStreamKind) -> String {
    match kind {
        DataStreamKind::Video => "video".to_string(),
        DataStreamKind::Audio => "audio".to_string(),
        DataStreamKind::Text => "text".to_string(),
        DataStreamKind::Number => "number".to_string(),
        DataStreamKind::Vector => "vector".to_string(),
        DataStreamKind::Custom => "custom".to_string(),
    }
}

fn video_report_with_observations(
    mut video_report: VideoReport,
    observations: Vec<ObservationReport>,
) -> VideoReport {
    video_report.observations = observations;
    video_report
}

fn write_json_report<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|err| DetectError::Source(format!("failed to encode report JSON: {err}")))?;
    fs::write(path, bytes)?;
    Ok(())
}

fn resolve_command(command: &Path) -> Option<PathBuf> {
    if command.components().count() > 1 {
        return command.exists().then(|| command.to_path_buf());
    }

    let name = command.to_str()?;
    env::var_os("PATH").and_then(|paths| {
        env::split_paths(&paths)
            .map(|dir| dir.join(name))
            .find(|path| path.is_file())
    })
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use video_analysis_core::TextAnalyzer;

    #[test]
    fn parses_transcript_whisper_json_segments() {
        let parsed = text_analysis_transcription::parse_whisper_json(
            br#"{"text":"hello world","segments":[{"start":0.0,"end":1.5,"text":" hello"}]}"#,
        )
        .unwrap();

        assert_eq!(parsed.text.as_deref(), Some("hello world"));
        assert_eq!(parsed.segments.len(), 1);
        assert_eq!(parsed.segments[0].start_seconds, Some(0.0));
    }

    #[test]
    fn transcript_heuristics_identify_common_events() {
        let mut analyzer = TranscriptHeuristicAnalyzer;
        let segment = video_analysis_core::OwnedTextSegment::new(0, "Visit https://example.com?");

        let events = analyzer.process_segment(&segment.as_segment()).unwrap();

        assert_eq!(events.len(), 2);
        assert_eq!(events[0].label, "speech:question");
        assert_eq!(events[1].label, "speech:url");
    }

    #[test]
    fn youtube_video_request_runs_local_video_without_clap() {
        if !(video_analysis_ffmpeg::is_ffmpeg_available()
            && video_analysis_ffmpeg::is_ffprobe_available())
        {
            eprintln!("skipping use-case integration test because ffmpeg/ffprobe is unavailable");
            return;
        }

        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("two-scenes.mp4");
        video_analysis_ffmpeg::write_two_scene_test_video(&input).unwrap();

        let report = run_youtube_video(YoutubeVideoRequest {
            input: Some(input),
            work_dir: dir.path().join("work"),
            output: Some(dir.path().join("analysis.json")),
            skip_transcription: true,
            max_frames: Some(8),
            ..YoutubeVideoRequest::default()
        })
        .unwrap();

        assert_eq!(report.use_case, YOUTUBE_VIDEO_USE_CASE);
        assert_eq!(report.transcription.status, "skipped");
        assert!(report.video.frames_processed > 0);
    }
}
