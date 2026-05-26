//! Library-owned runtime surface for `audio-analysis-separation`.

use std::path::PathBuf;

use video_analysis_core::runtime::{
    OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation, SurfaceRequest,
    SurfaceResponse,
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
                "Separation models",
                "Lists known Demucs model layouts without executing Demucs.",
                serde_json::json!({}),
            ),
            operation(
                "audio.separation.plan",
                "Separation plan",
                "Builds a Demucs command plan without running an external process.",
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
    SurfaceResponse {
        operation,
        value,
        diagnostics: Vec::new(),
        artifacts: Vec::new(),
    }
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
    let command = separator
        .build_command(PathBuf::from(&input_path))
        .map_err(|error| error.to_string())?;
    Ok(serde_json::json!({
        "input": input_path,
        "program": command.program,
        "args": command.args.iter().map(|arg| arg.to_string_lossy().to_string()).collect::<Vec<_>>(),
        "executed": false,
        "expectedStems": separator.expected_stems().iter().map(Stem::as_str).collect::<Vec<_>>()
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
        assert_eq!(response.value["executed"], false);
        assert!(response.value["args"]
            .as_array()
            .unwrap()
            .iter()
            .any(|arg| arg == "song.wav"));
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
