use std::collections::BTreeMap;
use std::fs;

#[test]
fn prioritized_crates_expose_more_than_describe() {
    let matrix = fs::read_to_string("docs/PACKAGE_SURFACE_MATRIX.md").unwrap();
    let operations_by_crate = parse_matrix(&matrix);
    let prioritized = [
        "moritzbrantner-numbers-core",
        "moritzbrantner-tensor-data",
        "moritzbrantner-vector-analysis-core",
        "moritzbrantner-vector-analysis-index",
        "moritzbrantner-graph-analysis-core",
        "moritzbrantner-geo-core",
        "moritzbrantner-geo-io-geojson",
        "moritzbrantner-geo-io-osm",
        "moritzbrantner-geo-clustering",
        "moritzbrantner-geo-viz",
        "moritzbrantner-math-geometry-2d",
        "moritzbrantner-math-linear",
        "moritzbrantner-math-signal-core",
        "moritzbrantner-math-sparse-data",
        "moritzbrantner-math-statistics",
        "moritzbrantner-finance-statistics",
        "moritzbrantner-maps-kernels-core",
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
