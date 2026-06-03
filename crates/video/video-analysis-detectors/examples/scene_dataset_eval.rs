use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use video_analysis_core::SceneDetector;
use video_analysis_detectors::{
    AdaptiveDetector, ContentDetector, HashDetector, HistogramDetector, ThresholdDetector,
};
use video_analysis_ffmpeg::{FfmpegSourceOptions, FfmpegVideoSource};
use video_analysis_ingest::VideoFrameSource;

#[derive(Debug, Clone)]
struct Args {
    dataset: String,
    root: PathBuf,
    detector: String,
    output: Option<PathBuf>,
    video_ids: Vec<String>,
    limit: Option<usize>,
    max_runtime: Option<Duration>,
    progress: bool,
    resume: bool,
    resize_width: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EvalReport {
    dataset: String,
    detector: String,
    videos: Vec<VideoReport>,
    summary: EvalSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    mode: Option<EvalMode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EvalMode {
    video_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    resize_width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_runtime_seconds: Option<u64>,
    complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VideoReport {
    id: String,
    path: String,
    annotation_path: Option<String>,
    predicted_cuts: Vec<u64>,
    ground_truth_cuts: Vec<u64>,
    elapsed_ms: f64,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EvalSummary {
    recall: f64,
    precision: f64,
    f1: f64,
    avg_elapsed_ms: f64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let started = Instant::now();
    let args = parse_args()?;
    if args.resume && args.output.is_none() {
        return Err("--resume requires --output".into());
    }

    let annotations = annotation_map(&args.root)?;
    let videos = select_videos(video_files(&args.root)?, &args.video_ids, args.limit)?;
    if videos.is_empty() {
        return Err(format!("no video files selected under `{}`", args.root.display()).into());
    }
    let selected_ids = videos
        .iter()
        .map(|video| video_id(video))
        .collect::<Vec<_>>();

    let mut reports = resumed_reports(&args, &selected_ids)?;
    let completed = reports
        .iter()
        .map(|report| report.id.clone())
        .collect::<BTreeSet<_>>();

    for video in videos
        .into_iter()
        .filter(|video| !completed.contains(&video_id(video)))
    {
        if runtime_exceeded(started, args.max_runtime) {
            write_report(&args, &selected_ids, &reports, false)?;
            return Err(runtime_error(args.max_runtime));
        }

        let id = video_id(&video);
        if args.progress {
            eprintln!("scene_dataset_eval: start {id}");
        }
        let annotation_path = annotations.get(&id).cloned();
        let ground_truth_cuts = annotation_path
            .as_ref()
            .map(|path| load_annotation(path))
            .transpose()?
            .unwrap_or_default();
        let video_started = Instant::now();
        let predicted_cuts = match detect_video(
            &video,
            &args.detector,
            args.resize_width,
            started,
            args.max_runtime,
        ) {
            Ok(cuts) => cuts,
            Err(error) => {
                write_report(&args, &selected_ids, &reports, false)?;
                return Err(error);
            }
        };
        reports.push(VideoReport {
            id: id.clone(),
            path: video.to_string_lossy().into_owned(),
            annotation_path: annotation_path.map(|path| path.to_string_lossy().into_owned()),
            predicted_cuts,
            ground_truth_cuts,
            elapsed_ms: video_started.elapsed().as_secs_f64() * 1000.0,
        });
        reports.sort_by(|left, right| left.id.cmp(&right.id));

        let complete =
            reports.len() == selected_ids.len() && !runtime_exceeded(started, args.max_runtime);
        write_report(&args, &selected_ids, &reports, complete)?;
        if args.progress {
            eprintln!("scene_dataset_eval: finish {id}");
        }

        if runtime_exceeded(started, args.max_runtime) {
            write_report(&args, &selected_ids, &reports, false)?;
            return Err(runtime_error(args.max_runtime));
        }
    }

    let complete = reports.len() == selected_ids.len();
    write_report(&args, &selected_ids, &reports, complete)?;
    if !complete {
        return Err("scene dataset evaluation did not complete all selected videos".into());
    }
    Ok(())
}

fn parse_args() -> Result<Args, String> {
    let mut dataset = None;
    let mut root = None;
    let mut detector = None;
    let mut output = None;
    let mut video_ids = Vec::new();
    let mut limit = None;
    let mut max_runtime = None;
    let mut progress = false;
    let mut resume = false;
    let mut resize_width = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--dataset" => dataset = Some(next_value(&mut args, "--dataset")?),
            "--root" => root = Some(PathBuf::from(next_value(&mut args, "--root")?)),
            "--detector" => detector = Some(next_value(&mut args, "--detector")?),
            "--output" => output = Some(PathBuf::from(next_value(&mut args, "--output")?)),
            "--video-id" => push_unique(&mut video_ids, next_value(&mut args, "--video-id")?),
            "--limit" => {
                limit = Some(
                    next_value(&mut args, "--limit")?
                        .parse::<usize>()
                        .map_err(|error| format!("invalid --limit: {error}"))?,
                );
            }
            "--max-runtime-seconds" => {
                let seconds = next_value(&mut args, "--max-runtime-seconds")?
                    .parse::<u64>()
                    .map_err(|error| format!("invalid --max-runtime-seconds: {error}"))?;
                max_runtime = Some(Duration::from_secs(seconds));
            }
            "--resize-width" => {
                let width = next_value(&mut args, "--resize-width")?
                    .parse::<u32>()
                    .map_err(|error| format!("invalid --resize-width: {error}"))?;
                if width == 0 {
                    return Err("--resize-width must be greater than 0".to_string());
                }
                resize_width = Some(width);
            }
            "--progress" => progress = true,
            "--resume" => resume = true,
            "-h" | "--help" => return Err(usage()),
            _ => return Err(format!("unknown argument `{arg}`")),
        }
    }
    Ok(Args {
        dataset: dataset.ok_or_else(|| "missing --dataset".to_string())?,
        root: root.ok_or_else(|| "missing --root".to_string())?,
        detector: detector.unwrap_or_else(|| "content".to_string()),
        output,
        video_ids,
        limit,
        max_runtime,
        progress,
        resume,
        resize_width,
    })
}

fn next_value(args: &mut impl Iterator<Item = String>, name: &str) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("missing value for {name}"))
}

