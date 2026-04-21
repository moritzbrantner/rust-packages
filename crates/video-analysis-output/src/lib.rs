use std::io::{self, Write};

use video_analysis_core::{DetectionResult, MetricsStore, Scene};

pub fn write_scene_list_csv(mut writer: impl Write, scenes: &[Scene]) -> io::Result<()> {
    writeln!(
        writer,
        "Scene Number,Start Frame,Start Timecode,Start Seconds,End Frame,End Timecode,End Seconds"
    )?;
    for (index, scene) in scenes.iter().enumerate() {
        let start_seconds = scene.start.timestamp.seconds();
        let end_seconds = scene.end.timestamp.seconds();
        writeln!(
            writer,
            "{},{},{},{:.6},{},{},{:.6}",
            index + 1,
            scene.start.frame_index,
            seconds_to_timecode(start_seconds),
            start_seconds,
            scene.end.frame_index,
            seconds_to_timecode(end_seconds),
            end_seconds
        )?;
    }
    Ok(())
}

pub fn write_stats_csv(mut writer: impl Write, metrics: &MetricsStore) -> io::Result<()> {
    let keys: Vec<_> = metrics.keys().collect();
    write!(writer, "Frame Number,Timecode")?;
    for key in &keys {
        write!(writer, ",{key}")?;
    }
    writeln!(writer)?;
    for (frame_index, row) in metrics.rows() {
        write!(writer, "{},{}", frame_index + 1, frame_index)?;
        for key in &keys {
            match row.get(*key) {
                Some(value) => write!(writer, ",{value}")?,
                None => write!(writer, ",")?,
            }
        }
        writeln!(writer)?;
    }
    Ok(())
}

pub fn write_scene_list_html(mut writer: impl Write, scenes: &[Scene]) -> io::Result<()> {
    writeln!(
        writer,
        "<!doctype html><meta charset=\"utf-8\"><title>Scene List</title><table>"
    )?;
    writeln!(
        writer,
        "<tr><th>Scene</th><th>Start Frame</th><th>Start</th><th>End Frame</th><th>End</th></tr>"
    )?;
    for (index, scene) in scenes.iter().enumerate() {
        writeln!(
            writer,
            "<tr><td>{}</td><td>{}</td><td>{:.3}</td><td>{}</td><td>{:.3}</td></tr>",
            index + 1,
            scene.start.frame_index,
            seconds_to_timecode(scene.start.timestamp.seconds()),
            scene.end.frame_index,
            seconds_to_timecode(scene.end.timestamp.seconds())
        )?;
    }
    writeln!(writer, "</table>")?;
    Ok(())
}

pub fn write_detection_outputs(
    result: &DetectionResult,
    scenes_writer: impl Write,
    stats_writer: Option<impl Write>,
) -> io::Result<()> {
    write_scene_list_csv(scenes_writer, &result.scenes)?;
    if let Some(stats_writer) = stats_writer {
        write_stats_csv(stats_writer, &result.metrics)?;
    }
    Ok(())
}

fn seconds_to_timecode(seconds: f64) -> String {
    let total_ms = (seconds * 1000.0).round() as u64;
    let total_seconds = total_ms / 1000;
    let ms = total_ms % 1000;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    format!("{hours:02}:{minutes:02}:{seconds:02}.{ms:03}")
}

#[cfg(test)]
mod tests {
    use num_rational::Rational64;
    use video_analysis_core::{FramePosition, MetricsSink, MetricsStore, Scene};

    use super::*;

    #[test]
    fn writes_stats_header_in_key_order() {
        let mut metrics = MetricsStore::default();
        metrics.set_metric(0, "z", 1.0);
        metrics.set_metric(0, "a", 2.0);
        metrics.set_metric(0, "combined.content.raw", 3.0);
        let mut out = Vec::new();
        write_stats_csv(&mut out, &metrics).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.starts_with("Frame Number,Timecode,a,combined.content.raw,z\n"));
    }

    #[test]
    fn writes_scene_csv_rows() {
        let pos = |frame| FramePosition::from_frame_index(frame, Rational64::new(10, 1));
        let scenes = vec![Scene {
            start: pos(0),
            end: pos(10),
        }];
        let mut out = Vec::new();
        write_scene_list_csv(&mut out, &scenes).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("Scene Number"));
        assert!(text.contains("1,0,00:00:00.000"));
    }
}
