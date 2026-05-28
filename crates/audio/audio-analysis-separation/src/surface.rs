//! Library-owned runtime surface for `audio-analysis-separation`.

use std::path::PathBuf;

use video_analysis_core::runtime::{
    structured_surface_response, OperationId, PackageSurface, RuntimeCapabilities,
    SurfaceOperation, SurfaceRequest, SurfaceResponse,
};

use crate::{
    DemucsModel, HtdemucsOptions, HtdemucsSeparator, SeparationOutputFormat, Stem, StemLayout,
};

/// Returns the package surface exposed by every transport wrapper.
pub fn package_surface() -> PackageSurface {
    PackageSurface {
        library: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: RuntimeCapabilities::pure_rust(),
        operations: vec![
            operation(
                "describe",
                "Describe package",
                "Deterministic source-separation command planning for video-analysis.",
                serde_json::json!({"includeOperations": true}),
            ),
            operation(
                "audio.separation.models",
                "Inspect separation models",
                "Inspects known Demucs model layouts without executing Demucs.",
                serde_json::json!({}),
            ),
            operation(
                "audio.separation.plan",
                "Preview separation command",
                "Previews a Demucs command without running an external process.",
                serde_json::json!({"input": "song.wav", "outputDir": "stems", "model": "htdemucs", "format": "wav"}),
            ),
            operation(
                "audio.separation.expectedStems",
                "Expected stems",
                "Returns expected separated stem filenames for a model/layout.",
                serde_json::json!({"model": "htdemucs", "format": "wav"}),
            ),
        ],
    }
}

fn operation(
    id: &str,
    name: &str,
    description: &str,
    example_request: serde_json::Value,
) -> SurfaceOperation {
    SurfaceOperation {
        id: OperationId::new(id),
        name: name.to_string(),
        description: Some(description.to_string()),
        input_schema: serde_json::json!({"type": "object", "additionalProperties": true}),
        output_schema: serde_json::json!({"type": "object"}),
        example_request,
        wasm_supported: true,
        server_supported: true,
    }
}

/// Runs one library-owned operation.
pub fn run_surface_operation(request: SurfaceRequest) -> Result<SurfaceResponse, String> {
    let operation = request.operation.clone();
    let value = match request.operation.as_str() {
        "describe" => describe_value(request.input),
        "audio.separation.models" => models_value(),
        "audio.separation.plan" => plan_value(request.input)?,
        "audio.separation.expectedStems" => expected_stems_value(request.input)?,
        operation => {
            return Err(format!(
                "unsupported operation `{operation}` for {}",
                env!("CARGO_PKG_NAME")
            ));
        }
    };
    Ok(response(operation, value))
}

fn response(operation: OperationId, value: serde_json::Value) -> SurfaceResponse {
    let (title, message, summary) = match operation.as_str() {
        "describe" => (
            "Separation package metadata",
            "Inspected the deterministic source-separation planning operations exposed by this package.",
            serde_json::json!({
                "operationCount": value.get("operationCount").cloned().unwrap_or(serde_json::Value::Null)
            }),
        ),
        "audio.separation.models" => (
            "Separation model inventory",
            "Inspected known Demucs model layouts without executing Demucs.",
            serde_json::json!({
                "backend": value.get("backend").cloned().unwrap_or(serde_json::Value::Null),
                "modelCount": value.get("models").and_then(serde_json::Value::as_array).map_or(0, Vec::len)
            }),
        ),
        "audio.separation.plan" => (
            "Separation command preview",
            "Built a Demucs command preview only; this operation does not run Demucs, read audio, or write stems.",
            serde_json::json!({
                "program": value.get("program").cloned().unwrap_or(serde_json::Value::Null),
                "model": value.get("model").cloned().unwrap_or(serde_json::Value::Null),
                "outputDir": value.get("outputDir").cloned().unwrap_or(serde_json::Value::Null),
                "executed": value.get("executed").cloned().unwrap_or(serde_json::Value::Null),
                "expectedStemCount": value.get("expectedStemPaths").and_then(serde_json::Value::as_array).map_or(0, Vec::len)
            }),
        ),
        "audio.separation.expectedStems" => (
            "Expected separation stems",
            "Computed deterministic separated stem filenames for the requested model and output format.",
            serde_json::json!({
                "model": value.get("model").cloned().unwrap_or(serde_json::Value::Null),
                "layout": value.get("layout").cloned().unwrap_or(serde_json::Value::Null),
                "stemCount": value.get("stems").and_then(serde_json::Value::as_array).map_or(0, Vec::len)
            }),
        ),
        _ => (
            "Separation operation result",
            "Completed the separation package surface operation.",
            serde_json::json!({}),
        ),
    };
    structured_surface_response(operation, title, message, summary, value)
}

fn describe_value(input: serde_json::Value) -> serde_json::Value {
    let surface = package_surface();
    serde_json::json!({
        "library": surface.library,
        "version": surface.version,
        "operationCount": surface.operations.len(),
        "operations": surface.operations.iter().map(|operation| operation.id.as_str()).collect::<Vec<_>>(),
        "input": input
    })
}

fn models_value() -> serde_json::Value {
    let models = [
        DemucsModel::Htdemucs,
        DemucsModel::HtdemucsFt,
        DemucsModel::Htdemucs6s,
        DemucsModel::MdX,
        DemucsModel::MdXExtra,
        DemucsModel::MdXQ,
    ];
    serde_json::json!({
        "backend": "demucs",
        "defaultModel": DemucsModel::Htdemucs.as_str(),
        "models": models.iter().map(|model| serde_json::json!({
            "id": model.as_str(),
            "layout": layout_name(&model.default_layout()),
            "stems": model.default_layout().stems().iter().map(Stem::as_str).collect::<Vec<_>>()
        })).collect::<Vec<_>>()
    })
}

