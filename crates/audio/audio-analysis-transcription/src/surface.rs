//! Library-owned runtime surface for `audio-analysis-transcription`.

use runtime_core::{
    structured_surface_response, MobileCapability, OperationId, PackageSurface,
    RuntimeCapabilities, SurfaceOperation, SurfaceRequest, SurfaceResponse,
};
use serde::Deserialize;

use crate::{
    candle_whisper_provider_plan, import_whisperx_json, transcribe, transcription_provider_plans,
    whisper_cpp_provider_plan, whisperx_provider_plan, AlignmentOptions, CandleWhisperOptions,
    DiarizationOptions, NativeDevicePreference, TranscriptionPipelineRequest,
    TranscriptionProviderSelection, TranscriptionSource, VadOptions, WhisperXCommandOptions,
    WhisperXDevice,
};

/// Returns the package surface exposed by every transport wrapper.
pub fn package_surface() -> PackageSurface {
    PackageSurface {
        library: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: RuntimeCapabilities {
            native: true,
            server: true,
            wasm: false,
            mobile: MobileCapability::Unsupported,
            requirements: vec![
                runtime_core::RuntimeRequirement {
                    name: "candle-whisper-model-bundle".to_string(),
                    description: Some(
                        "Required for native Candle Whisper ASR execution.".to_string(),
                    ),
                    required: false,
                },
                runtime_core::RuntimeRequirement {
                    name: "cuda".to_string(),
                    description: Some(
                        "Optional optimized device path when built with the cuda feature."
                            .to_string(),
                    ),
                    required: false,
                },
                runtime_core::RuntimeRequirement {
                    name: "whisperx".to_string(),
                    description: Some(
                        "Optional Python command for explicit external compatibility runs."
                            .to_string(),
                    ),
                    required: false,
                },
            ],
            max_recommended_input_bytes: None,
        },
        operations: vec![
            operation(
                "describe",
                "Describe package",
                "Rust-native audio and video transcription orchestration for video-analysis.",
                serde_json::json!({"includeOperations": true}),
                true,
            ),
            operation(
                "audio.transcription.transcribe",
                "Transcribe audio or video",
                "Runs real audio/video-to-text through the native transcription pipeline.",
                serde_json::json!({
                    "source": {"path": "{\"segments\":[{\"start\":0.0,\"end\":1.0,\"text\":\"Hello from offline compatibility output.\"}]}"},
                    "provider": {
                        "kind": "externalWhisperX",
                        "command": "/usr/bin/printf",
                        "model": "mock-whisperx-json",
                        "device": "cpu"
                    },
                    "vad": {"enabled": true},
                    "alignment": {"enabled": false},
                    "diarization": {"enabled": false},
                    "output": {"formats": ["json", "srt", "webvtt"]}
                }),
                false,
            ),
            operation(
                "audio.transcription.importWhisperX",
                "Import WhisperX JSON",
                "Parses existing WhisperX JSON output through text-transcripts without running models.",
                serde_json::json!({"content": "{\"segments\":[{\"start\":0.0,\"end\":1.0,\"text\":\"Hello.\"}]}"}),
                true,
            ),
            operation(
                "audio.transcription.providers",
                "Inspect transcription providers",
                "Lists native and compatibility transcription providers and runtime constraints.",
                serde_json::json!({"includeExternal": true}),
                true,
            ),
            operation(
                "audio.transcription.plan",
                "Plan transcription runtime",
                "Explains the native transcription runtime without running models.",
                serde_json::json!({"provider": {"kind": "candleWhisper", "modelId": "openai/whisper-large-v3-turbo"}}),
                true,
            ),
            operation(
                "audio.transcription.modelPlan",
                "Plan ASR model",
                "Explains Candle Whisper and compatibility model requirements.",
                serde_json::json!({"provider": "candle-whisper"}),
                true,
            ),
            operation(
                "audio.transcription.vadPlan",
                "Plan VAD",
                "Explains deterministic energy VAD chunking defaults.",
                serde_json::json!({"vad": {"enabled": true}}),
                true,
            ),
            operation(
                "audio.transcription.alignmentPlan",
                "Plan alignment",
                "Explains wav2vec2-style CTC forced-alignment requirements.",
                serde_json::json!({"alignment": {"enabled": true, "modelId": "facebook/wav2vec2-base-960h"}}),
                true,
            ),
        ],
    }
}