fn usage() -> String {
    "usage: scene_dataset_eval --dataset NAME --root PATH --detector content|adaptive|threshold|histogram|hash [--video-id ID ...] [--limit N] [--resize-width PIXELS] [--progress] [--resume] [--max-runtime-seconds SECONDS] [--output PATH]".to_string()
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

fn select_videos(
    videos: Vec<PathBuf>,
    requested_ids: &[String],
    limit: Option<usize>,
) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    if videos.is_empty() {
        return Ok(Vec::new());
    }
    let by_id = videos
        .into_iter()
        .map(|video| (video_id(&video), video))
        .collect::<BTreeMap<_, _>>();
    let mut selected = if requested_ids.is_empty() {
        by_id.values().cloned().collect::<Vec<_>>()
    } else {
        let mut selected = Vec::new();
        let mut missing = Vec::new();
        for id in requested_ids {
            if let Some(video) = by_id.get(id) {
                selected.push(video.clone());
            } else {
                missing.push(id.clone());
            }
        }
        if !missing.is_empty() {
            return Err(
                format!("requested video IDs were not found: {}", missing.join(", ")).into(),
            );
        }
        selected
    };
    selected.sort();
    if let Some(limit) = limit {
        selected.truncate(limit);
    }
    Ok(selected)
}

fn resumed_reports(
    args: &Args,
    selected_ids: &[String],
) -> Result<Vec<VideoReport>, Box<dyn std::error::Error>> {
    let Some(output) = args.output.as_ref().filter(|_| args.resume) else {
        return Ok(Vec::new());
    };
    if !output.exists() {
        return Ok(Vec::new());
    }
    let selected = selected_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let report = serde_json::from_str::<EvalReport>(&fs::read_to_string(output)?)?;
    let mut reports = report
        .videos
        .into_iter()
        .filter(|report| selected.contains(report.id.as_str()))
        .collect::<Vec<_>>();
    reports.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(reports)
}

fn detect_video(
    path: &Path,
    detector: &str,
    resize_width: Option<u32>,
    started: Instant,
    max_runtime: Option<Duration>,
) -> Result<Vec<u64>, Box<dyn std::error::Error>> {
    let options = resize_width
        .map(|width| FfmpegSourceOptions::recorded().resize_width(width))
        .unwrap_or_else(FfmpegSourceOptions::recorded);
    let mut source = FfmpegVideoSource::open_path_with_options(path, options)?;
    let mut detector = detector_by_name(detector)?;
    let mut cuts = Vec::new();
    let mut last_position = None;
    while let Some(frame) = source.next_video_frame()? {
        if runtime_exceeded(started, max_runtime) {
            return Err(runtime_error(max_runtime));
        }
        last_position = Some(frame.position);
        cuts.extend(
            detector
                .process_frame(&frame.as_frame(), None)?
                .into_iter()
                .map(|cut| cut.position.frame_index),
        );
    }
    if let Some(last_position) = last_position {
        cuts.extend(
            detector
                .finish(last_position, None)?
                .into_iter()
                .map(|cut| cut.position.frame_index),
        );
    }
    cuts.sort_unstable();
    cuts.dedup();
    Ok(cuts)
}

fn runtime_exceeded(started: Instant, max_runtime: Option<Duration>) -> bool {
    max_runtime
        .map(|max_runtime| started.elapsed() >= max_runtime)
        .unwrap_or(false)
}

fn runtime_error(max_runtime: Option<Duration>) -> Box<dyn std::error::Error> {
    match max_runtime {
        Some(duration) => format!(
            "scene dataset evaluation exceeded max runtime of {} seconds",
            duration.as_secs()
        )
        .into(),
        None => "scene dataset evaluation exceeded max runtime".into(),
    }
}

