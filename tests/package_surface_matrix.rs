use std::collections::BTreeMap;
use std::fs;

#[test]
fn prioritized_crates_expose_more_than_describe() {
    let matrix = fs::read_to_string("docs/PACKAGE_SURFACE_MATRIX.md").unwrap();
    let operations_by_crate = parse_matrix(&matrix);
    let prioritized = [
        "moenarch-numbers-core",
        "moenarch-tensor-data",
        "moenarch-vector-analysis-core",
        "moenarch-vector-analysis-index",
        "moenarch-graph-analysis-core",
        "moenarch-math-geometry-2d",
        "moenarch-math-linear",
        "moenarch-math-signal-core",
        "moenarch-math-sparse-data",
        "moenarch-math-statistics",
    ];

    for crate_name in prioritized {
        let operations = operations_by_crate
            .get(crate_name)
            .unwrap_or_else(|| panic!("missing matrix row for {crate_name}"));
        assert!(
            operations.iter().any(|operation| operation != "describe"),
            "{crate_name} must expose more than describe"
        );
    }
}

#[test]
fn transcription_surface_ownership_is_explicit() {
    let matrix = fs::read_to_string("docs/PACKAGE_SURFACE_MATRIX.md").unwrap();
    let operations_by_crate = parse_matrix(&matrix);

    let recognition = operations_by_crate
        .get("moenarch-audio-analysis-recognition")
        .expect("recognition row");
    assert!(!recognition
        .iter()
        .any(|operation| operation.contains("transcribe")));
    assert!(!recognition
        .iter()
        .any(|operation| operation.contains("transcription")));

    let transcription = operations_by_crate
        .get("moenarch-audio-analysis-transcription")
        .expect("transcription row");
    assert!(transcription.contains(&"audio.transcription.transcribe".to_string()));
    assert!(transcription.contains(&"audio.transcription.importWhisperX".to_string()));
    assert!(transcription.contains(&"audio.transcription.providers".to_string()));

    let text = operations_by_crate
        .get("moenarch-text-transcripts")
        .expect("text-transcripts row");
    assert!(text.contains(&"transcripts.normalize".to_string()));
    assert!(text.contains(&"transcripts.importWhisperX".to_string()));
}

fn parse_matrix(markdown: &str) -> BTreeMap<String, Vec<String>> {
    markdown
        .lines()
        .filter(|line| line.starts_with("| `"))
        .filter_map(|line| {
            let cells = line
                .trim_matches('|')
                .split('|')
                .map(str::trim)
                .collect::<Vec<_>>();
            if cells.len() < 7 {
                return None;
            }
            let crate_name = strip_ticks(cells[0]).to_string();
            let operations = cells[6]
                .split(',')
                .map(str::trim)
                .map(strip_ticks)
                .map(str::to_string)
                .collect::<Vec<_>>();
            Some((crate_name, operations))
        })
        .collect()
}

fn strip_ticks(value: &str) -> &str {
    value.trim_matches('`')
}