fn operation(
    id: &str,
    name: &str,
    description: &str,
    example_request: serde_json::Value,
    wasm_supported: bool,
) -> SurfaceOperation {
    SurfaceOperation {
        id: OperationId::new(id),
        name: name.to_string(),
        description: Some(description.to_string()),
        input_schema: serde_json::json!({"type": "object", "additionalProperties": true}),
        output_schema: serde_json::json!({"type": "object"}),
        example_request,
        wasm_supported,
        server_supported: true,
    }
}

/// Runs one library-owned operation.
pub fn run_surface_operation(request: SurfaceRequest) -> Result<SurfaceResponse, String> {
    let operation = request.operation.clone();
    let value = match request.operation.as_str() {
        "describe" => describe_value(request.input),
        "audio.transcription.transcribe" => transcribe_value(request.input)?,
        "audio.transcription.importWhisperX" => import_whisperx_value(parse_input(request.input)?)?,
        "audio.transcription.providers" => providers_value(request.input),
        "audio.transcription.plan" => plan_value(request.input),
        "audio.transcription.modelPlan" => model_plan_value(request.input),
        "audio.transcription.vadPlan" => vad_plan_value(request.input),
        "audio.transcription.alignmentPlan" => alignment_plan_value(request.input),
        operation => {
            return Err(runtime_core::SurfaceError::unsupported_operation(
                operation,
                env!("CARGO_PKG_NAME"),
            )
            .to_error_string())
        }
    };
    Ok(response(operation, value))
}

fn response(operation: OperationId, value: serde_json::Value) -> SurfaceResponse {
    let (title, message, summary) = match operation.as_str() {
        "describe" => (
            "Transcription package metadata",
            "Inspected native transcription operations and runtime support.",
            serde_json::json!({
                "operationCount": value.get("operationCount").cloned().unwrap_or(serde_json::Value::Null)
            }),
        ),
        "audio.transcription.transcribe" => (
            "Audio transcription result",
            "Ran audio/video-to-text through the selected provider.",
            serde_json::json!({
                "provider": value.get("provider").cloned().unwrap_or(serde_json::Value::Null),
                "modelId": value.get("modelId").cloned().unwrap_or(serde_json::Value::Null),
                "segmentCount": value.pointer("/transcript/segments").and_then(serde_json::Value::as_array).map_or(0, Vec::len)
            }),
        ),
        "audio.transcription.importWhisperX" => (
            "WhisperX import result",
            "Parsed existing WhisperX JSON through text-transcripts.",
            serde_json::json!({
                "segmentCount": value.get("segments").and_then(serde_json::Value::as_array).map_or(0, Vec::len),
                "hasText": value.get("text").and_then(serde_json::Value::as_str).map(|text| !text.is_empty()).unwrap_or(false)
            }),
        ),
        "audio.transcription.providers" => (
            "Transcription providers",
            "Inspected native and compatibility transcription provider support.",
            serde_json::json!({
                "providerCount": value.get("providers").and_then(serde_json::Value::as_array).map_or(0, Vec::len)
            }),
        ),
        "audio.transcription.plan"
        | "audio.transcription.modelPlan"
        | "audio.transcription.vadPlan"
        | "audio.transcription.alignmentPlan" => (
            "Transcription runtime plan",
            "Planned transcription setup without execution.",
            serde_json::json!({
                "defaultProvider": value.get("defaultProvider").cloned().unwrap_or(serde_json::Value::Null),
                "normalizationOwner": value.get("normalizationOwner").cloned().unwrap_or(serde_json::Value::Null)
            }),
        ),
        _ => (
            "Transcription operation result",
            "Completed the transcription package operation.",
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

fn transcribe_value(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let request: TranscriptionPipelineRequest =
        serde_json::from_value(input).map_err(|error| error.to_string())?;
    let response = transcribe(request).map_err(|error| error.to_string())?;
    Ok(serde_json::json!(response))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportContentRequest {
    content: String,
}

fn import_whisperx_value(request: ImportContentRequest) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!(import_whisperx_json(
        request.content.as_bytes()
    )
    .map_err(|error| error.to_string())?))
}

fn providers_value(input: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "input": input,
        "providers": transcription_provider_plans().into_iter().map(|plan| {
            serde_json::json!({
                "id": plan.provider_id,
                "externalRuntime": plan.external_runtime,
                "wasmSupported": plan.wasm_supported,
                "primary": plan.primary,
                "setup": plan.setup,
                "diagnostics": plan.diagnostics,
            })
        }).collect::<Vec<_>>()
    })
}

fn plan_value(input: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "operation": "audio.transcription.transcribe",
        "defaultProvider": "candle-whisper",
        "normalizationOwner": "moritzbrantner-text-transcripts",
        "vadProvider": "energy-vad",
        "alignmentProvider": "ctc-forced-aligner",
        "diarizationProvider": "audio-analysis-speakers-native-baseline",
        "providers": transcription_provider_plans(),
        "input": input
    })
}

