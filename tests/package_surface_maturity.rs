use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use video_analysis_core::runtime::{PackageSurface, SurfaceOperation};

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

#[test]
fn migrated_tranche_operation_metadata_is_complete() {
    for surface in [
        audio_analysis_core::surface::package_surface(),
        audio_analysis_processing::surface::package_surface(),
        image_analysis_processing::surface::package_surface(),
        text_core::surface::package_surface(),
        video_analysis_detectors::surface::package_surface(),
        video_analysis_editing::surface::package_surface(),
        video_analysis_output::surface::package_surface(),
        video_analysis_recognition::surface::package_surface(),
        video_analysis_split::surface::package_surface(),
        video_analysis_synthesis::surface::package_surface(),
        video_analysis_tracking::surface::package_surface(),
    ] {
        assert_surface_metadata(surface);
    }

    for (surface, runner) in [
        (
            video_analysis_detectors::surface::package_surface(),
            video_analysis_detectors::surface::run_surface_operation
                as fn(
                    video_analysis_core::runtime::SurfaceRequest,
                )
                    -> Result<video_analysis_core::runtime::SurfaceResponse, String>,
        ),
        (
            video_analysis_output::surface::package_surface(),
            video_analysis_output::surface::run_surface_operation,
        ),
        (
            video_analysis_recognition::surface::package_surface(),
            video_analysis_recognition::surface::run_surface_operation,
        ),
        (
            video_analysis_split::surface::package_surface(),
            video_analysis_split::surface::run_surface_operation,
        ),
        (
            video_analysis_synthesis::surface::package_surface(),
            video_analysis_synthesis::surface::run_surface_operation,
        ),
        (
            video_analysis_tracking::surface::package_surface(),
            video_analysis_tracking::surface::run_surface_operation,
        ),
    ] {
        assert_surface_operations_are_not_scaffold(surface, runner);
    }
}

fn assert_surface_metadata(surface: PackageSurface) {
    for operation in &surface.operations {
        assert_operation_metadata(&surface.library, operation);
    }
}

fn assert_operation_metadata(library: &str, operation: &SurfaceOperation) {
    let id = operation.id.as_str();
    assert!(!id.trim().is_empty(), "{library} operation id is empty");
    assert!(
        !operation.name.trim().is_empty(),
        "{library}:{id} operation name is empty"
    );
    assert!(
        operation
            .description
            .as_deref()
            .is_some_and(|description| !description.trim().is_empty()),
        "{library}:{id} operation description is empty"
    );
    assert!(
        operation.example_request.is_object(),
        "{library}:{id} example_request must be an object"
    );
    assert!(
        operation.input_schema.is_object(),
        "{library}:{id} input_schema must be an object"
    );
    assert!(
        operation.output_schema.is_object(),
        "{library}:{id} output_schema must be an object"
    );
}

fn assert_surface_operations_are_not_scaffold(
    surface: PackageSurface,
    runner: fn(
        video_analysis_core::runtime::SurfaceRequest,
    ) -> Result<video_analysis_core::runtime::SurfaceResponse, String>,
) {
    for operation in surface
        .operations
        .iter()
        .filter(|operation| operation.id.as_str() != "describe")
    {
        let response = runner(video_analysis_core::runtime::SurfaceRequest {
            operation: operation.id.clone(),
            input: operation.example_request.clone(),
        })
        .unwrap_or_else(|error| {
            panic!(
                "{}:{} example request failed: {error}",
                surface.library,
                operation.id.as_str()
            )
        });
        assert!(
            !response
                .value
                .to_string()
                .contains("A deterministic summary or execution plan owned by the Rust library"),
            "{}:{} still returns scaffold payload",
            surface.library,
            operation.id.as_str()
        );
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
    library.starts_with("text-")
        || library.starts_with("image-")
        || library.starts_with("audio-")
        || library.starts_with("math-")
        || library.ends_with("-data")
        || library.starts_with("data-")
        || library.starts_with("dense-")
        || library.starts_with("vector-")
        || library.starts_with("comfyui-")
        || library.starts_with("three-d-")
        || library.starts_with("video-")
        || matches!(
            library,
            "jobs-core" | "model-runtime" | "numbers-core" | "tensor-data"
        )
}
