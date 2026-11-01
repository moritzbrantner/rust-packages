use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

#[derive(Debug)]
struct MatrixRow {
    operations: Vec<String>,
    wasm: bool,
    server: bool,
}

#[test]
fn every_library_matrix_row_exposes_mature_runtime_surface() {
    for (library, row) in parse_matrix().into_iter() {
        assert!(
            row.operations
                .iter()
                .any(|operation| operation == "describe"),
            "{library} must expose describe"
        );
        assert!(
            row.operations.len() >= 3,
            "{library} must expose describe plus at least two crate-specific operations"
        );

        let mut ids = BTreeSet::new();
        for operation in &row.operations {
            assert!(
                !operation.trim().is_empty(),
                "{library} must not expose empty operation IDs"
            );
            assert!(
                ids.insert(operation),
                "{library} has duplicate operation id {operation}"
            );
        }
    }
}

#[test]
fn deterministic_family_matrix_rows_are_transport_complete() {
    for (library, row) in parse_matrix()
        .into_iter()
        .filter(|(library, _)| deterministic_family(library))
    {
        assert!(
            row.operations
                .iter()
                .any(|operation| operation != "describe"),
            "{library} must expose deterministic crate-specific operations"
        );
        assert!(row.wasm, "{library} must expose a Rust WASM surface");
        assert!(row.server, "{library} must expose a server surface");
    }
}

fn parse_matrix() -> BTreeMap<String, MatrixRow> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let matrix = fs::read_to_string(root.join("docs/PACKAGE_SURFACE_MATRIX.md"))
        .expect("read package surface matrix");
    matrix
        .lines()
        .filter(|line| line.starts_with("| `"))
        .filter_map(|line| {
            let cells = line
                .trim_matches('|')
                .split('|')
                .map(str::trim)
                .collect::<Vec<_>>();
            if cells.len() < 9 {
                return None;
            }
            let library = strip_ticks(cells[0]).to_string();
            let operations = cells[6]
                .split(',')
                .map(str::trim)
                .map(strip_ticks)
                .map(str::to_string)
                .collect::<Vec<_>>();
            Some((
                library,
                MatrixRow {
                    operations,
                    wasm: cells[7] == "yes",
                    server: cells[8] == "yes",
                },
            ))
        })
        .collect()
}

fn strip_ticks(value: &str) -> &str {
    value.trim_matches('`')
}

fn deterministic_family(library: &str) -> bool {
    library.starts_with("comfyui-")
        || library.starts_with("image-")
        || library.starts_with("three-d-")
        || library.starts_with("video-")
}
