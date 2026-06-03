use num_rational::Rational64;
use video_analysis_core::{Cut, DetectionResult, FramePosition, MetricsSink, MetricsStore, Scene};
use video_analysis_output::{write_detection_result_json, write_scene_list_csv, write_stats_csv};

fn pos(frame: u64) -> FramePosition {
    FramePosition::from_frame_index(frame, Rational64::new(10, 1))
}

fn result() -> DetectionResult {
    let mut metrics = MetricsStore::default();
    metrics.set_metric(0, "content_val", 0.0);
    metrics.set_metric(10, "content_val", 42.0);
    metrics.set_metric(10, "delta_lum", 42.0);
    DetectionResult {
        scenes: vec![
            Scene {
                start: pos(0),
                end: pos(10),
            },
            Scene {
                start: pos(10),
                end: pos(20),
            },
        ],
        cuts: vec![Cut {
            position: pos(10),
            detector: "content",
            score: Some(42.0),
        }],
        metrics,
        frames_processed: 20,
    }
}

#[test]
fn scene_csv_uses_pyscenedetect_compatible_columns() {
    let mut out = Vec::new();
    write_scene_list_csv(&mut out, &result().scenes).unwrap();
    let text = String::from_utf8(out).unwrap();

    assert!(text.starts_with(
        "Scene Number,Start Frame,Start Timecode,Start Seconds,End Frame,End Timecode,End Seconds\n"
    ));
    assert!(text.contains("1,0,00:00:00.000,0.000000,10,00:00:01.000,1.000000"));
    assert!(text.contains("2,10,00:00:01.000,1.000000,20,00:00:02.000,2.000000"));
}

#[test]
fn stats_csv_contains_frame_numbers_indices_and_sorted_metric_keys() {
    let mut out = Vec::new();
    write_stats_csv(&mut out, &result().metrics).unwrap();
    let text = String::from_utf8(out).unwrap();

    assert!(text.starts_with("Frame Number,Frame Index,content_val,delta_lum\n"));
    assert!(text.contains("1,0,0"));
    assert!(text.contains("11,10,42,42"));
}

#[test]
fn detection_json_contains_scenes_cuts_metrics_and_frame_count() {
    let mut out = Vec::new();
    write_detection_result_json(&mut out, &result()).unwrap();
    let json: serde_json::Value = serde_json::from_slice(&out).unwrap();

    assert_eq!(json["frames_processed"], 20);
    assert_eq!(json["scenes"].as_array().unwrap().len(), 2);
    assert_eq!(json["cuts"][0]["detector"], "content");
    assert_eq!(json["metrics"]["keys"][0], "content_val");
    assert_eq!(json["metrics"]["rows"]["10"]["delta_lum"], 42.0);
}