fn plan_value(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let input_path = string_field(&input, "input", "input.wav")?;
    let separator = separator_from_input(&input)?;
    let output_dir = separator.options.output_dir.clone();
    let output_dir_string = output_dir.to_string_lossy().to_string();
    let format = output_format(&input);
    let command = separator
        .build_command(PathBuf::from(&input_path))
        .map_err(|error| error.to_string())?;
    let expected_stem_paths = separator
        .expected_stems()
        .iter()
        .map(|stem| format!("{}/{}", output_dir_string, stem.file_name(&format)))
        .collect::<Vec<_>>();
    Ok(serde_json::json!({
        "input": input_path,
        "outputDir": output_dir_string,
        "model": separator.options.model.as_str(),
        "program": command.program,
        "args": command.args.iter().map(|arg| arg.to_string_lossy().to_string()).collect::<Vec<_>>(),
        "executed": false,
        "doesNot": ["read audio", "write stems", "run Demucs"],
        "setup": {
            "pythonPackage": "demucs",
            "exampleCommand": "python -m pip install demucs",
            "expectedExecutable": command.program
        },
        "missingToolBehavior": "A native execution path should report a missing Demucs executable before spawning a command; this surface operation only previews the command.",
        "expectedStems": separator.expected_stems().iter().map(Stem::as_str).collect::<Vec<_>>(),
        "expectedStemPaths": expected_stem_paths
    }))
}

fn expected_stems_value(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let separator = separator_from_input(&input)?;
    let format = output_format(&input);
    Ok(serde_json::json!({
        "model": separator.options.model.as_str(),
        "layout": layout_name(&separator.expected_layout()),
        "format": format.extension(),
        "stems": separator.expected_stems().iter().map(|stem| serde_json::json!({
            "stem": stem.as_str(),
            "fileName": stem.file_name(&format)
        })).collect::<Vec<_>>()
    }))
}

fn separator_from_input(input: &serde_json::Value) -> Result<HtdemucsSeparator, String> {
    let mut options = HtdemucsOptions::new(string_field(input, "outputDir", "stems")?)
        .command(string_field(input, "command", "demucs")?)
        .model(string_field(input, "model", "htdemucs")?)
        .output_format(output_format(input));
    if let Some(primary) = input.get("twoStems").and_then(serde_json::Value::as_str) {
        options = options.two_stems(primary.parse::<Stem>().map_err(|error| error.to_string())?);
    }
    if input
        .get("layout")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|layout| layout == "sixStem" || layout == "six-stem")
    {
        options = options.layout(StemLayout::SixStem);
    }
    HtdemucsSeparator::new(options).map_err(|error| error.to_string())
}

fn output_format(input: &serde_json::Value) -> SeparationOutputFormat {
    match input.get("format").and_then(serde_json::Value::as_str) {
        Some("mp3") => SeparationOutputFormat::Mp3,
        Some("flac") => SeparationOutputFormat::Flac,
        Some(other) if other != "wav" => SeparationOutputFormat::Custom(other.to_string()),
        _ => SeparationOutputFormat::Wav,
    }
}

fn string_field(
    input: &serde_json::Value,
    field: &str,
    default_value: &str,
) -> Result<String, String> {
    let value = input
        .get(field)
        .and_then(serde_json::Value::as_str)
        .unwrap_or(default_value)
        .to_string();
    if value.trim().is_empty() {
        return Err(format!("{field} must not be empty"));
    }
    Ok(value)
}

fn layout_name(layout: &StemLayout) -> &'static str {
    match layout {
        StemLayout::FourStem => "fourStem",
        StemLayout::SixStem => "sixStem",
        StemLayout::TwoStem { .. } => "twoStem",
        StemLayout::Custom(_) => "custom",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_surface_lists_separation_operations() {
        let surface = package_surface();
        let ids = surface
            .operations
            .iter()
            .map(|operation| operation.id.as_str())
            .collect::<Vec<_>>();
        assert!(ids.contains(&"audio.separation.models"));
        assert!(ids.contains(&"audio.separation.plan"));
    }

    #[test]
    fn plan_operation_does_not_execute() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("audio.separation.plan"),
            input: serde_json::json!({"input": "song.wav", "outputDir": "stems"}),
        })
        .expect("plan");
        assert_eq!(response.value["operation"], "audio.separation.plan");
        assert!(response.value["title"].is_string());
        assert!(response.value["summary"].is_object());
        assert!(response.value["result"].is_object());
        assert_eq!(response.value["executed"], false);
        assert_eq!(response.value["model"], "htdemucs");
        assert!(
            response.value["expectedStemPaths"]
                .as_array()
                .unwrap()
                .len()
                >= 4
        );
        assert!(response.value["missingToolBehavior"]
            .as_str()
            .unwrap()
            .contains("missing Demucs"));
        assert!(response.value["args"]
            .as_array()
            .unwrap()
            .iter()
            .any(|arg| arg == "song.wav"));
    }

    #[test]
    fn example_requests_run_with_structured_outputs() {
        for operation in package_surface().operations {
            let response = run_surface_operation(SurfaceRequest {
                operation: operation.id.clone(),
                input: operation.example_request.clone(),
            })
            .unwrap_or_else(|error| panic!("{} example failed: {error}", operation.id.as_str()));
            assert_eq!(response.value["operation"], operation.id.as_str());
            assert!(response.value["title"].is_string());
            assert!(response.value["summary"].is_object());
            assert!(response.value["result"].is_object());
        }
    }

    #[test]
    fn invalid_input_returns_error() {
        let error = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("audio.separation.plan"),
            input: serde_json::json!({"input": ""}),
        })
        .unwrap_err();
        assert!(error.contains("input"));
    }
}