fn detector_by_name(name: &str) -> video_analysis_core::Result<Box<dyn SceneDetector>> {
    match name {
        "content" | "detect-content" => Ok(Box::new(ContentDetector::default())),
        "adaptive" | "detect-adaptive" => Ok(Box::new(AdaptiveDetector::default())),
        "threshold" | "detect-threshold" => Ok(Box::new(ThresholdDetector::default())),
        "histogram" | "detect-hist" | "detect-histogram" => {
            Ok(Box::new(HistogramDetector::default()))
        }
        "hash" | "detect-hash" => Ok(Box::new(HashDetector::default())),
        _ => Err(video_analysis_core::DetectError::InvalidArgument(format!(
            "unsupported detector `{name}`"
        ))),
    }
}

fn annotation_map(root: &Path) -> std::io::Result<BTreeMap<String, PathBuf>> {
    let mut map = BTreeMap::new();
    for path in collect_files(root)? {
        let Some(ext) = path.extension().and_then(|ext| ext.to_str()) else {
            continue;
        };
        if !matches!(ext.to_ascii_lowercase().as_str(), "txt" | "tsv" | "csv") {
            continue;
        }
        if let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) {
            map.entry(stem.to_string()).or_insert_with(|| path.clone());
            if let Some(prefix) = stem.split('-').next() {
                map.entry(prefix.to_string())
                    .or_insert_with(|| path.clone());
                if prefix.chars().all(|ch| ch.is_ascii_digit()) {
                    map.entry(format!("bbc_{prefix}"))
                        .or_insert_with(|| path.clone());
                }
            }
        }
    }
    Ok(map)
}

fn video_files(root: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut files = collect_files(root)?
        .into_iter()
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| {
                    matches!(
                        ext.to_ascii_lowercase().as_str(),
                        "mp4" | "webm" | "mkv" | "mov" | "avi"
                    )
                })
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    files.sort();
    Ok(files)
}

fn collect_files(root: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    collect_files_into(root, &mut files)?;
    Ok(files)
}

fn collect_files_into(path: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if path.is_file() {
        files.push(path.to_path_buf());
        return Ok(());
    }
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_files_into(&path, files)?;
        } else {
            files.push(path);
        }
    }
    Ok(())
}

fn load_annotation(path: &Path) -> Result<Vec<u64>, Box<dyn std::error::Error>> {
    let mut cuts = Vec::new();
    for line in fs::read_to_string(path)?.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields = line
            .split(['\t', ',', ' '])
            .filter(|field| !field.is_empty())
            .collect::<Vec<_>>();
        let value = fields
            .get(1)
            .or_else(|| fields.first())
            .ok_or_else(|| format!("invalid annotation row `{line}`"))?;
        if let Ok(frame) = value.parse::<u64>() {
            cuts.push(frame + 1);
        }
    }
    Ok(cuts)
}

fn write_report(
    args: &Args,
    selected_ids: &[String],
    reports: &[VideoReport],
    complete: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let report = EvalReport {
        dataset: args.dataset.clone(),
        detector: args.detector.clone(),
        summary: summarize(reports),
        videos: reports.to_vec(),
        mode: Some(EvalMode {
            video_ids: selected_ids.to_vec(),
            resize_width: args.resize_width,
            max_runtime_seconds: args.max_runtime.map(|duration| duration.as_secs()),
            complete,
        }),
    };
    let json = serde_json::to_string_pretty(&report)?;
    if let Some(output) = &args.output {
        atomic_write(output, &format!("{json}\n"))?;
    } else if complete {
        println!("{json}");
    }
    Ok(())
}

fn atomic_write(path: &Path, content: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file_name = path
        .file_name()
        .and_then(|file_name| file_name.to_str())
        .unwrap_or("scene-dataset-eval.json");
    let tmp = path.with_file_name(format!("{file_name}.tmp"));
    fs::write(&tmp, content)?;
    fs::rename(tmp, path)
}

fn video_id(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("video")
        .to_string()
}

fn summarize(reports: &[VideoReport]) -> EvalSummary {
    let mut correct = 0usize;
    let mut predicted = 0usize;
    let mut ground_truth = 0usize;
    let elapsed = reports.iter().map(|report| report.elapsed_ms).sum::<f64>();
    for report in reports {
        let gt = report
            .ground_truth_cuts
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        correct += report
            .predicted_cuts
            .iter()
            .filter(|frame| gt.contains(frame))
            .count();
        predicted += report.predicted_cuts.len();
        ground_truth += report.ground_truth_cuts.len();
    }
    let recall = if ground_truth == 0 {
        0.0
    } else {
        correct as f64 / ground_truth as f64
    };
    let precision = if predicted == 0 {
        0.0
    } else {
        correct as f64 / predicted as f64
    };
    let f1 = if recall + precision == 0.0 {
        0.0
    } else {
        2.0 * recall * precision / (recall + precision)
    };
    EvalSummary {
        recall,
        precision,
        f1,
        avg_elapsed_ms: if reports.is_empty() {
            0.0
        } else {
            elapsed / reports.len() as f64
        },
    }
}