fn model_plan_value(input: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "defaultProvider": "candle-whisper",
        "normalizationOwner": "moritzbrantner-text-transcripts",
        "asr": candle_whisper_provider_plan(),
        "compatibility": [whisper_cpp_provider_plan(), whisperx_provider_plan()],
        "models": [
            "openai/whisper-large-v3",
            "openai/whisper-large-v3-turbo",
            "facebook/wav2vec2-base-960h",
            "pyannote/speaker-diarization-3.1"
        ],
        "input": input
    })
}

fn vad_plan_value(input: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "defaultProvider": "energy-vad",
        "normalizationOwner": "moritzbrantner-text-transcripts",
        "options": VadOptions::default(),
        "input": input
    })
}

fn alignment_plan_value(input: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "defaultProvider": "candle-whisper",
        "provider": "ctc-forced-aligner",
        "modelId": "facebook/wav2vec2-base-960h",
        "normalizationOwner": "moritzbrantner-text-transcripts",
        "requiresFeature": "alignment",
        "input": input
    })
}

fn parse_input<T: for<'de> Deserialize<'de>>(input: serde_json::Value) -> Result<T, String> {
    runtime_core::parse_surface_input(None, input)
}

/// Builds a default native Candle Whisper request for local callers.
pub fn default_native_request(path: impl Into<std::path::PathBuf>) -> TranscriptionPipelineRequest {
    TranscriptionPipelineRequest {
        source: TranscriptionSource::Path { path: path.into() },
        provider: TranscriptionProviderSelection::CandleWhisper(CandleWhisperOptions::default()),
        vad: VadOptions::default(),
        alignment: AlignmentOptions::default(),
        diarization: DiarizationOptions::default(),
        output: Default::default(),
    }
}

/// Builds a default WhisperX compatibility request for local callers.
pub fn default_whisperx_request(
    path: impl Into<std::path::PathBuf>,
) -> TranscriptionPipelineRequest {
    TranscriptionPipelineRequest {
        source: TranscriptionSource::Path { path: path.into() },
        provider: TranscriptionProviderSelection::ExternalWhisperX(WhisperXCommandOptions {
            device: WhisperXDevice::Cpu,
            compute_type: Some("int8".to_string()),
            ..WhisperXCommandOptions::default()
        }),
        vad: VadOptions::default(),
        alignment: AlignmentOptions::default(),
        diarization: DiarizationOptions::default(),
        output: Default::default(),
    }
}

/// Builds a CUDA-preferring native request.
pub fn cuda_native_request(path: impl Into<std::path::PathBuf>) -> TranscriptionPipelineRequest {
    let mut request = default_native_request(path);
    request.provider = TranscriptionProviderSelection::CandleWhisper(CandleWhisperOptions {
        device: NativeDevicePreference::Cuda,
        ..CandleWhisperOptions::default()
    });
    request
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_surface_lists_transcription_operations() {
        let surface = package_surface();
        let ids = surface
            .operations
            .iter()
            .map(|operation| operation.id.as_str())
            .collect::<Vec<_>>();
        assert!(ids.contains(&"audio.transcription.transcribe"));
        assert!(ids.contains(&"audio.transcription.importWhisperX"));
        assert!(ids.contains(&"audio.transcription.providers"));
        assert!(ids.contains(&"audio.transcription.plan"));
        assert!(ids.contains(&"audio.transcription.modelPlan"));
        assert!(ids.contains(&"audio.transcription.vadPlan"));
        assert!(ids.contains(&"audio.transcription.alignmentPlan"));
    }

    #[test]
    fn import_whisperx_operation_returns_transcript() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("audio.transcription.importWhisperX"),
            input: serde_json::json!({
                "content": "{\"segments\":[{\"start\":0.0,\"end\":1.0,\"text\":\"hello\"}]}"
            }),
        })
        .expect("import");
        assert_eq!(
            response.value["operation"],
            "audio.transcription.importWhisperX"
        );
        assert_eq!(
            response.value["result"]["segments"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn plan_reports_candle_as_primary_native_runtime() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("audio.transcription.plan"),
            input: serde_json::json!({}),
        })
        .expect("plan");
        assert_eq!(
            response.value["result"]["defaultProvider"],
            "candle-whisper"
        );
        assert_eq!(
            response.value["result"]["normalizationOwner"],
            "moritzbrantner-text-transcripts"
        );
    }
}
